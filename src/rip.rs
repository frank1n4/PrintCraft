//! The RIP step: rasterise each job with VectorCraft at the print resolution (within the raster
//! limit), apply scale/rotation/mirror, lay copies and nests out on the roll, split wide jobs
//! into tiles and write PNG files that carry their real dpi.
use crate::model::*;
use image::{imageops, RgbImage, RgbaImage};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct RipResult {
    pub files: Vec<PathBuf>,
    pub dpi: f32,
}

pub fn effective_dpi(width_mm: f32, length_mm: f32, wanted: f32) -> f32 {
    let w_in = (width_mm / MM_PER_INCH).max(0.01);
    let h_in = (length_mm / MM_PER_INCH).max(0.01);
    let limit = (RIP_RASTER_LIMIT_MP * 1.0e6 / (w_in * h_in)).sqrt();
    wanted.min(limit).max(10.0)
}

fn mm_px(mm: f32, dpi: f32) -> u32 {
    ((mm / MM_PER_INCH) * dpi).round().max(1.0) as u32
}

fn load_job_raster(job: &Job, dpi: f32, work: &Path) -> Result<RgbaImage, String> {
    let tmp = work.join(format!("rip_src_{}.png", job.id));
    let scale = dpi / POINTS_PER_INCH * job.settings.scale_pct / 100.0;
    crate::vc::rasterize(&job.path, &tmp, scale)?;
    let mut img = image::open(&tmp).map_err(|e| e.to_string())?.to_rgba8();
    let _ = std::fs::remove_file(&tmp);
    img = match ROTATIONS[job.settings.rotate] {
        90 => imageops::rotate90(&img),
        180 => imageops::rotate180(&img),
        270 => imageops::rotate270(&img),
        _ => img,
    };
    if job.settings.mirror {
        imageops::flip_horizontal_in_place(&mut img);
    }
    Ok(img)
}

fn paste(canvas: &mut RgbImage, src: &RgbaImage, x0: u32, y0: u32) {
    for (x, y, p) in src.enumerate_pixels() {
        let (cx, cy) = (x0 + x, y0 + y);
        if cx >= canvas.width() || cy >= canvas.height() {
            continue;
        }
        let a = p[3] as u32;
        if a == 0 {
            continue;
        }
        let d = canvas.get_pixel_mut(cx, cy);
        for c in 0..3 {
            d[c] = ((p[c] as u32 * a + d[c] as u32 * (255 - a)) / 255) as u8;
        }
    }
}

pub fn save_png_dpi(img: &RgbImage, path: &Path, dpi: f32) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width(), img.height());
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let ppm = (dpi / 0.0254).round() as u32;
    enc.set_pixel_dims(Some(png::PixelDimensions { xppu: ppm, yppu: ppm, unit: png::Unit::Meter }));
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(img.as_raw()).map_err(|e| e.to_string())
}

pub fn rip(jobs: &[Job], setup: &Setup, out_dir: &Path, base_name: &str) -> Result<RipResult, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let refs: Vec<&Job> = jobs.iter().collect();
    let lay = layout(&refs, setup.printable_mm());
    let wanted = RESOLUTIONS[jobs[0].settings.resolution].1 as f32;
    let tile_w = if lay.tiles > 1 { setup.printable_mm() } else { lay.width_mm };
    let one_len = if lay.tiles > 1 { lay.items[0].h } else { lay.length_mm };
    let dpi = effective_dpi(tile_w, one_len, wanted);

    let mut rasters: HashMap<u64, RgbaImage> = HashMap::new();
    for j in jobs {
        rasters.insert(j.id, load_job_raster(j, dpi, out_dir)?);
    }
    let safe: String = base_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let mut files = Vec::new();

    if lay.tiles > 1 {
        let src = &rasters[&lay.items[0].job];
        let step = mm_px(setup.printable_mm() - TILE_OVERLAP_MM, dpi);
        let tw = mm_px(setup.printable_mm(), dpi);
        for t in 0..lay.tiles {
            let x = (t * step).min(src.width().saturating_sub(1));
            let w = tw.min(src.width() - x);
            let part = imageops::crop_imm(src, x, 0, w, src.height()).to_image();
            let mut canvas = RgbImage::from_pixel(w, src.height(), image::Rgb([255, 255, 255]));
            paste(&mut canvas, &part, 0, 0);
            let p = out_dir.join(format!("{safe}_painel{}.png", t + 1));
            save_png_dpi(&canvas, &p, dpi)?;
            files.push(p);
        }
    } else {
        let mut canvas = RgbImage::from_pixel(
            mm_px(lay.width_mm, dpi),
            mm_px(lay.length_mm, dpi),
            image::Rgb([255, 255, 255]),
        );
        for it in &lay.items {
            if let Some(src) = rasters.get(&it.job) {
                paste(&mut canvas, src, mm_px(it.x, dpi).saturating_sub(1), mm_px(it.y, dpi).saturating_sub(1));
            }
        }
        let p = out_dir.join(format!("{safe}.png"));
        save_png_dpi(&canvas, &p, dpi)?;
        files.push(p);
    }
    Ok(RipResult { files, dpi })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End to end with the installed VectorCraft: preview, then a RIP of 3 copies.
    #[test]
    fn rip_with_vectorcraft() {
        if crate::vc::install_dir().is_none() {
            return;
        }
        let dir = std::env::temp_dir().join("ecorip_test");
        std::fs::create_dir_all(&dir).unwrap();
        let svg = dir.join("t.svg");
        std::fs::write(&svg, r##"<svg xmlns="http://www.w3.org/2000/svg" width="283.46" height="141.73"><rect width="283.46" height="141.73" fill="#e33"/></svg>"##).unwrap();
        let (w, h, cov) = make_preview(&svg.to_string_lossy(), &dir.join("p.png")).unwrap();
        assert!((w - 100.0).abs() < 1.0 && (h - 50.0).abs() < 1.0, "{w} {h}");
        assert!(cov[1] > 0.5, "{cov:?}");
        let mut s = JobSettings::default();
        s.copies = 3;
        s.resolution = 0;
        let job = Job {
            id: 1, name: "t".into(), path: svg.to_string_lossy().into(), art_w_mm: w, art_h_mm: h,
            queue: Queue::Hold, status: Status::Waiting, settings: s, coverage: cov, preview: None,
            ripped: vec![], rip_dpi: 0.0, mtime: 0, nest: None, started: None, expected_secs: 0.0, demo_progress: None,
        };
        let r = rip(&[job], &Setup::default(), &dir.join("out"), "t").unwrap();
        assert_eq!(r.files.len(), 1);
        let img = image::open(&r.files[0]).unwrap();
        // 3 copies of 100 mm with 10 mm gaps = 320 mm at 720 dpi
        let expect = (320.0 / 25.4 * r.dpi).round() as i64;
        assert!((img.width() as i64 - expect).abs() <= 2, "{} vs {}", img.width(), expect);
    }
}

/// Preview + coverage for a freshly added or edited file.
pub fn make_preview(path: &str, out: &Path) -> Result<(f32, f32, [f32; 4]), String> {
    let (w_pt, h_pt) = crate::vc::artboard_pt(path)?;
    let scale = (480.0 / w_pt.max(h_pt)).min(4.0);
    crate::vc::rasterize(path, out, scale)?;
    let img = image::open(out).map_err(|e| e.to_string())?.to_rgba8();
    let cov = coverage_from_rgba(img.as_raw());
    Ok((pt_to_mm(w_pt), pt_to_mm(h_pt), cov))
}
