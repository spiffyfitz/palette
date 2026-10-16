#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod canvas;
mod icons;
mod ribbon;
mod session;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1386.0, 963.0])
            .with_min_inner_size([780.0, 520.0])
            .with_title("Untitled — Palette"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Palette",
        options,
        Box::new(|cc| Ok(Box::new(app::PaletteApp::new(cc)))),
    )
}
