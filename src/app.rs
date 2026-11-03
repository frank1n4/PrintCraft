//! App state and job actions. The drawing lives in ui.rs and ui_windows.rs.
use crate::model::*;
use crate::{demo, vc, winprint};
use eframe::egui::{self, Color32, TextureHandle, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;

pub const BG: Color32 = Color32::from_rgb(236, 238, 242);
pub const HEADER: Color32 = Color32::from_rgb(52, 64, 84);
pub const ACCENT: Color32 = Color32::from_rgb(0, 120, 215);

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub enum Msg {
    Imported { id: u64, w: f32, h: f32, cov: [f32; 4], preview: String, mtime: u64 },
    ImportFailed { id: u64, err: String },
    RipDone { ids: Vec<u64>, files: Vec<String>, dpi: f32 },
    RipFailed { ids: Vec<u64>, err: String },
    PrintDone { ids: Vec<u64> },
    PrintFailed { ids: Vec<u64>, err: String },
    Printers(Vec<String>),
    NewDoc { path: String },
    Notice(String, bool),
}

pub struct App {
    pub jobs: Vec<Job>,
    pub setup: Setup,
    pub next_id: u64,
    pub selected: Vec<u64>,
    pub tx: Sender<Msg>,
    rx: Receiver<Msg>,
    textures: HashMap<u64, (String, TextureHandle)>,
    pub printers: Vec<String>,
    pub data_dir: PathBuf,
    pub demo: bool,
    pub screenshot: Option<PathBuf>,
    pub frame: u32,
    pub shot_sent: bool,
    pub notice: Option<(String, bool)>,
    pub show_setup: bool,
    pub show_estimate: bool,
    pub show_about: bool,
    pub show_new_doc: bool,
    pub confirm_ink: Option<usize>,
    pub confirm_delete: bool,
    pub new_doc_name: String,
    pub new_doc_w: f32,
    pub new_doc_h: f32,
    last_watch: Instant,
    pub vc_found: bool,
    pub dirty: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>, files: Vec<String>) -> Self {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = BG;
        visuals.window_fill = Color32::from_rgb(248, 249, 251);
        visuals.selection.bg_fill = ACCENT;
        cc.egui_ctx.set_visuals(visuals);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(6.0, 5.0);
        style.spacing.button_padding = Vec2::new(8.0, 4.0);
        cc.egui_ctx.set_style(style);

        let data_dir = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("EcoRIP");
        let (tx, rx) = channel();
        let demo = screenshot.is_some();
        let mut app = Self {
            jobs: Vec::new(),
            setup: Setup::default(),
            next_id: 1,
            selected: Vec::new(),
            tx,
            rx,
            textures: HashMap::new(),
            printers: Vec::new(),
            data_dir,
            demo,
            screenshot,
            frame: 0,
            shot_sent: false,
            notice: None,
            show_setup: false,
            show_estimate: false,
            show_about: false,
            show_new_doc: false,
            confirm_ink: None,
            confirm_delete: false,
            new_doc_name: "Novo trabalho".into(),
            new_doc_w: 1000.0,
            new_doc_h: 500.0,
            last_watch: Instant::now(),
            vc_found: vc::install_dir().is_some(),
            dirty: false,
        };
        if demo {
            app.jobs = demo::jobs();
            app.next_id = 200;
            app.selected = vec![2];
            app.setup.ink_ml = [742.0, 655.0, 810.0, 430.0];
            app.setup.output = Output::WindowsPrinter("Ecossolvente XP600".into());
        } else {
            app.load();
            for j in &mut app.jobs {
                // anything interrupted by closing the app goes back to waiting
                if j.status.running() {
                    j.status = Status::Waiting;
                    j.queue = Queue::Hold;
                }
            }
            if let Output::Folder(f) = &app.setup.output {
                if f.is_empty() {
                    app.setup.output = Output::Folder(app.data_dir.join("Saida").to_string_lossy().into());
                }
            }
            app.refresh_printers();
            let paths: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
            if !paths.is_empty() {
                app.add_files(paths);
            }
        }
        app
    }

    // ---- persistence ----------------------------------------------------------------------
    fn load(&mut self) {
        let p = self.data_dir.join("estado.json");
        if let Ok(s) = std::fs::read_to_string(p) {
            if let Ok(st) = serde_json::from_str::<SavedState>(&s) {
                self.jobs = st.jobs;
                if let Some(s) = st.setup {
                    self.setup = s;
                }
                self.next_id = st.next_id.max(1);
            }
        }
    }

    pub fn save(&mut self) {
        if self.demo || !self.dirty {
            return;
        }
        self.dirty = false;
        let _ = std::fs::create_dir_all(&self.data_dir);
        let st = SavedState { jobs: self.jobs.clone(), setup: Some(self.setup.clone()), next_id: self.next_id };
        if let Ok(s) = serde_json::to_string_pretty(&st) {
            let _ = std::fs::write(self.data_dir.join("estado.json"), s);
        }
    }

    pub fn refresh_printers(&self) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Printers(winprint::list_printers()));
        });
    }

    pub fn job(&self, id: u64) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    /// Preview texture of a job, cached until its preview file changes.
    pub fn texture(&mut self, ctx: &egui::Context, id: u64) -> Option<TextureHandle> {
        let key = self.job(id)?.preview.clone().unwrap_or_else(|| "demo".into());
        if let Some((k, t)) = self.textures.get(&id) {
            if *k == key {
                return Some(t.clone());
            }
        }
        let (k, t) = self.load_texture(ctx, id)?;
        self.textures.insert(id, (k, t.clone()));
        Some(t)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(m) = self.rx.try_recv() {
            self.handle_message(m);
        }
        let mut last = self.last_watch;
        self.watch_files(&mut last);
        self.last_watch = last;
        if self.jobs.iter().any(|j| j.status.running()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        } else if !self.demo {
            ctx.request_repaint_after(std::time::Duration::from_secs(2));
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() && !self.demo {
            self.add_files(dropped);
        }
        let (ctrl_r, ctrl_k, del) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::R),
                i.modifiers.command && i.key_pressed(egui::Key::K),
                i.key_pressed(egui::Key::Delete),
            )
        });
        if ctrl_r {
            self.action_print();
        }
        if ctrl_k {
            self.show_setup = true;
        }
        if del && !ctx.wants_keyboard_input() && !self.selected.is_empty() {
            self.confirm_delete = true;
        }
        self.draw(ctx);
        self.windows(ctx);
        self.save();
        self.handle_screenshot(ctx);
    }
}

impl App {
    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frame += 1;
        ctx.request_repaint();
        if self.frame >= 120 && !self.shot_sent {
            self.shot_sent = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let mut buf = Vec::with_capacity(w * h * 4);
            for p in &img.pixels {
                buf.extend_from_slice(&p.to_array());
            }
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Some(rgba) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                let _ = rgba.save(&path);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
