//! Sample jobs used for `--screenshot` runs, with drawn placeholder artwork.
use crate::model::*;
use eframe::egui::{Color32, ColorImage};

fn job(id: u64, name: &str, w: f32, h: f32, queue: Queue, status: Status) -> Job {
    Job {
        id,
        name: name.into(),
        path: String::new(),
        art_w_mm: w,
        art_h_mm: h,
        queue,
        status,
        settings: JobSettings::default(),
        coverage: [0.32, 0.28, 0.30, 0.18],
        preview: None,
        ripped: vec![],
        rip_dpi: 0.0,
        mtime: 0,
        nest: None,
        started: None,
        expected_secs: 0.0,
        demo_progress: None,
    }
}

pub fn jobs() -> Vec<Job> {
    let mut v = vec![
        job(1, "Fachada Padaria.vectorcraft", 3000.0, 1000.0, Queue::Hold, Status::Waiting),
        job(2, "Adesivo Promoção.svg", 600.0, 400.0, Queue::Hold, Status::Waiting),
        job(7, "Cartaz A.vectorcraft", 700.0, 1000.0, Queue::Hold, Status::Waiting),
        job(8, "Cartaz B.vectorcraft", 700.0, 1000.0, Queue::Hold, Status::Waiting),
        job(3, "Banner Evento.pdf", 1500.0, 900.0, Queue::Rip, Status::Ripping),
        job(4, "Placa PVC.vectorcraft", 800.0, 600.0, Queue::Print, Status::Printing),
        job(5, "Logo Vitrine.svg", 500.0, 500.0, Queue::Print, Status::Ripped),
        job(6, "Etiquetas.svg", 300.0, 200.0, Queue::Done, Status::Printed),
    ];
    v[1].settings.copies = 4;
    v[2].nest = Some(100);
    v[3].nest = Some(100);
    v[4].demo_progress = Some(45.0);
    v[5].demo_progress = Some(70.0);
    v[0].settings.passes = 5;
    v[1].settings.media = 1;
    v
}

/// Placeholder artwork: a gradient with a sun and stripes, tinted per job.
pub fn art(id: u64, w_mm: f32, h_mm: f32) -> ColorImage {
    let w = 256usize;
    let h = ((256.0 * h_mm / w_mm.max(1.0)) as usize).clamp(24, 512);
    let hues = [(230, 70, 60), (40, 120, 220), (250, 170, 30), (40, 170, 110), (150, 60, 190), (20, 150, 170)];
    let (r0, g0, b0) = hues[(id as usize) % hues.len()];
    let mut img = ColorImage::new([w, h], Color32::WHITE);
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            let mut r = r0 as f32 * (1.0 - 0.45 * t) + 255.0 * 0.45 * t * 0.3;
            let mut g = g0 as f32 * (1.0 - 0.45 * t) + 255.0 * 0.45 * t * 0.3;
            let mut b = b0 as f32 * (1.0 - 0.45 * t) + 255.0 * 0.45 * t * 0.3;
            let (cx, cy) = (w as f32 * 0.7, h as f32 * 0.4);
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if d < h.min(w) as f32 * 0.22 {
                r = 255.0;
                g = 236.0;
                b = 150.0;
            }
            if y > h * 3 / 4 && (x / 18) % 2 == 0 {
                r *= 0.7;
                g *= 0.7;
                b *= 0.7;
            }
            if x < w / 2 && y > h / 6 && y < h / 6 + h / 8 {
                r = 255.0;
                g = 255.0;
                b = 255.0;
            }
            img.pixels[y * w + x] = Color32::from_rgb(r as u8, g as u8, b as u8);
        }
    }
    img
}
