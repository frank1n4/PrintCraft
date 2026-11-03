//! Media preview, pop-up windows (setup, estimate, new drawing, confirmations) and helpers.
use crate::app::{rgb, App, ACCENT, HEADER};
use crate::model::*;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use std::collections::HashMap;

impl App {
    pub fn preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let unit = self.units(&self.selected.clone()).into_iter().next().unwrap_or_default();
        let jobs: Vec<Job> = unit.iter().filter_map(|id| self.job(*id).cloned()).collect();
        let printable = self.setup.printable_mm();
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(rect, 4.0, Color32::from_rgb(70, 76, 88));
        if jobs.is_empty() {
            p.text(rect.center(), Align2::CENTER_CENTER, "Selecione um trabalho para ver como ele fica na mídia", FontId::proportional(14.0), Color32::from_gray(210));
            return;
        }
        let refs: Vec<&Job> = jobs.iter().collect();
        let lay = layout(&refs, printable);
        let media_w = self.setup.media_width_mm;
        let shown_len = lay.length_mm.max(200.0) + 60.0;
        let area = Rect::from_min_max(rect.min + Vec2::new(40.0, 40.0), rect.max - Vec2::new(40.0, 30.0));
        let s = (area.width() / media_w).min(area.height() / shown_len);
        let media = Rect::from_min_size(Pos2::new(area.center().x - media_w * s / 2.0, area.top()), Vec2::new(media_w * s, shown_len * s));
        p.rect_filled(media, 0.0, Color32::WHITE);
        let shade = Color32::from_rgb(220, 224, 232);
        p.rect_filled(Rect::from_min_max(media.min, Pos2::new(media.left() + self.setup.margin_left_mm * s, media.bottom())), 0.0, shade);
        p.rect_filled(Rect::from_min_max(Pos2::new(media.right() - self.setup.margin_right_mm * s, media.top()), media.max), 0.0, shade);
        // ruler: a tick every 100 mm, a number every 500 mm
        let mut mm = 0.0f32;
        while mm <= media_w + 0.1 {
            let x = media.left() + mm * s;
            let long = (mm.round() as i32) % 500 == 0;
            let len = if long { 8.0 } else { 4.0 };
            p.line_segment([Pos2::new(x, media.top() - len), Pos2::new(x, media.top())], Stroke::new(1.0_f32, Color32::from_gray(200)));
            if long {
                p.text(Pos2::new(x, media.top() - 9.0), Align2::CENTER_BOTTOM, format!("{mm:.0}"), FontId::proportional(9.5), Color32::from_gray(210));
            }
            mm += 100.0;
        }
        let origin = Pos2::new(media.left() + self.setup.margin_left_mm * s, media.top() + 30.0 * s);
        let mut tex: HashMap<u64, Option<TextureHandle>> = HashMap::new();
        for j in &jobs {
            let t = self.texture(ctx, j.id);
            tex.insert(j.id, t);
        }
        if lay.tiles > 1 {
            let j = &jobs[0];
            let it = &lay.items[0];
            let step = printable - TILE_OVERLAP_MM;
            for t in 0..lay.tiles {
                let x0 = t as f32 * step;
                let w = (it.w - x0).min(printable);
                let r = Rect::from_min_size(Pos2::new(origin.x, origin.y + t as f32 * (it.h + NEST_GAP_MM) * s), Vec2::new(w * s, it.h * s));
                draw_job(&p, r, tex.get(&j.id).cloned().flatten(), j, Some((x0 / it.w, (x0 + w) / it.w)));
                p.text(r.left_top() + Vec2::new(4.0, 3.0), Align2::LEFT_TOP, format!("Painel {}", t + 1), FontId::proportional(11.0), Color32::WHITE);
            }
        } else {
            for it in &lay.items {
                let Some(j) = jobs.iter().find(|j| j.id == it.job) else { continue };
                let r = Rect::from_min_size(origin + Vec2::new(it.x * s, it.y * s), Vec2::new(it.w * s, it.h * s));
                draw_job(&p, r, tex.get(&j.id).cloned().flatten(), j, None);
            }
        }
        let title = if jobs.len() > 1 {
            format!("{}{} — {} trabalhos", NEST_PREFIX, self.nest_number(jobs[0].nest.unwrap_or(0)), jobs.len())
        } else {
            format!("Prévia na mídia — {}", jobs[0].name)
        };
        p.text(rect.left_top() + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, title, FontId::proportional(13.0), Color32::WHITE);
        let info = if lay.tiles > 1 {
            format!("Maior que a mídia: {} painéis com {:.0} mm de sobreposição", lay.tiles, TILE_OVERLAP_MM)
        } else {
            format!("Ocupa {:.0} × {:.0} mm da mídia", lay.width_mm.min(printable), lay.length_mm)
        };
        p.text(rect.left_bottom() + Vec2::new(10.0, -8.0), Align2::LEFT_BOTTOM, info, FontId::proportional(12.0), Color32::from_gray(225));
        p.text(
            rect.right_bottom() + Vec2::new(-10.0, -8.0),
            Align2::RIGHT_BOTTOM,
            format!("Largura da mídia {media_w:.0} mm  ·  a mídia avança para baixo"),
            FontId::proportional(11.0),
            Color32::from_gray(190),
        );
    }

    pub fn windows(&mut self, ctx: &egui::Context) {
        if let Some((text, err)) = self.notice.clone() {
            let mut open = true;
            egui::Window::new(if err { "Atenção" } else { "Aviso" })
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_TOP, [0.0, 90.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_max_width(380.0);
                    ui.label(&text);
                    if ui.button("OK").clicked() {
                        self.notice = None;
                    }
                });
            if !open {
                self.notice = None;
            }
        }
        if let Some(c) = self.confirm_ink {
            egui::Window::new("Tinta virtual").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                let which = if c < 4 { INK_NAMES[c].to_string() } else { "todas as cores".to_string() };
                ui.label(format!("A tinta virtual ({which}) será reduzida em {VIRTUAL_INK_DECREMENT_ML:.0} ml. Continuar?"));
                ui.horizontal(|ui| {
                    if ui.button("Sim").clicked() {
                        for i in 0..4 {
                            if c == 4 || c == i {
                                self.setup.ink_ml[i] = (self.setup.ink_ml[i] - VIRTUAL_INK_DECREMENT_ML).max(0.0);
                            }
                        }
                        self.dirty = true;
                        self.confirm_ink = None;
                    }
                    if ui.button("Não").clicked() {
                        self.confirm_ink = None;
                    }
                });
            });
        }
        if self.confirm_delete {
            egui::Window::new("Excluir").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label(format!("Excluir {} trabalho(s) da lista? Os arquivos originais não são apagados.", self.selected.len()));
                ui.horizontal(|ui| {
                    if ui.button("Excluir").clicked() {
                        self.action_delete();
                        self.confirm_delete = false;
                    }
                    if ui.button("Cancelar").clicked() {
                        self.confirm_delete = false;
                    }
                });
            });
        }
        if self.show_estimate {
            self.estimate_window(ctx);
        }
        if self.show_setup {
            self.setup_window(ctx);
        }
        if self.show_new_doc {
            self.new_doc_window(ctx);
        }
        if self.show_about {
            let mut open = true;
            egui::Window::new("Sobre o EcoRIP").open(&mut open).collapsible(false).show(ctx, |ui| {
                ui.label("Central de impressão para impressoras ecossolventes, ligada ao VectorCraft.");
                ui.label("Filas de espera, RIP e impressão, aninhamento, painéis, estimativa de tinta e custo.");
            });
            self.show_about = open;
        }
    }

    fn estimate_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let units = self.units(&self.selected.clone());
        egui::Window::new("Estimativa do trabalho").open(&mut open).collapsible(false).default_width(340.0).show(ctx, |ui| {
            if units.is_empty() {
                ui.label("Selecione um ou mais trabalhos.");
            }
            let (mut cost, mut minutes) = (0.0, 0.0);
            for (k, u) in units.iter().enumerate() {
                let jobs: Vec<Job> = u.iter().filter_map(|i| self.job(*i).cloned()).collect();
                if jobs.is_empty() {
                    continue;
                }
                let refs: Vec<&Job> = jobs.iter().collect();
                let e = estimate(&refs, &self.setup);
                let name = if jobs.len() > 1 { format!("Aninhamento com {} trabalhos", jobs.len()) } else { jobs[0].name.clone() };
                ui.label(RichText::new(name).strong());
                estimate_grid(ui, &format!("est_win_{k}"), &e);
                cost += e.cost_media + e.cost_ink;
                minutes += e.minutes;
                ui.separator();
            }
            ui.label(RichText::new(format!("Total: R$ {cost:.2}  ·  {minutes:.0} min")).strong());
            ui.label(RichText::new("Tinta calculada pela cobertura da arte com 3 pL por gota; tempo aproximado.").small().color(Color32::GRAY));
        });
        self.show_estimate = open;
    }

    fn new_doc_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut create = false;
        egui::Window::new("Novo desenho no VectorCraft").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label("Cria um arquivo em branco do tamanho da peça, abre no VectorCraft e já coloca na fila de espera.");
            egui::Grid::new("nd").num_columns(2).show(ui, |ui| {
                ui.label("Nome");
                ui.text_edit_singleline(&mut self.new_doc_name);
                ui.end_row();
                ui.label("Largura");
                ui.add(egui::DragValue::new(&mut self.new_doc_w).range(10.0..=20000.0).suffix(" mm"));
                ui.end_row();
                ui.label("Altura");
                ui.add(egui::DragValue::new(&mut self.new_doc_h).range(10.0..=50000.0).suffix(" mm"));
                ui.end_row();
            });
            if ui.button("Usar a largura útil da mídia").clicked() {
                self.new_doc_w = self.setup.printable_mm();
            }
            create = ui.add(egui::Button::new(RichText::new("Criar e abrir").color(Color32::WHITE)).fill(ACCENT)).clicked();
        });
        if create {
            self.create_new_doc();
            open = false;
        }
        self.show_new_doc = open;
    }

    fn setup_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let printers = self.printers.clone();
        let default_folder = self.data_dir.join("Saida").to_string_lossy().to_string();
        let mut refresh = false;
        let mut media_count = None;
        egui::Window::new("Propriedades da configuração").open(&mut open).default_width(420.0).show(ctx, |ui| {
            let st = &mut self.setup;
            egui::Grid::new("setup").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("Nome");
                ui.text_edit_singleline(&mut st.name);
                ui.end_row();
                ui.label("Impressora");
                ui.text_edit_singleline(&mut st.printer_model);
                ui.end_row();
                ui.label("Largura da mídia");
                ui.add(egui::DragValue::new(&mut st.media_width_mm).range(100.0..=5000.0).suffix(" mm"));
                ui.end_row();
                ui.label("Margem esquerda");
                ui.add(egui::DragValue::new(&mut st.margin_left_mm).range(0.0..=200.0).suffix(" mm"));
                ui.end_row();
                ui.label("Margem direita");
                ui.add(egui::DragValue::new(&mut st.margin_right_mm).range(0.0..=200.0).suffix(" mm"));
                ui.end_row();
                ui.label("Preço da tinta");
                ui.add(egui::DragValue::new(&mut st.ink_price_ml).range(0.0..=100.0).speed(0.01).prefix("R$ ").suffix(" /ml"));
                ui.end_row();
            });
            ui.checkbox(&mut st.print_while_rip, "Imprimir assim que o RIP terminar");
            ui.separator();
            ui.label(RichText::new("Para onde vai a impressão").strong());
            let is_printer = matches!(st.output, Output::WindowsPrinter(_));
            if ui.radio(!is_printer, "Pasta de saída (arquivos PNG com o dpi real)").clicked() && is_printer {
                st.output = Output::Folder(default_folder.clone());
            }
            if let Output::Folder(f) = &mut st.output {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(f).desired_width(280.0));
                    if ui.button("Escolher...").clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            *f = d.to_string_lossy().into();
                        }
                    }
                });
            }
            if ui.radio(is_printer, "Impressora do Windows").clicked() && !is_printer {
                st.output = Output::WindowsPrinter(printers.first().cloned().unwrap_or_default());
            }
            if let Output::WindowsPrinter(p) = &mut st.output {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("printer").selected_text(p.as_str()).width(260.0).show_ui(ui, |ui| {
                        for name in &printers {
                            ui.selectable_value(p, name.clone(), name);
                        }
                    });
                    if ui.button("Atualizar").clicked() {
                        refresh = true;
                    }
                });
            }
            ui.separator();
            ui.label(RichText::new("Mídias e preço por m²").strong());
            let mut remove = None;
            for (i, m) in st.media.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(200.0));
                    ui.add(egui::DragValue::new(&mut m.price_m2).range(0.0..=1000.0).speed(0.1).prefix("R$ "));
                    if ui.small_button("✖").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                if st.media.len() > 1 {
                    st.media.remove(i);
                    media_count = Some(st.media.len());
                }
            }
            if ui.button("+ Adicionar mídia").clicked() {
                st.media.push(Media { name: "Nova mídia".into(), price_m2: 10.0 });
            }
        });
        if let Some(n) = media_count {
            for j in &mut self.jobs {
                j.settings.media = j.settings.media.min(n - 1);
            }
        }
        if refresh {
            self.refresh_printers();
        }
        if !open {
            self.dirty = true;
        }
        self.show_setup = open;
    }
}

// ---- helpers ----------------------------------------------------------------------------------

pub fn strs(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

pub fn combo(ui: &mut egui::Ui, label: &str, value: &mut usize, options: &[String]) {
    ui.label(label);
    let cur = options.get(*value).cloned().unwrap_or_default();
    egui::ComboBox::from_id_salt(label).selected_text(cur).width(170.0).show_ui(ui, |ui| {
        for (i, o) in options.iter().enumerate() {
            ui.selectable_value(value, i, o);
        }
    });
    ui.end_row();
}

pub fn section_title(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).strong().size(14.0).color(HEADER));
    ui.add_space(2.0);
}

pub fn estimate_grid(ui: &mut egui::Ui, id: &str, e: &Estimate) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
        ui.label("Área impressa");
        ui.label(format!("{:.2} m²", e.area_m2));
        ui.end_row();
        ui.label("Mídia gasta");
        ui.label(format!("{:.2} m²", e.media_m2));
        ui.end_row();
        ui.label("Tinta (ml)");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for c in 0..4 {
                ui.label(RichText::new("■").color(rgb(INK_COLORS[c])));
                ui.label(format!("{:.1} ", e.ink_ml[c]));
            }
        });
        ui.end_row();
        ui.label("Tempo aprox.");
        ui.label(format!("{:.0} min", e.minutes));
        ui.end_row();
        ui.label("Custo mídia");
        ui.label(format!("R$ {:.2}", e.cost_media));
        ui.end_row();
        ui.label("Custo tinta");
        ui.label(format!("R$ {:.2}", e.cost_ink));
        ui.end_row();
        ui.label(RichText::new("Total").strong());
        ui.label(RichText::new(format!("R$ {:.2}", e.cost_media + e.cost_ink)).strong());
        ui.end_row();
    });
}

pub fn draw_job(p: &egui::Painter, r: Rect, tex: Option<TextureHandle>, j: &Job, tile: Option<(f32, f32)>) {
    match tex {
        Some(t) => {
            // uv of each shown corner: top-left, top-right, bottom-right, bottom-left
            let mut uv = match tile {
                Some((u0, u1)) => [Pos2::new(u0, 0.0), Pos2::new(u1, 0.0), Pos2::new(u1, 1.0), Pos2::new(u0, 1.0)],
                None => {
                    let mut c = [Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0)];
                    c.rotate_right((ROTATIONS[j.settings.rotate] / 90) as usize);
                    c
                }
            };
            if j.settings.mirror {
                uv.swap(0, 1);
                uv.swap(2, 3);
            }
            let mut mesh = egui::Mesh::with_texture(t.id());
            let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
            for k in 0..4 {
                mesh.vertices.push(egui::epaint::Vertex { pos: corners[k], uv: uv[k], color: Color32::WHITE });
            }
            mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
            p.add(egui::Shape::mesh(mesh));
        }
        None => {
            p.rect_filled(r, 0.0, Color32::from_rgb(210, 216, 228));
            p.text(r.center(), Align2::CENTER_CENTER, "sem prévia", FontId::proportional(11.0), Color32::DARK_GRAY);
        }
    }
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(40, 110, 200)));
}

pub fn draw_printer_icon(p: &egui::Painter, r: Rect) {
    let body = Rect::from_min_size(r.min + Vec2::new(0.0, 8.0), Vec2::new(r.width(), 12.0));
    let paper = Rect::from_min_size(r.min + Vec2::new(5.0, 2.0), Vec2::new(r.width() - 10.0, 7.0));
    p.rect_filled(paper, 1.0, Color32::WHITE);
    p.rect_stroke(paper, 1.0, Stroke::new(1.0_f32, Color32::GRAY));
    p.rect_filled(body, 2.0, Color32::from_rgb(70, 80, 100));
    p.rect_filled(Rect::from_min_size(r.min + Vec2::new(4.0, 20.0), Vec2::new(r.width() - 8.0, 5.0)), 0.0, Color32::from_rgb(0, 160, 227));
    p.circle_filled(body.right_center() - Vec2::new(5.0, 0.0), 2.0, Color32::from_rgb(90, 230, 120));
}

pub fn draw_cutter_icon(p: &egui::Painter, r: Rect) {
    let body = Rect::from_min_size(r.min + Vec2::new(0.0, 6.0), Vec2::new(r.width(), 10.0));
    p.rect_filled(body, 2.0, Color32::from_gray(150));
    p.line_segment([body.center_top(), body.center_bottom() + Vec2::new(4.0, 5.0)], Stroke::new(2.0_f32, Color32::from_gray(90)));
}
