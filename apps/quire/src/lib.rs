use eframe::egui;
use quire_core::{Calculation, MathError, RegionContent, RegionLayout, Worksheet};
use std::path::PathBuf;

macro_rules! target {
    ($app:expr, $name:expr, $response:expr) => {
        let _ = &$response;
        #[cfg(test)]
        {
            $app.targets.insert($name.into(), $response.rect);
        }
    };
}

mod canvas;
mod chrome;
mod equation;

#[derive(Clone)]
enum Action {
    New,
    Open(PathBuf),
    Example,
    Close,
    Overwrite(PathBuf),
}
#[derive(Clone, Copy, PartialEq)]
enum FileMode {
    Open,
    Save,
}
#[derive(Clone, Copy, PartialEq)]
enum RibbonTab {
    Math,
    Document,
    Calculation,
}

/// Desktop state around the portable worksheet. Region geometry is stored in
/// document coordinates; zoom, selection and editor state belong to the view.
pub struct QuireApp {
    worksheet: Worksheet,
    saved: Worksheet,
    current_path: Option<PathBuf>,
    path_input: String,
    results: Vec<Option<Result<Calculation, MathError>>>,
    status: String,
    status_is_error: bool,
    pending: Option<Action>,
    file_mode: Option<FileMode>,
    selected: Option<u64>,
    editing: Option<u64>,
    focus_region: Option<u64>,
    insertion: Option<RegionLayout>,
    allow_close: bool,
    editor_generation: u64,
    configured: bool,
    zoom: f32,
    show_grid: bool,
    tab: RibbonTab,
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
            status: "Click the page to place a region. Double-click a region to edit.".into(),
            status_is_error: false,
            pending: None,
            file_mode: None,
            selected: None,
            editing: None,
            focus_region: None,
            insertion: Some(RegionLayout::automatic(0)),
            allow_close: false,
            editor_generation: 0,
            configured: false,
            zoom: 1.0,
            show_grid: true,
            tab: RibbonTab::Math,
            #[cfg(test)]
            targets: Default::default(),
        }
    }
}

impl QuireApp {
    fn dirty(&self) -> bool {
        self.worksheet != self.saved
    }
    fn report(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = message.into();
        self.status_is_error = is_error;
    }
    fn recalculate(&mut self) {
        self.results = self.worksheet.calculate();
    }

    fn request(&mut self, action: Action, ctx: &egui::Context) {
        self.editing = None;
        if self.dirty() {
            self.pending = Some(action);
        } else {
            self.perform(action, ctx);
        }
    }
    fn reset_view(&mut self) {
        self.editor_generation = self.editor_generation.wrapping_add(1);
        self.selected = None;
        self.editing = None;
        self.focus_region = None;
        self.insertion = None;
    }
    fn perform(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::New => {
                self.reset_view();
                self.insertion = Some(RegionLayout::automatic(0));
                self.worksheet = Worksheet::default();
                self.saved = self.worksheet.clone();
                self.current_path = None;
                self.results.clear();
                self.report("New worksheet", false);
            }
            Action::Open(path) => match Worksheet::open(&path) {
                Ok(sheet) => {
                    self.reset_view();
                    self.worksheet = sheet;
                    self.saved = self.worksheet.clone();
                    self.current_path = Some(path);
                    self.recalculate();
                    self.report("Opened worksheet", false);
                }
                Err(e) => self.report(e.to_string(), true),
            },
            Action::Example => match Worksheet::from_json(include_str!(
                "../../../corpus/canvas-example.quire.json"
            )) {
                Ok(sheet) => {
                    self.reset_view();
                    self.worksheet = sheet;
                    self.saved = self.worksheet.clone();
                    self.current_path = None;
                    self.path_input = "length-study.quire.json".into();
                    self.recalculate();
                    self.report(
                        "Example worksheet. Double-click any region to edit it.",
                        false,
                    );
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
                self.current_path = Some(path.clone());
                self.path_input = path.to_string_lossy().into_owned();
                self.file_mode = None;
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
            self.file_mode = None;
            self.pending = Some(Action::Overwrite(path));
        } else {
            self.save(path);
        }
    }
    fn begin_save(&mut self) {
        if let Some(path) = self.current_path.clone() {
            self.save(path);
        } else {
            self.file_mode = Some(FileMode::Save);
        }
    }
    fn add_region(&mut self, text: bool) {
        if self.insertion.is_none()
            && self
                .worksheet
                .regions
                .iter()
                .filter_map(|region| region.layout)
                .max_by_key(|layout| (layout.page, layout.y))
                .is_some_and(|last| {
                    last.page >= 999 && last.y + 84 > quire_core::layout::CONTENT_BOTTOM
                })
        {
            self.report(
                "The 1000-page limit has been reached. Place a region on an existing page.",
                true,
            );
            return;
        }
        let id = self.worksheet.add_region(if text {
            RegionContent::Text(String::new())
        } else {
            RegionContent::Math(String::new())
        });
        if let Some(position) = self.insertion.take() {
            self.worksheet
                .regions
                .iter_mut()
                .find(|region| region.id == id)
                .unwrap()
                .layout = Some(position);
        }
        self.worksheet.sort_by_position();
        self.recalculate();
        self.selected = Some(id);
        self.editing = Some(id);
        self.focus_region = Some(id);
    }
    fn delete_selected(&mut self) {
        if let Some(id) = self.selected.take() {
            self.worksheet.regions.retain(|region| region.id != id);
            self.editing = None;
            self.editor_generation = self.editor_generation.wrapping_add(1);
            self.recalculate();
        }
    }
    fn edit_selected(&mut self) {
        if let Some(id) = self.selected {
            self.editing = Some(id);
            self.focus_region = Some(id);
        }
    }
    fn source_id(&self, id: u64) -> egui::Id {
        egui::Id::unique(("source", self.editor_generation, id))
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() || self.file_mode.is_some() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.begin_save();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.file_mode = Some(FileMode::Open);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.request(Action::New, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::M)) {
            self.add_region(false);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::T)) {
            self.add_region(true);
        }
        if self.editing.is_some()
            && ctx.input(|i| {
                i.key_pressed(egui::Key::Escape)
                    || (i.modifiers.command && i.key_pressed(egui::Key::Enter))
            })
        {
            self.editing = None;
            ctx.memory_mut(|memory| {
                memory.surrender_focus(memory.focused().unwrap_or(egui::Id::NULL))
            });
        }
        if self.editing.is_none() && !ctx.egui_wants_keyboard_input() {
            if self.selected.is_none()
                && self.insertion.is_some()
                && ctx.input(|i| {
                    !i.modifiers.command
                        && i.events.iter().any(
                            |event| matches!(event, egui::Event::Text(text) if !text.is_empty()),
                        )
                })
            {
                // Leave the text event for the newly focused source editor.
                self.add_region(false);
            }
            if ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            {
                self.delete_selected();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.edit_selected();
            }
            let step = if ctx.input(|i| i.modifiers.shift) {
                1.0
            } else {
                8.0
            };
            let delta = ctx.input(|i| {
                egui::vec2(
                    if i.key_pressed(egui::Key::ArrowRight) {
                        step
                    } else if i.key_pressed(egui::Key::ArrowLeft) {
                        -step
                    } else {
                        0.0
                    },
                    if i.key_pressed(egui::Key::ArrowDown) {
                        step
                    } else if i.key_pressed(egui::Key::ArrowUp) {
                        -step
                    } else {
                        0.0
                    },
                )
            });
            if delta != egui::Vec2::ZERO {
                self.move_selected(delta);
            }
        }
    }

    /// The production UI also runs directly in headless egui workflow tests.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.configured {
            equation::install_fonts(&ctx);
            let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
            style.visuals = egui::Visuals::light();
            style.visuals.selection.bg_fill = egui::Color32::from_rgb(211, 230, 250);
            style.visuals.selection.stroke.color = equation::BLUE;
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
            ctx.set_theme(egui::Theme::Light);
            ui.set_style(style.clone());
            ctx.set_style_of(egui::Theme::Light, style);
            self.configured = true;
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Action::Close);
        }
        self.shortcuts(&ctx);
        self.chrome(ui);
        self.canvas(ui);
        self.dialogs(&ctx);
    }
}

impl eframe::App for QuireApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests;
