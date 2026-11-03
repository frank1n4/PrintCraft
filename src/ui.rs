//! Main window layout: menu, toolbar, setups/ink on the left, queues in the middle,
//! job properties on the right, status bar at the bottom.
use crate::app::{rgb, App, ACCENT, HEADER};
use crate::model::*;
use crate::ui_windows::{combo, draw_cutter_icon, draw_printer_icon, estimate_grid, section_title, strs};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

fn status_dot(ui: &mut egui::Ui, col: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(12.0, 14.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 5.0, col);
        ui.label(RichText::new(text).color(col));
    });
}

impl App {
    pub fn draw(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu_bar(ui));
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none().fill(Color32::from_rgb(246, 247, 250)).inner_margin(egui::Margin::symmetric(8.0, 6.0)))
            .show(ctx, |ui| self.toolbar(ui));
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::none().fill(Color32::from_rgb(222, 225, 232)).inner_margin(egui::Margin::symmetric(8.0, 3.0)))
            .show(ctx, |ui| self.status_bar(ui));
        egui::SidePanel::left("left").exact_width(232.0).resizable(false).show(ctx, |ui| {
            ui.add_space(6.0);
            self.left_panel(ui);
        });
        egui::SidePanel::right("right").exact_width(318.0).resizable(false).show(ctx, |ui| {
            ui.add_space(6.0);
            egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui));
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let total = ui.available_height();
            let preview_h = (total * 0.30).clamp(170.0, 300.0);
            let fixed = 4.0 * (22.0 + 18.0 + 4.0 + 15.0);
            let rows = (total - preview_h - fixed).max(160.0);
            self.queue_table(ui, "Fila de espera", Queue::Hold, rgb(HOLD_QUEUE_COLOR), rows * 0.37);
            self.queue_table(ui, "Fila de RIP", Queue::Rip, rgb(RIP_QUEUE_COLOR), rows * 0.18);
            self.queue_table(ui, "Fila de impressão", Queue::Print, rgb(PRINT_QUEUE_COLOR), rows * 0.25);
            self.queue_table(ui, "Trabalhos impressos", Queue::Done, Color32::from_rgb(232, 240, 232), rows * 0.20);
            self.preview(ui, ctx);
        });
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Arquivo", |ui| {
                if ui.button("Adicionar trabalho...").clicked() {
                    ui.close_menu();
                    self.pick_files();
                }
                if ui.button("Novo desenho no VectorCraft...").clicked() {
                    ui.close_menu();
                    self.show_new_doc = true;
                }
                if ui.button("Abrir no VectorCraft").clicked() {
                    ui.close_menu();
                    self.open_in_vectorcraft();
                }
                ui.separator();
                if ui.button("Sair").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Trabalho", |ui| {
                if ui.button("RIP e imprimir    Ctrl+R").clicked() {
                    ui.close_menu();
                    self.action_print();
                }
                if ui.button("Mover para espera").clicked() {
                    ui.close_menu();
                    self.action_hold();
                }
                if ui.button("Aninhar trabalhos").clicked() {
                    ui.close_menu();
                    self.action_nest();
                }
                if ui.button("Desaninhar trabalhos").clicked() {
                    ui.close_menu();
                    self.action_unnest();
                }
                if ui.button("Estimativa do trabalho...").clicked() {
                    ui.close_menu();
                    self.show_estimate = true;
                }
                ui.separator();
                if ui.button("Excluir    Del").clicked() {
                    ui.close_menu();
                    self.confirm_delete = !self.selected.is_empty();
                }
            });
            ui.menu_button("Configuração", |ui| {
                if ui.button("Propriedades da configuração...    Ctrl+K").clicked() {
                    ui.close_menu();
                    self.show_setup = true;
                }
            });
            ui.menu_button("Ajuda", |ui| {
                if ui.button("Sobre o EcoRIP").clicked() {
                    ui.close_menu();
                    self.show_about = true;
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let big = |t: &str| egui::Button::new(RichText::new(t).size(13.5)).min_size(Vec2::new(0.0, 30.0));
            if ui.add(big("+ Adicionar")).on_hover_text("Adicionar arquivos à fila de espera").clicked() {
                self.pick_files();
            }
            if ui.add(big("✏ Novo no VectorCraft")).on_hover_text("Criar um desenho do tamanho da peça").clicked() {
                self.show_new_doc = true;
            }
            if ui.add(big("↗ Abrir no VectorCraft")).on_hover_text("Editar o trabalho selecionado").clicked() {
                self.open_in_vectorcraft();
            }
            ui.separator();
            let print = egui::Button::new(RichText::new("▶ RIP e imprimir").size(13.5).color(Color32::WHITE).strong())
                .fill(ACCENT)
                .min_size(Vec2::new(0.0, 30.0));
            if ui.add(print).clicked() {
                self.action_print();
            }
            if ui.add(big("⏸ Espera")).clicked() {
                self.action_hold();
            }
            if ui.add(big("▦ Aninhar")).clicked() {
                self.action_nest();
            }
            if ui.add(big("▢ Desaninhar")).clicked() {
                self.action_unnest();
            }
            if ui.add(big("✖ Excluir")).clicked() {
                self.confirm_delete = !self.selected.is_empty();
            }
            ui.separator();
            if ui.add(big("Σ Estimativa")).clicked() {
                self.show_estimate = true;
            }
            if ui.add(big("⚙ Configuração")).clicked() {
                self.show_setup = true;
            }
        });
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Configurações");
        let printing = self.jobs.iter().any(|j| j.status == Status::Printing);
        let ripping = self.jobs.iter().any(|j| j.status == Status::Ripping);
        egui::Frame::none()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, ACCENT))
            .rounding(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, p) = ui.allocate_painter(Vec2::new(30.0, 26.0), Sense::hover());
                    draw_printer_icon(&p, r.rect);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&self.setup.name).strong());
                        ui.label(RichText::new(&self.setup.printer_model).small().color(Color32::DARK_GRAY));
                    });
                });
                let (txt, col) = if printing {
                    ("Imprimindo", Color32::from_rgb(0, 140, 60))
                } else if ripping {
                    ("Processando (RIP)", Color32::from_rgb(70, 70, 200))
                } else {
                    ("Pronta", Color32::from_rgb(0, 140, 60))
                };
                status_dot(ui, col, txt);
                ui.label(RichText::new(format!("Mídia: {:.0} mm de largura", self.setup.media_width_mm)).small());
                let dest = match &self.setup.output {
                    Output::WindowsPrinter(p) => format!("Saída: {p}"),
                    Output::Folder(_) => "Saída: pasta de arquivos".to_string(),
                };
                ui.label(RichText::new(dest).small());
            });
        ui.add_space(4.0);
        egui::Frame::none().fill(Color32::WHITE).rounding(4.0).inner_margin(8.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, p) = ui.allocate_painter(Vec2::new(30.0, 22.0), Sense::hover());
                draw_cutter_icon(&p, r.rect);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Plotter de recorte").strong());
                    ui.label(RichText::new("não configurado").small().color(Color32::GRAY));
                });
            });
        });

        ui.add_space(10.0);
        section_title(ui, "Tinta virtual");
        for c in 0..4 {
            let ml = self.setup.ink_ml[c];
            ui.label(RichText::new(INK_NAMES[c]).small());
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, 3.0, Color32::WHITE);
            let frac = (ml / INK_BOTTLE_ML).clamp(0.0, 1.0);
            let mut fill = rect;
            fill.set_width(rect.width() * frac);
            p.rect_filled(fill, 3.0, rgb(INK_COLORS[c]));
            p.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::from_gray(170)));
            let txt_col = if frac > 0.55 && c != 2 { Color32::WHITE } else { Color32::BLACK };
            p.text(rect.center(), Align2::CENTER_CENTER, format!("{ml:.0} ml"), FontId::proportional(11.0), txt_col);
            resp.context_menu(|ui| {
                if ui.button(format!("Diminuir {VIRTUAL_INK_DECREMENT_ML:.0} ml")).clicked() {
                    self.confirm_ink = Some(c);
                    ui.close_menu();
                }
                if ui.button("Encher").clicked() {
                    self.setup.ink_ml[c] = INK_BOTTLE_ML;
                    self.dirty = true;
                    ui.close_menu();
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.small_button(format!("−{VIRTUAL_INK_DECREMENT_ML:.0} ml")).clicked() {
                self.confirm_ink = Some(4);
            }
            if ui.small_button("Encher tudo").clicked() {
                self.setup.ink_ml = [INK_BOTTLE_ML; 4];
                self.dirty = true;
            }
        });

        ui.add_space(10.0);
        section_title(ui, "VectorCraft");
        if self.vc_found {
            status_dot(ui, Color32::from_rgb(0, 140, 60), "conectado");
        } else {
            status_dot(ui, Color32::RED, "não encontrado");
        }
        ui.label(
            RichText::new("Arraste arquivos .vectorcraft, .svg, .pdf ou imagens para a janela. Ao salvar no VectorCraft, a prévia é atualizada sozinha.")
                .small()
                .color(Color32::DARK_GRAY),
        );
    }

    fn queue_table(&mut self, ui: &mut egui::Ui, title: &str, queue: Queue, color: Color32, height: f32) {
        let ids: Vec<u64> = self.jobs.iter().filter(|j| j.queue == queue).map(|j| j.id).collect();
        let width = ui.available_width();
        let (bar, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(bar, egui::Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }, HEADER);
        p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(6.0, bar.height())), 0.0, color);
        p.text(bar.left_center() + Vec2::new(14.0, 0.0), Align2::LEFT_CENTER, title, FontId::proportional(13.0), Color32::WHITE);
        p.text(
            bar.right_center() - Vec2::new(10.0, 0.0),
            Align2::RIGHT_CENTER,
            format!("{} trabalho(s)", ids.len()),
            FontId::proportional(11.5),
            Color32::from_gray(200),
        );
        let cols = [0.31f32, 0.24, 0.17, 0.10, 0.10, 0.08];
        let names = ["Nome", "Status", "Configuração", "Largura", "Altura", "Cópias"];
        let (hdr, _) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(hdr, 0.0, Color32::from_rgb(222, 225, 232));
        let mut x = hdr.left();
        for (i, n) in names.iter().enumerate() {
            p.text(Pos2::new(x + STATUS_TEXT_PADDING, hdr.center().y), Align2::LEFT_CENTER, *n, FontId::proportional(11.5), Color32::from_gray(60));
            x += cols[i] * width;
            p.line_segment([Pos2::new(x, hdr.top() + 2.0), Pos2::new(x, hdr.bottom() - 2.0)], Stroke::new(1.0_f32, Color32::from_gray(190)));
        }
        let body = Rect::from_min_size(ui.cursor().min, Vec2::new(width, height));
        ui.painter().rect_filled(body, 0.0, color);
        egui::ScrollArea::vertical()
            .id_salt(title)
            .max_height(height)
            .min_scrolled_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if ids.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("   (vazia)").color(Color32::from_gray(140)).italics());
                }
                for id in ids {
                    self.queue_row(ui, id, &cols, width);
                }
            });
        ui.add_space(4.0);
    }

    fn queue_row(&mut self, ui: &mut egui::Ui, id: u64, cols: &[f32; 6], width: f32) {
        let Some(j) = self.job(id).cloned() else { return };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click());
        let selected = self.selected.contains(&id);
        let p = ui.painter().clone();
        let text_col = if selected { Color32::WHITE } else { Color32::BLACK };
        if selected {
            p.rect_filled(rect, 0.0, ACCENT);
        } else if resp.hovered() {
            p.rect_filled(rect, 0.0, Color32::from_black_alpha(14));
        }
        p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0_f32, Color32::from_black_alpha(20)));
        let (w, h) = j.out_size_mm();
        let name = match j.nest {
            Some(n) => format!("{NEST_PREFIX}{} · {}", self.nest_number(n), j.name),
            None => j.name.clone(),
        };
        let cells = [
            name,
            j.status.label(),
            self.setup.name.clone(),
            format!("{w:.0} mm"),
            format!("{h:.0} mm"),
            format!("{}", j.settings.copies),
        ];
        let mut x = rect.left();
        for (i, c) in cells.iter().enumerate() {
            let cw = cols[i] * width;
            let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(cw, rect.height()));
            x += cw;
            if i == 1 {
                if let Some(pct) = j.progress() {
                    // the status column draws a progress bar, as the original does
                    let bar = cell.shrink2(Vec2::new(3.0, 3.0));
                    p.rect_filled(bar, 0.0, Color32::WHITE);
                    p.rect_stroke(bar, 0.0, Stroke::new(1.0_f32, Color32::from_gray(150)));
                    let inner = bar.shrink(1.0);
                    let mut fill = inner;
                    fill.set_width(inner.width() * pct / PROGRESS_MAX);
                    p.rect_filled(fill, 0.0, rgb(PROGRESS_COLOR));
                    let label = match j.status {
                        Status::Printing => format!("Imprimindo {pct:.0}%"),
                        Status::Importing => "Lendo arquivo...".to_string(),
                        _ => format!("RIP {pct:.0}%"),
                    };
                    p.text(Pos2::new(bar.left() + STATUS_TEXT_PADDING, bar.center().y), Align2::LEFT_CENTER, label, FontId::proportional(11.5), Color32::BLACK);
                    continue;
                }
            }
            let col = if i == 1 && matches!(j.status, Status::Error(_)) && !selected {
                Color32::from_rgb(190, 0, 0)
            } else {
                text_col
            };
            let clipped = p.with_clip_rect(cell.shrink2(Vec2::new(2.0, 0.0)));
            clipped.text(Pos2::new(cell.left() + STATUS_TEXT_PADDING, rect.center().y), Align2::LEFT_CENTER, c, FontId::proportional(12.5), col);
        }
        if resp.clicked() {
            let multi = ui.input(|i| i.modifiers.command || i.modifiers.shift);
            if multi {
                if selected {
                    self.selected.retain(|s| *s != id);
                } else {
                    self.selected.push(id);
                }
            } else {
                self.selected = vec![id];
            }
        }
        if resp.double_clicked() {
            self.selected = vec![id];
            self.open_in_vectorcraft();
        }
        if resp.secondary_clicked() && !selected {
            self.selected = vec![id];
        }
        let resp = if let Status::Error(e) = &j.status { resp.on_hover_text(e.as_str()) } else { resp };
        resp.context_menu(|ui| {
            if ui.button("RIP e imprimir").clicked() {
                self.action_print();
                ui.close_menu();
            }
            if ui.button("Abrir no VectorCraft").clicked() {
                self.open_in_vectorcraft();
                ui.close_menu();
            }
            if ui.button("Mover para espera").clicked() {
                self.action_hold();
                ui.close_menu();
            }
            if ui.button("Aninhar trabalhos").clicked() {
                self.action_nest();
                ui.close_menu();
            }
            if ui.button("Desaninhar trabalhos").clicked() {
                self.action_unnest();
                ui.close_menu();
            }
            if ui.button("Estimativa do trabalho...").clicked() {
                self.show_estimate = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Excluir").clicked() {
                self.confirm_delete = true;
                ui.close_menu();
            }
        });
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Propriedades do trabalho");
        let Some(id) = self.selected.first().copied() else {
            ui.label(RichText::new("Nenhum trabalho selecionado.").color(Color32::GRAY));
            return;
        };
        let Some(j) = self.job(id).cloned() else { return };
        ui.label(RichText::new(&j.name).strong());
        ui.label(RichText::new(format!("Arte: {:.0} × {:.0} mm", j.art_w_mm, j.art_h_mm)).small().color(Color32::DARK_GRAY));
        let mut s = j.settings.clone();
        let media_names: Vec<String> = self.setup.media.iter().map(|m| m.name.clone()).collect();
        ui.add_enabled_ui(!j.status.running(), |ui| {
            egui::Grid::new("props").num_columns(2).spacing([8.0, 5.0]).show(ui, |ui| {
                combo(ui, "Mídia", &mut s.media, &media_names);
                let res: Vec<String> = RESOLUTIONS.iter().map(|r| r.0.to_string()).collect();
                combo(ui, "Resolução (dpi)", &mut s.resolution, &res);
                combo(ui, "Modo de cor", &mut s.color_mode, &strs(&COLOR_MODES));
                let passes: Vec<String> = PASSES.iter().map(|p| format!("{p} passadas")).collect();
                combo(ui, "Passo", &mut s.passes, &passes);
                combo(ui, "Velocidade", &mut s.speed, &strs(&SPEEDS));
                combo(ui, "Direção", &mut s.direction, &strs(&DIRECTIONS));
                combo(ui, "Tamanho do ponto", &mut s.dot, &strs(&DOT_SIZES));
                let levels: Vec<String> = INK_LEVELS.iter().map(|l| l.0.to_string()).collect();
                combo(ui, "Nível de tinta", &mut s.ink_level, &levels);
                combo(ui, "Tinta branca", &mut s.white, &strs(&WHITE_MODES));
                ui.label("Escala");
                ui.add(egui::DragValue::new(&mut s.scale_pct).range(1.0..=1000.0).suffix(" %").speed(1.0));
                ui.end_row();
                ui.label("Cópias");
                ui.add(egui::DragValue::new(&mut s.copies).range(1..=999));
                ui.end_row();
                let rots: Vec<String> = ROTATIONS.iter().map(|r| format!("{r}°")).collect();
                combo(ui, "Girar", &mut s.rotate, &rots);
                ui.label("Espelhar");
                ui.checkbox(&mut s.mirror, "para o verso");
                ui.end_row();
            });
        });
        if s != j.settings {
            let nest = j.nest;
            for x in &mut self.jobs {
                if x.id == id {
                    x.settings = s.clone();
                } else if nest.is_some() && x.nest == nest {
                    // nested jobs keep sharing media, mode, resolution and colours
                    x.settings = JobSettings {
                        copies: x.settings.copies,
                        scale_pct: x.settings.scale_pct,
                        rotate: x.settings.rotate,
                        mirror: x.settings.mirror,
                        ..s.clone()
                    };
                } else {
                    continue;
                }
                if x.queue == Queue::Print && x.status == Status::Ripped {
                    x.queue = Queue::Hold;
                    x.status = Status::Waiting;
                    x.ripped.clear();
                }
            }
            self.dirty = true;
        }
        ui.label(
            RichText::new(format!(
                "Limite de tinta: {MAX_INK_VOLUME_START:.0}% a {MAX_INK_VOLUME_END:.0}%, inclinação {MAX_SLOPE:.0}"
            ))
            .small()
            .color(Color32::DARK_GRAY),
        );
        ui.add_space(8.0);
        section_title(ui, "Estimativa");
        let unit = self.units(&[id]).into_iter().next().unwrap_or_default();
        let jobs: Vec<Job> = unit.iter().filter_map(|i| self.job(*i).cloned()).collect();
        let refs: Vec<&Job> = jobs.iter().collect();
        let e = estimate(&refs, &self.setup);
        estimate_grid(ui, "est_side", &e);
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let busy = self.jobs.iter().filter(|j| j.status.running()).count();
            ui.label(if busy > 0 { format!("{busy} trabalho(s) em andamento") } else { "Pronto".into() });
            ui.separator();
            ui.label("Threads de RIP: 1");
            ui.separator();
            ui.label(format!("Na fila: {}", self.jobs.iter().filter(|j| j.queue != Queue::Done).count()));
            ui.separator();
            ui.label(format!("Impressos: {}", self.jobs.iter().filter(|j| j.queue == Queue::Done).count()));
        });
    }
}
