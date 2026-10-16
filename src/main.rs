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
            .with_inner_size([1120.0, 820.0])
            .with_min_inner_size([820.0, 640.0])
            .with_title("Rust AutoClicker"),
        ..Default::default()
    };

    eframe::run_native(
        "Rust AutoClicker",
        options,
        Box::new(|cc| Ok(Box::new(app::AutoClickerApp::new(cc)))),
    )
}
