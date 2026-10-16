#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod canvas;
mod icons;
mod interface;
mod persistence;
mod ribbon;
mod session;

fn main() -> eframe::Result {
    let mut window_size = [1386.0, 963.0];
    if cfg!(debug_assertions) {
        let args: Vec<_> = std::env::args().collect();
        if let Some(index) = args.iter().position(|arg| arg == "--screenshot-size")
            && let Some(size) = args.get(index + 1)
            && let Some((width, height)) = size.split_once('x')
            && let (Ok(width), Ok(height)) = (width.parse::<u32>(), height.parse::<u32>())
            && (780..=3840).contains(&width)
            && (520..=2160).contains(&height)
        {
            window_size = [width as f32, height as f32];
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(window_size)
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
