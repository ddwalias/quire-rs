use crate::{Action, FileMode, QuireApp, RibbonTab, equation};
use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;

impl QuireApp {
    pub(crate) fn chrome(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let blocked = self.pending.is_some() || self.file_mode.is_some();
        let math_selected = self.worksheet.regions.iter().any(|region| {
            Some(region.id) == self.selected
                && matches!(region.content, quire_core::RegionContent::Math(_))
        });
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(240, 244, 248))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let pages = self
                        .worksheet
                        .regions
                        .iter()
                        .filter_map(|region| region.layout)
                        .map(|layout| layout.page + 1)
                        .max()
                        .unwrap_or(1);
                    ui.label(
                        RichText::new(format!(
                            "{pages} {}",
                            if pages == 1 { "page" } else { "pages" }
                        ))
                        .size(11.0),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new("● AUTO")
                            .size(11.0)
                            .color(Color32::from_rgb(49, 128, 85)),
                    );
                    let color = if self.status_is_error {
                        Color32::from_rgb(172, 45, 45)
                    } else {
                        Color32::from_rgb(94, 107, 123)
                    };
                    ui.add(
                        egui::Label::new(RichText::new(&self.status).size(11.0).color(color))
                            .truncate(),
                    )
                    .on_hover_text(&self.status);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{}%", (self.zoom * 100.0).round())).size(11.0),
                        );
                        let response =
                            ui.add(egui::Slider::new(&mut self.zoom, 0.65..=1.5).show_value(false));
                        target!(self, "zoom", response);
                    });
                });
            });
        egui::Panel::top("ribbon")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(241, 246, 251))
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ui, |ui| {
                ui.add_enabled_ui(!blocked, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Q").size(23.0).strong().color(equation::BLUE));
                        ui.label(RichText::new("Quire").size(12.0).strong());
                        ui.add_space(12.0);
                        let response = ui.small_button("New");
                        target!(self, "new", response);
                        if response.clicked() {
                            self.request(Action::New, &ctx);
                        }
                        let response = ui.small_button("Open…");
                        target!(self, "open", response);
                        if response.clicked() {
                            self.file_mode = Some(FileMode::Open);
                        }
                        let response = ui.small_button("Save");
                        target!(self, "save", response);
                        if response.clicked() {
                            self.begin_save();
                        }
                        let response = ui.small_button("Save as…");
                        target!(self, "save_as", response);
                        if response.clicked() {
                            self.file_mode = Some(FileMode::Save);
                        }
                    });
                    ui.horizontal(|ui| {
                        for (tab, label) in [
                            (RibbonTab::Math, "Math"),
                            (RibbonTab::Document, "Document"),
                            (RibbonTab::Calculation, "Calculation"),
                        ] {
                            let response = ui
                                .selectable_label(self.tab == tab, RichText::new(label).size(13.0));
                            target!(self, format!("tab_{label}"), response);
                            if response.clicked() {
                                self.tab = tab;
                            }
                        }
                        ui.add_space(16.0);
                        let response = ui.small_button("Example worksheet");
                        target!(self, "example", response);
                        if response.clicked() {
                            self.request(Action::Example, &ctx);
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| match self.tab {
                        RibbonTab::Math => {
                            ribbon_group(ui, "Regions", |ui| {
                                let response = ribbon_button(ui, "x+y", "Math");
                                target!(self, "add_math", response);
                                if response.clicked() {
                                    self.add_region(false);
                                }
                                let response = ribbon_button(ui, "T", "Text");
                                target!(self, "add_text", response);
                                if response.clicked() {
                                    self.add_region(true);
                                }
                                ui.vertical(|ui| {
                                    let response = ui.add_enabled(
                                        self.selected.is_some(),
                                        egui::Button::new("Edit").small(),
                                    );
                                    target!(self, "edit", response);
                                    if response.clicked() {
                                        self.edit_selected();
                                    }
                                    let response = ui.add_enabled(
                                        self.selected.is_some(),
                                        egui::Button::new("Move up").small(),
                                    );
                                    target!(self, "move", response);
                                    if response.clicked() {
                                        self.move_before_selected();
                                    }
                                    let response = ui.add_enabled(
                                        self.selected.is_some(),
                                        egui::Button::new("Delete").small(),
                                    );
                                    target!(self, "delete", response);
                                    if response.clicked() {
                                        self.delete_selected();
                                    }
                                });
                            });
                            ui.separator();
                            ribbon_group(ui, "Operators", |ui| {
                                for (icon, label, insertion) in [
                                    ("a/b", "Fraction", "/"),
                                    ("x²", "Power", "^"),
                                    ("a·b", "Multiply", "*"),
                                    (":=", "Define", ":="),
                                    ("→", "Convert", " -> "),
                                ] {
                                    let response = ui
                                        .add_enabled_ui(math_selected, |ui| {
                                            ribbon_button(ui, icon, label)
                                        })
                                        .inner;
                                    target!(self, format!("operator_{label}"), response);
                                    if response.clicked() {
                                        self.insert_operator(&ctx, insertion);
                                    }
                                }
                            });
                            ui.separator();
                            ribbon_group(ui, "Units", |ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Unit system: SI").size(12.0));
                                    ui.horizontal(|ui| {
                                        for unit in ["m", "cm", "mm", "km"] {
                                            let response = ui.add_enabled(
                                                math_selected,
                                                egui::Button::new(unit).small(),
                                            );
                                            if response.clicked() {
                                                self.insert_operator(&ctx, unit);
                                            }
                                        }
                                    });
                                });
                            });
                        }
                        RibbonTab::Document => {
                            ribbon_group(ui, "View", |ui| {
                                ui.checkbox(&mut self.show_grid, "Show grid");
                                if ui.button("Actual size").clicked() {
                                    self.zoom = 1.0;
                                }
                            });
                            ui.separator();
                            ribbon_group(ui, "Worksheet", |ui| {
                                ui.label(
                                    RichText::new("A4 portrait · positioned regions").size(12.0),
                                );
                                ui.label(
                                    RichText::new("Drag a selected region's handle to move it.")
                                        .size(12.0),
                                );
                            });
                        }
                        RibbonTab::Calculation => {
                            ribbon_group(ui, "Controls", |ui| {
                                let response = ui.button("Calculate worksheet");
                                target!(self, "recalculate", response);
                                if response.clicked() {
                                    self.recalculate();
                                    self.report("Worksheet recalculated", false);
                                }
                                ui.label(RichText::new("Automatic calculation is on").size(12.0));
                            });
                            ui.separator();
                            ribbon_group(ui, "Worksheet order", |ui| {
                                ui.label(
                                    RichText::new("Top to bottom, then left to right.").size(12.0),
                                );
                                ui.label(
                                    RichText::new("Definitions apply to later regions.").size(12.0),
                                );
                            });
                        }
                    });
                });
            });
        egui::Panel::top("document-tab")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(222, 233, 244))
                    .inner_margin(egui::Margin::symmetric(18, 4)),
            )
            .show(ui, |ui| {
                ui.add_enabled_ui(!blocked, |ui| {
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.worksheet.title)
                                .id_salt(("title", self.editor_generation))
                                .desired_width(280.0)
                                .frame(egui::Frame::new())
                                .font(egui::TextStyle::Body),
                        );
                        target!(self, "title", response);
                        if self.dirty() {
                            ui.label(RichText::new("*").color(equation::BLUE));
                        }
                    });
                });
            });
    }

    fn insert_operator(&mut self, ctx: &egui::Context, symbol: &str) {
        if let Some(id) = self.selected
            && self.worksheet.regions.iter().any(|region| {
                region.id == id && matches!(region.content, quire_core::RegionContent::Math(_))
            })
        {
            self.editing = Some(id);
            self.focus_region = Some(id);
            ctx.memory_mut(|memory| memory.request_focus(self.source_id(id)));
            ctx.input_mut(|input| input.events.push(egui::Event::Text(symbol.into())));
        }
    }

    fn move_before_selected(&mut self) {
        if let Some(index) = self
            .worksheet
            .regions
            .iter()
            .position(|region| Some(region.id) == self.selected)
            .filter(|index| *index > 0)
        {
            let (before, after) = self.worksheet.regions.split_at_mut(index);
            if let (Some(a), Some(b)) = (&mut before[index - 1].layout, &mut after[0].layout) {
                std::mem::swap(&mut a.page, &mut b.page);
                std::mem::swap(&mut a.x, &mut b.x);
                std::mem::swap(&mut a.y, &mut b.y);
                a.width = a.width.min(quire_core::layout::PAGE_WIDTH - 24 - a.x);
                b.width = b.width.min(quire_core::layout::PAGE_WIDTH - 24 - b.x);
            }
            self.worksheet.sort_by_position();
            self.recalculate();
        }
    }

    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(mode) = self.file_mode {
            let response = egui::Modal::new(egui::Id::unique("file-dialog")).show(ctx, |ui| {
                ui.set_width(510.0);
                ui.heading(if mode == FileMode::Open {
                    "Open worksheet"
                } else {
                    "Save worksheet"
                });
                ui.label("Quire worksheet  ·  .quire.json");
                ui.add_space(12.0);
                ui.label("File path");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.path_input).desired_width(f32::INFINITY),
                );
                target!(self, "path", response);
                ui.add_space(8.0);
                if self.status_is_error {
                    ui.colored_label(Color32::from_rgb(172, 45, 45), &self.status);
                }
                ui.horizontal(|ui| {
                    let response = ui.button(if mode == FileMode::Open {
                        "Open worksheet"
                    } else {
                        "Save worksheet"
                    });
                    target!(self, "file_submit", response);
                    if response.clicked() {
                        if mode == FileMode::Open {
                            self.file_mode = None;
                            self.request(Action::Open(PathBuf::from(self.path_input.trim())), ctx);
                        } else {
                            self.request_save();
                        }
                    }
                    let response = ui.button("Cancel");
                    target!(self, "file_cancel", response);
                    if response.clicked() {
                        self.file_mode = None;
                    }
                });
            });
            if response.should_close() {
                self.file_mode = None;
            }
        }
        if self.pending.is_some() {
            let response = egui::Modal::new(egui::Id::unique("confirmation")).show(ctx, |ui| {
                ui.set_width(440.0);
                if let Some(Action::Overwrite(path)) = &self.pending {
                    ui.heading("Replace worksheet?");
                    ui.label(format!("The file at {} already exists.", path.display()));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let response = ui.button("Replace file"); target!(self, "confirm", response);
                        if response.clicked() { let action = self.pending.take().unwrap(); self.perform(action, ctx); }
                        let response = ui.button("Cancel"); target!(self, "cancel", response);
                        if response.clicked() { self.pending = None; }
                    });
                } else {
                    ui.heading("Save your changes?");
                    ui.label("This worksheet has unsaved changes. Keep editing to save first, or discard them to continue.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let response = ui.button("Discard changes"); target!(self, "discard", response);
                        if response.clicked() { let action = self.pending.take().unwrap(); self.perform(action, ctx); }
                        let response = ui.button("Keep editing"); target!(self, "cancel", response);
                        if response.clicked() { self.pending = None; }
                    });
                }
            });
            if response.should_close() {
                self.pending = None;
            }
        }
    }
}

fn ribbon_group(ui: &mut egui::Ui, name: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.set_min_height(64.0);
            contents(ui);
        });
        ui.label(
            RichText::new(name)
                .size(10.0)
                .color(Color32::from_rgb(104, 120, 138)),
        );
    });
}

fn ribbon_button(ui: &mut egui::Ui, symbol: &str, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(59.0, 61.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::accesskit::Role::Button, ui.is_enabled(), label)
    });
    if response.hovered() && ui.is_enabled() {
        ui.painter()
            .rect_filled(rect, 3, Color32::from_rgb(218, 234, 248));
        ui.painter().rect_stroke(
            rect,
            3,
            egui::Stroke::new(1.0, Color32::from_rgb(157, 188, 217)),
            egui::StrokeKind::Inside,
        );
    }
    let ink = if ui.is_enabled() {
        equation::BLUE
    } else {
        Color32::from_gray(166)
    };
    let font = egui::FontId::new(23.0, egui::FontFamily::Name("math".into()));
    let center = egui::pos2(rect.center().x, rect.top() + 20.0);
    if symbol == "a/b" {
        let small = egui::FontId::new(15.0, egui::FontFamily::Name("math".into()));
        ui.painter().text(
            center - egui::vec2(0.0, 9.0),
            egui::Align2::CENTER_CENTER,
            "a",
            small.clone(),
            ink,
        );
        ui.painter().text(
            center + egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            "b",
            small,
            ink,
        );
        ui.painter().line_segment(
            [center - egui::vec2(9.0, 0.0), center + egui::vec2(9.0, 0.0)],
            egui::Stroke::new(1.0, ink),
        );
    } else {
        ui.painter()
            .text(center, egui::Align2::CENTER_CENTER, symbol, font, ink);
    }
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 6.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        egui::FontId::proportional(11.0),
        if ui.is_enabled() {
            equation::INK
        } else {
            Color32::from_gray(166)
        },
    );
    response
}
