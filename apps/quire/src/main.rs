use eframe::egui;
use quire_core::Worksheet;

#[derive(Default)]
struct QuireApp {
    worksheet: Worksheet,
}

impl eframe::App for QuireApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(self.worksheet.title());
            ui.label("Changes are not saved yet.");
            ui.add_space(12.0);

            if ui.button("Add text").clicked() {
                self.worksheet.add_text_region();
            }

            ui.add_space(8.0);
            if self.worksheet.text_regions().is_empty() {
                ui.label("Add text to start a worksheet.");
            }

            for text in self.worksheet.text_regions_mut() {
                ui.add(egui::TextEdit::multiline(text).desired_width(f32::INFINITY));
            }
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Quire",
        eframe::NativeOptions::default(),
        Box::new(|_context| Ok(Box::new(QuireApp::default()))),
    )
}
