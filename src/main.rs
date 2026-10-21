#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod canvas;
mod gpu;
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
            && (ribbon::layout::MIN_WINDOW_WIDTH as u32..=3840).contains(&width)
            && (520..=2160).contains(&height)
        {
            window_size = [width as f32, height as f32];
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            // Leave all window controls to the OS/window manager. The command
            // and document-tab row stays entirely inside the decorated client
            // area on Windows, macOS and Linux, including left-side controls.
            .with_decorations(true)
            .with_inner_size(window_size)
            .with_min_inner_size([ribbon::layout::MIN_WINDOW_WIDTH, 520.0])
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
