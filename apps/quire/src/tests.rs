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
        driver.frame(Vec::new());
        driver
    }
    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 1400.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        self.app.targets.clear();
        let mut output = self.ctx.run_ui(input, |ui| self.app.show(ui));
        output.textures_delta.clear();
    }
    fn click_at(&mut self, position: Pos2) {
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
    fn click(&mut self, name: &str) {
        let position = self
            .app
            .targets
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "Missing UI target {name}; visible: {:?}",
                    self.app.targets.keys()
                )
            })
            .center();
        self.click_at(position);
    }
    fn key(&mut self, key: egui::Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }]);
    }
    fn command() -> Modifiers {
        Modifiers {
            command: true,
            ctrl: !cfg!(target_os = "macos"),
            mac_cmd: cfg!(target_os = "macos"),
            ..Default::default()
        }
    }
    fn replace(&mut self, name: &str, text: &str) {
        let target = if let Some(id) = name.strip_prefix("region_") {
            let id: u64 = id.parse().unwrap();
            if self.app.editing != Some(id) {
                self.click(name);
                self.click("edit");
            }
            format!("source_{id}")
        } else {
            name.to_owned()
        };
        self.click(&target);
        self.key(egui::Key::A, Self::command());
        self.frame(vec![Event::Text(text.into())]);
        self.frame(Vec::new());
    }
    fn finish_edit(&mut self) {
        self.key(egui::Key::Escape, Modifiers::NONE);
    }
    fn value(&self, index: usize) -> &Calculation {
        self.app.results[index].as_ref().unwrap().as_ref().unwrap()
    }
    fn add_math(&mut self, source: &str) -> u64 {
        self.click("add_math");
        let id = self.app.selected.unwrap();
        self.replace(&format!("region_{id}"), source);
        self.finish_edit();
        id
    }
    fn open(&mut self, path: &std::path::Path) {
        self.click("open");
        self.replace("path", path.to_str().unwrap());
        self.click("file_submit");
    }
    fn save_as(&mut self, path: &std::path::Path) {
        self.finish_edit();
        self.click("save_as");
        self.replace("path", path.to_str().unwrap());
        self.click("file_submit");
    }
    fn drag(&mut self, name: &str, delta: Vec2) {
        let from = self.app.targets[name].center();
        self.frame(vec![
            Event::PointerMoved(from),
            Event::PointerButton {
                pos: from,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.frame(vec![Event::PointerMoved(from + delta * 0.5)]);
        self.frame(vec![Event::PointerMoved(from + delta)]);
        self.frame(vec![Event::PointerButton {
            pos: from + delta,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame(Vec::new());
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
    driver.finish_edit();
    driver.add_math("a := 25 cm");
    driver.add_math("b := a + 1 m");
    driver.add_math("b -> cm");
    assert_eq!(driver.value(3).display_value, 125.0);
    driver.replace("region_2", "a := 50 cm");
    assert_eq!(driver.value(3).display_value, 150.0);
    driver.save_as(&path);
    assert!(!driver.app.status_is_error, "{}", driver.app.status);
    assert!(!driver.app.dirty());
    let saved = driver.app.worksheet.clone();
    assert_eq!(saved.title, "Engineering lengths");
    assert_eq!(saved.regions[0].content.source(), "Length check – notes 🦀");
    driver.click("new");
    assert!(driver.app.worksheet.regions.is_empty());
    driver.open(&path);
    assert_eq!(driver.app.worksheet, saved);
    assert_eq!(driver.value(3).display_value, 150.0);
    driver.replace("region_2", "a := 1 m + 2");
    assert!(driver.app.results[3].as_ref().unwrap().is_err());
    driver.replace("region_2", "a := 75 cm");
    assert_eq!(driver.value(3).display_value, 175.0);
    driver.finish_edit();
    driver.click("save");
    driver.click("new");
    driver.open(&path);
    assert_eq!(driver.value(3).display_value, 175.0);
}

#[test]
fn ui_failed_open_save_and_discard_confirmation_protect_work() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing/file.quire.json");
    let mut driver = Driver::new();
    driver.add_math("x := 5");
    driver.click("new");
    assert!(driver.app.pending.is_some());
    driver.click("cancel");
    assert_eq!(driver.value(0).quantity.si_value, 5.0);
    driver.save_as(&missing);
    assert!(driver.app.status_is_error);
    assert!(driver.app.dirty());
    driver.click("file_cancel");
    driver.open(&missing);
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
    driver.add_math("2 + 3");
    driver.save_as(&path);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    driver.click("cancel");
    assert!(driver.app.dirty());
    driver.save_as(&path);
    driver.click("confirm");
    assert!(!driver.app.dirty());
    assert_eq!(Worksheet::open(path).unwrap(), driver.app.worksheet);
}

#[test]
fn ui_moving_and_deleting_regions_recalculates_in_new_order() {
    let mut driver = Driver::new();
    driver.add_math("a := 3");
    driver.add_math("a * 2");
    assert_eq!(driver.value(1).quantity.si_value, 6.0);
    driver.click("region_2");
    driver.click("move");
    assert_eq!(driver.app.worksheet.regions[0].id, 2);
    assert!(driver.app.results[0].as_ref().unwrap().is_err());
    driver.click("region_1");
    driver.click("move");
    assert_eq!(driver.value(1).quantity.si_value, 6.0);
    driver.click("region_1");
    driver.click("delete");
    assert_eq!(driver.app.worksheet.regions.len(), 1);
    assert!(driver.app.results[0].as_ref().unwrap().is_err());
}

#[test]
fn ui_window_close_requires_a_decision_when_unsaved() {
    let mut driver = Driver::new();
    driver.add_math("x := 5");
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
    driver.add_math("42");
    let original_generation = driver.app.editor_generation;
    driver.click("new");
    driver.click("discard");
    assert_ne!(driver.app.editor_generation, original_generation);
    driver.add_math("25");
    let generation = driver.app.editor_generation;
    driver.click("delete");
    assert_ne!(driver.app.editor_generation, generation);
    driver.click("add_math");
    driver.key(egui::Key::Z, Driver::command());
    assert_eq!(driver.app.worksheet.regions[0].content.source(), "");
}

#[test]
fn ui_canvas_placement_drag_zoom_and_layout_save_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("positioned.quire.json");
    let mut driver = Driver::new();
    let page = driver.app.targets["page_0"];
    driver.click_at(page.min + egui::vec2(180.0, 300.0));
    driver.add_math("a := 25 cm");
    assert_eq!(driver.app.worksheet.regions[0].layout.unwrap().x, 180);
    driver.add_math("a + 1 m -> cm");
    assert_eq!(driver.value(1).display_value, 125.0);
    driver.click("region_1");
    let before = driver.app.worksheet.regions[0].layout.unwrap();
    driver.drag("drag_1", egui::vec2(60.0, 190.0));
    let after = driver
        .app
        .worksheet
        .regions
        .iter()
        .find(|region| region.id == 1)
        .unwrap()
        .layout
        .unwrap();
    assert!(
        after.y > before.y + 100,
        "before={before:?}, after={after:?}"
    );
    assert_eq!(after.x, before.x + 60, "before={before:?}, after={after:?}");
    assert_eq!(driver.app.worksheet.regions[0].id, 2);
    assert!(driver.app.results[0].as_ref().unwrap().is_err());
    let document = driver.app.worksheet.clone();
    let slider = driver.app.targets["zoom"];
    driver.click_at(slider.left_center() + egui::vec2(20.0, 0.0));
    assert_ne!(driver.app.zoom, 1.0);
    assert_eq!(
        driver.app.worksheet, document,
        "Zoom must not mutate saved coordinates"
    );
    driver.save_as(&path);
    driver.click("new");
    driver.open(&path);
    assert_eq!(driver.app.worksheet, document);
}

#[test]
fn ui_example_contains_typeset_fractions_powers_and_live_conversion() {
    let mut driver = Driver::new();
    driver.click("example");
    assert_eq!(driver.value(4).display_value, 1.25);
    assert_eq!(driver.value(5).display_value, 125.0);
    assert_eq!(driver.value(7).display_value, 0.625);
    assert_eq!(driver.value(8).quantity.length_power, 2);
    driver.replace("region_2", "a := 50 cm");
    assert_eq!(driver.value(5).display_value, 150.0);
    assert_eq!(driver.value(7).display_value, 0.75);
}

#[test]
fn ui_ribbon_operator_inserts_at_the_formula_cursor() {
    let mut driver = Driver::new();
    driver.add_math("2");
    driver.click("operator_Power");
    assert_eq!(driver.app.worksheet.regions[0].content.source(), "2^");
    driver.frame(vec![Event::Text("3".into())]);
    driver.frame(Vec::new());
    assert_eq!(driver.app.worksheet.regions[0].content.source(), "2^3");
    assert_eq!(driver.value(0).display_value, 8.0);
}

#[test]
fn ui_text_region_width_can_be_resized_without_losing_content() {
    let mut driver = Driver::new();
    driver.click("add_text");
    let text = "A paragraph with enough text to wrap when the region is made narrower. It must all survive resizing.";
    driver.replace("region_1", text);
    driver.finish_edit();
    let width = driver.app.worksheet.regions[0].layout.unwrap().width;
    driver.drag("resize_1", egui::vec2(-240.0, 0.0));
    assert_eq!(
        driver.app.worksheet.regions[0].layout.unwrap().width,
        width - 240
    );
    assert_eq!(driver.app.worksheet.regions[0].content.source(), text);
    let reopened = Worksheet::from_json(&driver.app.worksheet.to_json().unwrap()).unwrap();
    assert_eq!(reopened, driver.app.worksheet);
}

#[test]
fn ui_typing_at_the_page_crosshair_starts_a_mathematical_region() {
    let mut driver = Driver::new();
    let page = driver.app.targets["page_0"];
    driver.click_at(page.min + egui::vec2(250.0, 400.0));
    driver.frame(vec![Event::Text("25 cm".into())]);
    driver.frame(Vec::new());
    assert_eq!(driver.app.worksheet.regions.len(), 1);
    assert_eq!(driver.app.worksheet.regions[0].content.source(), "25 cm");
    assert_eq!(driver.app.worksheet.regions[0].layout.unwrap().x, 250);
    assert_eq!(driver.value(0).quantity.si_value, 0.25);
}

#[test]
fn ui_move_up_between_columns_keeps_both_regions_within_page_bounds() {
    let mut driver = Driver::new();
    driver.click("example");
    driver.click("region_6");
    driver.click("move");
    assert!(
        driver
            .app
            .worksheet
            .regions
            .iter()
            .all(|region| region.layout.unwrap().is_valid())
    );
    assert_eq!(
        Worksheet::from_json(&driver.app.worksheet.to_json().unwrap()).unwrap(),
        driver.app.worksheet
    );
}
