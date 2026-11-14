mod app;
mod disk;
mod fs_ops;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("oxifs — file manager")
            .with_inner_size([1280.0_f32, 720.0_f32])
            .with_min_inner_size([800.0_f32, 500.0_f32])
            .with_app_id("oxifs"),
        ..Default::default()
    };
    eframe::run_native("oxifs", options, Box::new(|cc| Ok(Box::new(app::OxifsApp::new(cc)))))
        .expect("Failed to launch oxifs");
}
