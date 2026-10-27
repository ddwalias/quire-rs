use eframe::egui;
use quire_core::{Calculation, MathError, RegionContent, Worksheet};
use std::path::PathBuf;

#[derive(Clone)]
enum Action {
    New,
    Open(PathBuf),
    Close,
    Overwrite(PathBuf),
}

/// Application services and UI share this state; calculation and I/O stay in
/// the portable core. The same UI can run in an egui test context without a GPU.
pub struct QuireApp {
    worksheet: Worksheet,
    saved: Worksheet,
    current_path: Option<PathBuf>,
    path_input: String,
    results: Vec<Option<Result<Calculation, MathError>>>,
    status: String,
    status_is_error: bool,
    pending: Option<Action>,
    focus_region: Option<u64>,
    allow_close: bool,
    editor_generation: u64,
    #[cfg(test)]
    targets: std::collections::HashMap<String, egui::Rect>,
}

impl Default for QuireApp {
    fn default() -> Self {
        let worksheet = Worksheet::default();
        Self {
            saved: worksheet.clone(),
            worksheet,
            current_path: None,
            path_input: "worksheet.quire.json".into(),
            results: Vec::new(),
            status: "Ready. Save worksheets as .quire.json.".into(),
            status_is_error: false,
            pending: None,
            focus_region: None,
            allow_close: false,
            editor_generation: 0,
            #[cfg(test)]
            targets: Default::default(),
        }
    }
}

// Record real widget locations for tests that send pointer and keyboard events.
macro_rules! target {
    ($app:expr, $name:expr, $response:expr) => {
        let _ = &$response;
        #[cfg(test)]
        {
            $app.targets.insert($name.into(), $response.rect);
        }
    };
}

impl QuireApp {
    fn dirty(&self) -> bool {
        self.worksheet != self.saved
    }

    fn report(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = message.into();
        self.status_is_error = is_error;
    }

    fn request(&mut self, action: Action, ctx: &egui::Context) {
        if self.dirty() {
            self.pending = Some(action);
        } else {
            self.perform(action, ctx);
        }
    }

    fn perform(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::New => {
                self.editor_generation = self.editor_generation.wrapping_add(1);
                self.worksheet = Worksheet::default();
                self.saved = self.worksheet.clone();
                self.current_path = None;
                self.results.clear();
                self.report("New worksheet", false);
            }
            Action::Open(path) => match Worksheet::open(&path) {
                Ok(sheet) => {
                    self.editor_generation = self.editor_generation.wrapping_add(1);
                    self.worksheet = sheet;
                    self.saved = self.worksheet.clone();
                    self.current_path = Some(path);
                    self.results = self.worksheet.calculate();
                    self.report("Opened worksheet", false);
                }
                Err(e) => self.report(e.to_string(), true),
            },
            Action::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Overwrite(path) => {
                self.save(path);
            }
        }
    }

    fn save(&mut self, path: PathBuf) -> bool {
        match self.worksheet.save(&path) {
            Ok(()) => {
                self.saved = self.worksheet.clone();
                self.current_path = Some(path);
                self.report("Saved worksheet", false);
                true
            }
            Err(e) => {
                self.report(e.to_string(), true);
                false
            }
        }
    }

    fn request_save(&mut self) {
        let path = PathBuf::from(self.path_input.trim());
        if path.exists() && self.current_path.as_ref() != Some(&path) {
            self.pending = Some(Action::Overwrite(path));
        } else {
            self.save(path);
        }
    }

    /// Render the production UI. Tests drive this exact function with raw
    /// keyboard/pointer events, including the file controls and confirmations.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Action::Close);
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Quire worksheet");
            ui.add_enabled_ui(self.pending.is_none(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Title:");
                let response = ui.add(egui::TextEdit::singleline(&mut self.worksheet.title).id_salt(("title", self.editor_generation)));
                target!(self, "title", response);
                ui.label(if self.dirty() { "Unsaved changes" } else { "Saved / unchanged" });
            });
            ui.horizontal(|ui| {
                ui.label("File:");
                let response = ui.add(egui::TextEdit::singleline(&mut self.path_input).desired_width(400.0));
                target!(self, "path", response);
            });
            ui.horizontal(|ui| {
                let response = ui.button("New"); target!(self, "new", response);
                if response.clicked() { self.request(Action::New, &ctx); }
                let response = ui.button("Open"); target!(self, "open", response);
                if response.clicked() { self.request(Action::Open(PathBuf::from(self.path_input.trim())), &ctx); }
                let response = ui.button("Save"); target!(self, "save", response);
                if response.clicked() { self.request_save(); }
                if ui.button("Recalculate").clicked() { self.results = self.worksheet.calculate(); }
            });
            });
            let color = if self.status_is_error { ui.visuals().error_fg_color } else { ui.visuals().text_color() };
            ui.colored_label(color, &self.status);
            ui.collapsing("Math syntax and limits", |ui| {
                ui.label("Define: a := 25 cm   |   Evaluate: a + 1 m -> cm");
                ui.label("Operators: + - * / ^ and parentheses. Powers: integers -16..16.");
                ui.label("Units: m, cm, mm, km. Use * between variables. Names are case-sensitive ASCII.");
                ui.label("Calculates from top to bottom as you edit. Mathcad files are unsupported; see README for limited reference checks.");
            });

            if self.pending.is_some() {
                ui.separator();
                if let Some(Action::Overwrite(path)) = &self.pending {
                    ui.label(format!("Replace the existing file at {}?", path.display()));
                    ui.horizontal(|ui| {
                        let response = ui.button("Replace file"); target!(self, "confirm", response);
                        if response.clicked() { let action = self.pending.take().unwrap(); self.perform(action, &ctx); }
                        let response = ui.button("Cancel"); target!(self, "cancel", response);
                        if response.clicked() { self.pending = None; }
                    });
                } else {
                    ui.label("Unsaved changes. Choose Keep editing to save first, or discard them to continue.");
                    ui.horizontal(|ui| {
                        let response = ui.button("Discard changes"); target!(self, "discard", response);
                        if response.clicked() { let action = self.pending.take().unwrap(); self.perform(action, &ctx); }
                        let response = ui.button("Keep editing"); target!(self, "cancel", response);
                        if response.clicked() { self.pending = None; }
                    });
                }
            }
            ui.separator();
            let mut changed = false;
            let blocked = self.pending.is_some();
            ui.add_enabled_ui(!blocked, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.button("Add text"); target!(self, "add_text", response);
                    if response.clicked() {
                        self.focus_region = Some(self.worksheet.add_region(RegionContent::Text(String::new())));
                        changed = true;
                    }
                    let response = ui.button("Add math"); target!(self, "add_math", response);
                    if response.clicked() {
                        self.focus_region = Some(self.worksheet.add_region(RegionContent::Math(String::new())));
                        changed = true;
                    }
                });
                let mut movement = None;
                let mut deletion = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, region) in self.worksheet.regions.iter_mut().enumerate() {
                        ui.push_id((self.editor_generation, region.id), |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} · {}", index + 1, if matches!(region.content, RegionContent::Text(_)) { "Text" } else { "Math" }));
                                    let response = ui.add_enabled(index > 0, egui::Button::new("Move up"));
                                    target!(self, format!("move_{}", region.id), response);
                                    if response.clicked() { movement = Some(index); }
                                    let response = ui.button("Delete");
                                    target!(self, format!("delete_{}", region.id), response);
                                    if response.clicked() { deletion = Some(index); }
                                });
                                let is_math = matches!(region.content, RegionContent::Math(_));
                                let edit = if is_math { egui::TextEdit::singleline(region.content.source_mut()).font(egui::TextStyle::Monospace) }
                                    else { egui::TextEdit::multiline(region.content.source_mut()).desired_rows(2) };
                                let response = ui.add(edit.id_salt("source").desired_width(f32::INFINITY));
                                #[cfg(test)]
                                { self.targets.insert(format!("region_{}", region.id), response.rect); }
                                if self.focus_region == Some(region.id) { response.request_focus(); self.focus_region = None; }
                                changed |= response.changed();
                                if let Some(Some(result)) = self.results.get(index) {
                                    match result {
                                        Ok(value) => { ui.label(format!("= {value}")); }
                                        Err(e) => { ui.colored_label(ui.visuals().error_fg_color, e.to_string()); }
                                    }
                                }
                            });
                        });
                    }
                });
                if let Some(index) = deletion {
                    self.worksheet.regions.remove(index);
                    // Deleted IDs may be reused. Start fresh editor state so
                    // a new region cannot inherit deleted text's undo history.
                    self.editor_generation = self.editor_generation.wrapping_add(1);
                    changed = true;
                }
                else if let Some(index) = movement { self.worksheet.regions.swap(index - 1, index); changed = true; }
            });
            if changed { self.results = self.worksheet.calculate(); ctx.request_repaint(); }
        });
    }
}

impl eframe::App for QuireApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

    struct Driver {
        app: QuireApp,
        ctx: egui::Context,
        time: f64,
    }

    impl Driver {
        fn new() -> Self {
            let mut driver = Self {
                app: QuireApp::default(),
                ctx: egui::Context::default(),
                time: 0.0,
            };
            driver.frame(Vec::new());
            driver
        }
        fn frame(&mut self, events: Vec<Event>) {
            self.time += 0.1;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 1200.0))),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            let mut output = self.ctx.run_ui(input, |ui| self.app.show(ui));
            // This harness has no renderer; consume texture changes explicitly.
            output.textures_delta.clear();
        }
        fn click(&mut self, name: &str) {
            let position = self
                .app
                .targets
                .get(name)
                .unwrap_or_else(|| panic!("Missing UI target {name}"))
                .center();
            self.frame(vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ]);
            self.frame(vec![Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }]);
            self.frame(Vec::new());
        }
        fn replace(&mut self, name: &str, text: &str) {
            self.click(name);
            let modifiers = Modifiers {
                command: true,
                ctrl: !cfg!(target_os = "macos"),
                mac_cmd: cfg!(target_os = "macos"),
                ..Default::default()
            };
            self.frame(vec![Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }]);
            self.frame(vec![Event::Text(text.into())]);
            self.frame(Vec::new());
        }
        fn value(&self, index: usize) -> &Calculation {
            self.app.results[index].as_ref().unwrap().as_ref().unwrap()
        }
    }

    #[test]
    fn ui_add_edit_recalculate_save_new_and_reopen_workflow() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow.quire.json");
        let mut driver = Driver::new();
        driver.replace("title", "Engineering lengths");
        driver.click("add_text");
        driver.replace("region_1", "Length check – notes 🦀");
        for (id, source) in [(2, "a := 25 cm"), (3, "b := a + 1 m"), (4, "b -> cm")] {
            driver.click("add_math");
            driver.replace(&format!("region_{id}"), source);
        }
        assert_eq!(driver.value(3).display_value, 125.0);
        assert_eq!(driver.value(3).quantity.length_power, 1);
        driver.replace("region_2", "a := 50 cm");
        assert_eq!(driver.value(3).display_value, 150.0);
        driver.replace("path", path.to_str().unwrap());
        driver.click("save");
        assert!(!driver.app.status_is_error, "{}", driver.app.status);
        assert!(!driver.app.dirty());
        let saved = driver.app.worksheet.clone();
        assert_eq!(saved.title, "Engineering lengths");
        assert_eq!(saved.regions[0].content.source(), "Length check – notes 🦀");
        driver.click("new");
        assert!(driver.app.worksheet.regions.is_empty());
        driver.click("open");
        assert_eq!(driver.app.worksheet, saved);
        assert_eq!(driver.value(3).display_value, 150.0);
        driver.replace("region_2", "a := 1 m + 2");
        assert!(driver.app.results[3].as_ref().unwrap().is_err());
        driver.replace("region_2", "a := 75 cm");
        assert_eq!(driver.value(3).display_value, 175.0);
        driver.click("save");
        driver.click("new");
        driver.click("open");
        assert_eq!(driver.value(3).display_value, 175.0);
    }

    #[test]
    fn ui_failed_open_save_and_discard_confirmation_protect_work() {
        let directory = tempfile::tempdir().unwrap();
        let mut driver = Driver::new();
        driver.click("add_math");
        driver.replace("region_1", "x := 5");
        driver.click("new");
        assert!(driver.app.pending.is_some());
        driver.click("cancel");
        assert_eq!(driver.value(0).quantity.si_value, 5.0);
        let missing = directory.path().join("missing/file.quire.json");
        driver.replace("path", missing.to_str().unwrap());
        driver.click("save");
        assert!(driver.app.status_is_error);
        assert!(driver.app.dirty());
        driver.click("open");
        driver.click("discard");
        assert!(driver.app.status_is_error);
        assert!(driver.app.dirty());
        assert_eq!(driver.value(0).quantity.si_value, 5.0);
        driver.click("new");
        driver.click("discard");
        assert!(driver.app.worksheet.regions.is_empty());
    }

    #[test]
    fn ui_existing_save_as_requires_explicit_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing.quire.json");
        std::fs::write(&path, "original").unwrap();
        let mut driver = Driver::new();
        driver.click("add_text");
        driver.replace("region_1", "replacement");
        driver.replace("path", path.to_str().unwrap());
        driver.click("save");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
        driver.click("cancel");
        assert!(driver.app.dirty());
        driver.click("save");
        driver.click("confirm");
        assert!(!driver.app.dirty());
        assert_eq!(Worksheet::open(path).unwrap(), driver.app.worksheet);
    }

    #[test]
    fn ui_moving_and_deleting_regions_recalculates_in_new_order() {
        let mut driver = Driver::new();
        driver.click("add_math");
        driver.replace("region_1", "a := 3");
        driver.click("add_math");
        driver.replace("region_2", "a * 2");
        assert_eq!(driver.value(1).quantity.si_value, 6.0);
        driver.click("move_2");
        assert_eq!(driver.app.worksheet.regions[0].id, 2);
        assert!(driver.app.results[0].as_ref().unwrap().is_err());
        driver.click("move_1");
        assert_eq!(driver.value(1).quantity.si_value, 6.0);
        driver.click("delete_1");
        assert_eq!(driver.app.worksheet.regions.len(), 1);
        assert!(driver.app.results[0].as_ref().unwrap().is_err());
    }

    #[test]
    fn ui_window_close_requires_a_decision_when_unsaved() {
        let mut driver = Driver::new();
        driver.click("add_text");
        driver.replace("region_1", "Work to retain");
        let mut input = RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = driver.ctx.run_ui(input, |ui| driver.app.show(ui));
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::CancelClose)
        );
        output.textures_delta.clear();
        assert!(matches!(driver.app.pending, Some(Action::Close)));
        driver.frame(Vec::new());
        driver.click("cancel");
        assert!(!driver.app.allow_close);
        assert!(driver.app.dirty());
    }

    #[test]
    fn ui_new_document_and_deleted_regions_start_with_fresh_editor_history() {
        let mut driver = Driver::new();
        driver.click("add_text");
        driver.replace("region_1", "Old document text");
        driver.frame(Vec::new());
        let original_generation = driver.app.editor_generation;
        driver.click("new");
        driver.click("discard");
        assert_ne!(driver.app.editor_generation, original_generation);
        driver.click("add_text");
        driver.replace("region_1", "Current document text");
        let document_generation = driver.app.editor_generation;
        driver.click("delete_1");
        assert_ne!(driver.app.editor_generation, document_generation);
        driver.click("add_text");
        driver.click("region_1");
        driver.frame(vec![Event::Key {
            key: egui::Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers {
                command: true,
                ctrl: !cfg!(target_os = "macos"),
                mac_cmd: cfg!(target_os = "macos"),
                ..Default::default()
            },
        }]);
        assert_eq!(driver.app.worksheet.regions[0].content.source(), "");
    }
}
