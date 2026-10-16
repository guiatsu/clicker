#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod hotkeys;
mod input;
mod overlay;
mod positions;
mod profiles;
mod settings;

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    overlay::set_process_dpi_awareness();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 900.0])
            .with_min_inner_size([980.0, 720.0])
            .with_title("Neo AutoClicker"),
        ..Default::default()
    };

    eframe::run_native(
        "Neo AutoClicker",
        options,
        Box::new(|cc| Ok(Box::new(app::AutoClickerApp::new(cc)))),
    )
}
