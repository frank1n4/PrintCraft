#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod demo;
mod ui;
mod ui_windows;
mod model;
mod rip;
mod vc;
mod winprint;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut screenshot = None;
    let mut files = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" && i + 1 < args.len() {
            screenshot = Some(std::path::PathBuf::from(&args[i + 1]));
            i += 2;
        } else {
            files.push(args[i].clone());
            i += 1;
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 820.0])
            .with_min_inner_size([980.0, 620.0])
            .with_title("EcoRIP — Central de impressão ecossolvente"),
        ..Default::default()
    };
    eframe::run_native(
        "EcoRIP",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, screenshot, files)))),
    )
}
