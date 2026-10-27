use quire::QuireApp;

fn main() -> eframe::Result {
    eframe::run_native(
        "Quire",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1120.0, 800.0])
                .with_min_inner_size([820.0, 600.0]),
            ..Default::default()
        },
        Box::new(|_context| Ok(Box::new(QuireApp::default()))),
    )
}
