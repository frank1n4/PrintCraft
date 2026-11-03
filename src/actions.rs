//! What the buttons do: add files, RIP, print, hold, nest, delete, VectorCraft link.
use crate::app::{App, Msg};
use crate::model::*;
use crate::{demo, rip, vc, winprint};
use eframe::egui::{self, TextureHandle};
use std::path::PathBuf;
use std::time::Instant;

impl App {
    pub fn add_files(&mut self, paths: Vec<PathBuf>) {
        for p in paths {
            let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !vc::VECTOR_EXTS.contains(&ext.as_str()) {
                self.notice = Some((format!("Formato não suportado: {}", p.display()), true));
                continue;
            }
            let id = self.next_id;
            self.next_id += 1;
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            self.jobs.push(Job {
                id,
                name,
                path: p.to_string_lossy().into(),
                art_w_mm: 100.0,
                art_h_mm: 100.0,
                queue: Queue::Hold,
                status: Status::Importing,
                settings: JobSettings::default(),
                coverage: [0.25; 4],
                preview: None,
                ripped: vec![],
                rip_dpi: 0.0,
                mtime: 0,
                nest: None,
                started: Some(Instant::now()),
                expected_secs: 3.0,
                demo_progress: None,
            });
            self.selected = vec![id];
            self.start_import(id);
        }
        self.dirty = true;
    }

    fn start_import(&mut self, id: u64) {
        let out = self.data_dir.join("trabalhos").join("previas");
        let tx = self.tx.clone();
        let Some(j) = self.jobs.iter_mut().find(|j| j.id == id) else { return };
        j.status = Status::Importing;
        j.started = Some(Instant::now());
        j.expected_secs = 3.0;
        let path = j.path.clone();
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&out);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let prev = out.join(format!("previa_{id}_{stamp}.png"));
            let mtime = vc::mtime(&path);
            let msg = match rip::make_preview(&path, &prev) {
                Ok((w, h, cov)) => Msg::Imported { id, w, h, cov, preview: prev.to_string_lossy().into(), mtime },
                Err(err) => Msg::ImportFailed { id, err },
            };
            let _ = tx.send(msg);
        });
    }

    /// Re-read files that were saved again in VectorCraft.
    pub fn watch_files(&mut self, last: &mut Instant) {
        if self.demo || last.elapsed().as_secs_f32() < 2.0 {
            return;
        }
        *last = Instant::now();
        let changed: Vec<u64> = self
            .jobs
            .iter()
            .filter(|j| matches!(j.queue, Queue::Hold | Queue::Print) && !j.status.running() && !j.path.is_empty())
            .filter(|j| {
                let m = vc::mtime(&j.path);
                m != 0 && j.mtime != 0 && m != j.mtime
            })
            .map(|j| j.id)
            .collect();
        for id in changed {
            if let Some(j) = self.jobs.iter_mut().find(|j| j.id == id) {
                j.queue = Queue::Hold;
                j.ripped.clear();
            }
            self.start_import(id);
            self.notice = Some(("Arquivo atualizado no VectorCraft — prévia refeita.".into(), false));
        }
    }

    /// Selected jobs grouped in print units: a nest is printed as one unit.
    pub fn units(&self, ids: &[u64]) -> Vec<Vec<u64>> {
        let mut units: Vec<Vec<u64>> = Vec::new();
        let mut seen = Vec::new();
        for id in ids {
            let Some(j) = self.job(*id) else { continue };
            match j.nest {
                Some(n) => {
                    if !seen.contains(&n) {
                        seen.push(n);
                        units.push(self.jobs.iter().filter(|x| x.nest == Some(n)).map(|x| x.id).collect());
                    }
                }
                None => units.push(vec![j.id]),
            }
        }
        units
    }

    pub fn nest_number(&self, nest: u64) -> usize {
        let mut seen: Vec<u64> = Vec::new();
        for j in &self.jobs {
            if let Some(n) = j.nest {
                if !seen.contains(&n) {
                    seen.push(n);
                }
            }
        }
        seen.iter().position(|n| *n == nest).map(|p| p + 1).unwrap_or(0)
    }

    pub fn action_print(&mut self) {
        let units = self.units(&self.selected.clone());
        if units.is_empty() {
            self.notice = Some(("Selecione um trabalho para imprimir.".into(), true));
            return;
        }
        for u in units {
            if u.iter().any(|id| self.job(*id).map(|j| j.status.running()).unwrap_or(true)) {
                continue;
            }
            let ready = u.iter().all(|id| {
                self.job(*id)
                    .map(|j| !j.ripped.is_empty() && matches!(j.queue, Queue::Print | Queue::Done))
                    .unwrap_or(false)
            });
            if ready {
                self.start_print(u);
            } else {
                self.start_rip(u);
            }
        }
        self.dirty = true;
    }

    fn start_rip(&mut self, ids: Vec<u64>) {
        if self.demo {
            return;
        }
        let jobs: Vec<Job> = ids.iter().filter_map(|id| self.job(*id).cloned()).collect();
        if jobs.is_empty() {
            return;
        }
        let area: f32 = jobs
            .iter()
            .map(|j| {
                let (w, h) = j.out_size_mm();
                w * h / 1e6 * j.settings.copies as f32
            })
            .sum();
        for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
            j.queue = Queue::Rip;
            j.status = Status::Ripping;
            j.started = Some(Instant::now());
            j.expected_secs = 3.0 + area * 4.0;
            j.ripped.clear();
        }
        let setup = self.setup.clone();
        let base = if jobs.len() > 1 { format!("nest_{}", jobs[0].id) } else { jobs[0].name.clone() };
        let dir = self.data_dir.join("trabalhos").join("rip").join(format!("{}", jobs[0].id));
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let msg = match rip::rip(&jobs, &setup, &dir, &base) {
                Ok(r) => Msg::RipDone {
                    ids,
                    files: r.files.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                    dpi: r.dpi,
                },
                Err(err) => Msg::RipFailed { ids, err },
            };
            let _ = tx.send(msg);
        });
    }

    fn start_print(&mut self, ids: Vec<u64>) {
        if self.demo {
            return;
        }
        let Some(first) = ids.first().and_then(|id| self.job(*id)).cloned() else { return };
        let files: Vec<PathBuf> = first.ripped.iter().map(PathBuf::from).collect();
        let dpi = first.rip_dpi;
        for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
            j.queue = Queue::Print;
            j.status = Status::Printing;
            j.started = Some(Instant::now());
            j.expected_secs = 4.0 * files.len() as f32;
        }
        let output = self.setup.output.clone();
        let tx = self.tx.clone();
        let doc = first.name.clone();
        std::thread::spawn(move || {
            let res = match output {
                Output::WindowsPrinter(p) => winprint::print_files(&p, &files, dpi, &doc),
                Output::Folder(dir) => {
                    let d = PathBuf::from(dir);
                    std::fs::create_dir_all(&d).map_err(|e| e.to_string()).and_then(|_| {
                        for f in &files {
                            let name = f.file_name().unwrap_or_default();
                            std::fs::copy(f, d.join(name)).map_err(|e| e.to_string())?;
                        }
                        Ok(())
                    })
                }
            };
            let _ = tx.send(match res {
                Ok(()) => Msg::PrintDone { ids },
                Err(err) => Msg::PrintFailed { ids, err },
            });
        });
    }

    pub fn action_hold(&mut self) {
        let ids: Vec<u64> = self.units(&self.selected.clone()).concat();
        for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
            if !j.status.running() {
                j.queue = Queue::Hold;
                j.status = Status::Waiting;
                j.demo_progress = None;
            }
        }
        self.dirty = true;
    }

    pub fn action_nest(&mut self) {
        let sel: Vec<&Job> = self.selected.iter().filter_map(|id| self.job(*id)).collect();
        let err = if sel.len() < MIN_JOBS_TO_NEST {
            Some("Para aninhar, selecione pelo menos 2 trabalhos (Ctrl + clique).")
        } else if sel.iter().any(|j| j.nest.is_some()) {
            Some("Não é possível juntar trabalhos a um aninhamento já feito. Desaninhe e aninhe de novo com os trabalhos extras.")
        } else if sel.iter().any(|j| j.queue != Queue::Hold || j.status.running()) {
            Some("Só trabalhos da fila de espera podem ser aninhados.")
        } else if sel.iter().any(|j| j.settings.nest_key() != sel[0].settings.nest_key()) {
            Some("Os trabalhos selecionados têm mídia, modo de impressão, resolução ou cores diferentes.")
        } else if sel.iter().any(|j| j.out_size_mm().0 > self.setup.printable_mm()) {
            Some("Um ou mais trabalhos não podem ser aninhados por causa da largura da mídia. A divisão em painéis só é feita trabalho a trabalho.")
        } else {
            None
        };
        if let Some(e) = err {
            self.notice = Some((e.into(), true));
            return;
        }
        let nest = self.next_id;
        self.next_id += 1;
        let sel = self.selected.clone();
        for j in self.jobs.iter_mut().filter(|j| sel.contains(&j.id)) {
            j.nest = Some(nest);
        }
        self.dirty = true;
    }

    pub fn action_unnest(&mut self) {
        let nests: Vec<u64> = self.selected.iter().filter_map(|id| self.job(*id).and_then(|j| j.nest)).collect();
        for j in &mut self.jobs {
            if j.nest.map(|n| nests.contains(&n)).unwrap_or(false) && !j.status.running() {
                j.nest = None;
                if j.queue != Queue::Done {
                    j.queue = Queue::Hold;
                    j.status = Status::Waiting;
                    j.ripped.clear();
                }
            }
        }
        self.dirty = true;
    }

    pub fn action_delete(&mut self) {
        let sel = self.selected.clone();
        if sel.iter().any(|id| self.job(*id).map(|j| j.status.running()).unwrap_or(false)) {
            self.notice = Some(("O trabalho está ocupado no momento.".into(), true));
        }
        self.jobs.retain(|j| !(sel.contains(&j.id) && !j.status.running()));
        let ids: Vec<u64> = self.jobs.iter().map(|j| j.id).collect();
        self.selected.retain(|id| ids.contains(id));
        self.dirty = true;
    }

    pub fn open_in_vectorcraft(&mut self) {
        if self.demo {
            return;
        }
        let path = self.selected.first().and_then(|id| self.job(*id)).map(|j| j.path.clone());
        match vc::open_editor(path.as_deref().filter(|p| !p.is_empty())) {
            Err(e) => self.notice = Some((e, true)),
            Ok(()) => {
                self.notice = Some(("Aberto no VectorCraft. Salve lá e o trabalho é atualizado aqui.".into(), false))
            }
        }
    }

    pub fn create_new_doc(&mut self) {
        let dir = self.data_dir.join("Desenhos");
        let safe: String = self
            .new_doc_name
            .chars()
            .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
            .collect();
        let path = dir.join(format!("{safe}.vectorcraft"));
        let (w, h) = (self.new_doc_w, self.new_doc_h);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&dir);
            let msg = match vc::new_document(&path, w, h) {
                Ok(()) => Msg::NewDoc { path: path.to_string_lossy().into() },
                Err(e) => Msg::Notice(format!("VectorCraft: {e}"), true),
            };
            let _ = tx.send(msg);
        });
    }

    pub fn pick_files(&mut self) {
        if self.demo {
            return;
        }
        if let Some(files) = rfd::FileDialog::new()
            .add_filter("Arquivos de impressão", &vc::VECTOR_EXTS)
            .set_title("Adicionar trabalho")
            .pick_files()
        {
            self.add_files(files);
        }
    }

    pub fn handle_message(&mut self, m: Msg) {
        self.dirty = true;
        match m {
            Msg::Imported { id, w, h, cov, preview, mtime } => {
                if let Some(j) = self.jobs.iter_mut().find(|j| j.id == id) {
                    if let Some(old) = j.preview.replace(preview) {
                        let _ = std::fs::remove_file(old);
                    }
                    j.art_w_mm = w;
                    j.art_h_mm = h;
                    j.coverage = cov;
                    j.mtime = mtime;
                    j.status = Status::Waiting;
                    j.started = None;
                }
            }
            Msg::ImportFailed { id, err } => {
                if let Some(j) = self.jobs.iter_mut().find(|j| j.id == id) {
                    j.status = Status::Error(err);
                    j.mtime = vc::mtime(&j.path);
                }
            }
            Msg::RipDone { ids, files, dpi } => {
                for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
                    j.queue = Queue::Print;
                    j.status = Status::Ripped;
                    j.ripped = files.clone();
                    j.rip_dpi = dpi;
                    j.started = None;
                }
                if self.setup.print_while_rip {
                    self.start_print(ids);
                }
            }
            Msg::RipFailed { ids, err } => {
                for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
                    j.queue = Queue::Hold;
                    j.status = Status::Error(err.clone());
                }
            }
            Msg::PrintDone { ids } => {
                let jobs: Vec<Job> = ids.iter().filter_map(|id| self.job(*id).cloned()).collect();
                let refs: Vec<&Job> = jobs.iter().collect();
                let est = estimate(&refs, &self.setup);
                for c in 0..4 {
                    self.setup.ink_ml[c] = (self.setup.ink_ml[c] - est.ink_ml[c]).max(0.0);
                }
                for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
                    j.queue = Queue::Done;
                    j.status = Status::Printed;
                    j.started = None;
                }
            }
            Msg::PrintFailed { ids, err } => {
                for j in self.jobs.iter_mut().filter(|j| ids.contains(&j.id)) {
                    j.status = Status::Error(err.clone());
                }
            }
            Msg::Printers(p) => {
                self.printers = p;
                self.dirty = false;
            }
            Msg::NewDoc { path } => {
                self.add_files(vec![PathBuf::from(&path)]);
                let _ = vc::open_editor(Some(&path));
                self.notice = Some(("Desenho criado e aberto no VectorCraft.".into(), false));
            }
            Msg::Notice(t, e) => self.notice = Some((t, e)),
        }
    }

    pub fn load_texture(&self, ctx: &egui::Context, id: u64) -> Option<(String, TextureHandle)> {
        let j = self.job(id)?;
        let key = j.preview.clone().unwrap_or_else(|| "demo".into());
        let img = if let Some(p) = &j.preview {
            let im = image::open(p).ok()?.to_rgba8();
            egui::ColorImage::from_rgba_unmultiplied([im.width() as usize, im.height() as usize], im.as_raw())
        } else if self.demo {
            demo::art(id, j.art_w_mm, j.art_h_mm)
        } else {
            return None;
        };
        Some((key, ctx.load_texture(format!("job{id}"), img, egui::TextureOptions::LINEAR)))
    }
}
