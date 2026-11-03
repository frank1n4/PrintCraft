//! Bridge to VectorCraft: read sizes, make previews, rasterise for the RIP, open for editing.
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const VECTOR_EXTS: [&str; 12] =
    ["vectorcraft", "svg", "svgz", "pdf", "ai", "eps", "ps", "png", "jpg", "jpeg", "tif", "tiff"];

pub fn install_dir() -> Option<PathBuf> {
    let mut cands = vec![PathBuf::from(r"C:\Program Files\ArtCraft\VectorCraft")];
    if let Ok(pf) = std::env::var("ProgramFiles") {
        cands.push(Path::new(&pf).join("ArtCraft").join("VectorCraft"));
    }
    if let Ok(la) = std::env::var("LOCALAPPDATA") {
        cands.push(Path::new(&la).join("Programs").join("ArtCraft").join("VectorCraft"));
    }
    cands.into_iter().find(|d| d.join("vectorcraft-cli.exe").exists())
}

fn cli() -> Result<PathBuf, String> {
    install_dir()
        .map(|d| d.join("vectorcraft-cli.exe"))
        .ok_or_else(|| "VectorCraft não encontrado".to_string())
}

fn run(args: &[&str], stdin: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new(cli()?);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = if let Some(input) = stdin {
        use std::io::Write;
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        if let Some(mut si) = child.stdin.take() {
            let _ = si.write_all(input.as_bytes());
        }
        child.wait_with_output().map_err(|e| e.to_string())?
    } else {
        cmd.output().map_err(|e| e.to_string())?
    };
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() { String::from_utf8_lossy(&out.stdout).to_string() } else { err };
        return Err(msg.lines().next().unwrap_or("falhou").chars().take(120).collect());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Size of the first artboard in points.
pub fn artboard_pt(file: &str) -> Result<(f32, f32), String> {
    let out = run(&["info", file], None)?;
    let v: serde_json::Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
    let r = v["artboards"][0]["rect"].as_array().ok_or("arquivo sem prancheta")?;
    let f = |i: usize| r.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let (w, h) = ((f(2) - f(0)).abs(), (f(3) - f(1)).abs());
    if w <= 0.0 || h <= 0.0 {
        return Err("prancheta vazia".into());
    }
    Ok((w, h))
}

/// Raster export at `scale` pixels per point.
pub fn rasterize(file: &str, out_png: &Path, scale: f32) -> Result<(), String> {
    let s = format!("{scale:.5}");
    run(&["convert", file, &out_png.to_string_lossy(), "--scale", &s], None).map(|_| ())
}

/// A blank document whose artboard has the given size in mm.
pub fn new_document(out: &Path, w_mm: f32, h_mm: f32) -> Result<(), String> {
    let w = crate::model::mm_to_pt(w_mm);
    let h = crate::model::mm_to_pt(h_mm);
    let steps = format!(
        r#"[{{"command":"artboard.setProps","params":{{"index":0,"x":0,"y":0,"width":{w:.2},"height":{h:.2}}}}}]"#
    );
    run(&["run", "--commands", "-", "--export", &out.to_string_lossy()], Some(&steps)).map(|_| ())
}

/// Open the file in the VectorCraft editor (does not wait).
pub fn open_editor(file: Option<&str>) -> Result<(), String> {
    let exe = install_dir()
        .map(|d| d.join("vectorcraft.exe"))
        .ok_or("VectorCraft não encontrado")?;
    let mut cmd = Command::new(exe);
    if let Some(f) = file {
        cmd.arg(f);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn mtime(path: &str) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
