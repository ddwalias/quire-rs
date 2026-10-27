use quire::QuireApp;

fn main() -> eframe::Result {
    eframe::run_native(
        "Quire",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default().with_inner_size([900.0, 760.0]),
            ..Default::default()
        },
        Box::new(|_context| Ok(Box::new(QuireApp::default()))),
    )
}
