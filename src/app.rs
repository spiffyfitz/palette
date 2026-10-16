mod panels;

use crate::{
    icons::{self, Icon},
    session::{Session, Tool},
};
use eframe::egui::{
    self, Color32, ColorImage, Context, Key, RichText, Sense, TextureHandle, TextureOptions, Ui,
    Vec2, vec2,
};
use palette_core::{
    brush::Brush,
    composite,
    document::{BlendMode, Document, Filter, FilterInstance},
    io,
    raster::Raster,
};
use std::{collections::HashMap, path::PathBuf, sync::mpsc, time::Duration};

const PANEL: Color32 = Color32::from_rgb(39, 39, 47);
pub const WORKSPACE: Color32 = Color32::from_rgb(30, 30, 39);
pub const ACCENT: Color32 = Color32::from_rgb(53, 215, 225);

/// Build GPU-ready pixels on the rendering worker. Extremely wide/tall images
/// retain their full editing raster but use a reduced display texture, avoiding
/// a driver-limit panic. Tiled display textures will restore detail at high zoom.
fn texture_image(raster: &Raster, max_side: usize) -> ColorImage {
    let divisor = (raster.width().max(raster.height()) as usize).div_ceil(max_side.max(1));
    let width = raster.width().div_ceil(divisor as u32);
    let height = raster.height().div_ceil(divisor as u32);
    let rgba = if divisor == 1 {
        raster.rgba()
    } else {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&raster.get(x * divisor as u32, y * divisor as u32));
            }
        }
        rgba
    };
    ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba)
}

pub struct RenderRequest {
    id: u64,
    generation: u64,
    document: Document,
    max_texture_side: usize,
}
struct RenderResult {
    id: u64,
    generation: u64,
    pixels: Raster,
    image: ColorImage,
}
enum FileResult {
    Opened(PathBuf, Result<Document, String>),
    Saved {
        id: u64,
        revision: u64,
        path: PathBuf,
        export: bool,
        result: Result<(), String>,
    },
}

pub struct PaletteApp {
    pub sessions: Vec<Session>,
    pub active: usize,
    next_id: u64,
    pub tool: Tool,
    pub brush: Brush,
    pub background: Color32,
    pub show_layers: bool,
    pub checkerboard: bool,
    pub cursor: Option<[u32; 2]>,
    pub stroke: Option<([f32; 2], [f32; 2])>,
    pub gesture_session: Option<u64>,
    pub shape_fill: bool,
    pub stroke_background: bool,
    pub error: Option<String>,
    message: String,
    pub show_filters: bool,
    close_pending: Option<u64>,
    close_after_save: Option<u64>,
    quitting: bool,
    allow_quit: bool,
    busy: bool,
    pending_files: usize,
    render_tx: mpsc::SyncSender<RenderRequest>,
    render_rx: mpsc::Receiver<RenderResult>,
    file_tx: mpsc::Sender<FileResult>,
    file_rx: mpsc::Receiver<FileResult>,
    thumbnails: HashMap<(u64, u64), (u64, TextureHandle)>,
    pub canvas_rect: egui::Rect,
    pub resize_start: Option<([u32; 2], egui::Pos2)>,
    pub resize_preview: Option<[u32; 2]>,
    screenshot: Option<PathBuf>,
    screenshot_requested: bool,
}

impl PaletteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        let mut app = Self::with_context(ctx);
        let mut args = std::env::args_os().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--screenshot" {
                app.screenshot = args.next().map(PathBuf::from);
                ctx.set_pixels_per_point(1.0);
            } else {
                let path = PathBuf::from(arg);
                if path.is_file() {
                    app.open_path(path, ctx);
                }
            }
        }
        app
    }

    fn with_context(ctx: &Context) -> Self {
        ctx.set_theme(egui::Theme::Dark);
        let mut style = (*ctx.global_style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = PANEL;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = WORKSPACE;
        style.visuals.selection.bg_fill = Color32::from_rgb(37, 83, 90);
        style.visuals.selection.stroke.color = ACCENT;
        style.spacing.item_spacing = vec2(7.0, 5.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
        ctx.set_global_style(style);
        let (render_tx, requests) = mpsc::sync_channel::<RenderRequest>(1);
        let (results, render_rx) = mpsc::channel();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            while let Ok(request) = requests.recv() {
                let pixels = composite::render(&request.document);
                let image = texture_image(&pixels, request.max_texture_side);
                if results
                    .send(RenderResult {
                        id: request.id,
                        generation: request.generation,
                        pixels,
                        image,
                    })
                    .is_err()
                {
                    break;
                }
                repaint.request_repaint();
            }
        });
        let (file_tx, file_rx) = mpsc::channel();
        let mut app = Self {
            sessions: vec![],
            active: 0,
            next_id: 1,
            tool: Tool::Brush,
            brush: Brush {
                diameter: 12.0,
                opacity: 1.0,
                hardness: 1.0,
                color: [0, 0, 0, 255],
                erase: false,
            },
            background: Color32::WHITE,
            show_layers: true,
            checkerboard: true,
            cursor: None,
            stroke: None,
            gesture_session: None,
            shape_fill: false,
            stroke_background: false,
            error: None,
            message: "Ready".into(),
            show_filters: false,
            close_pending: None,
            close_after_save: None,
            quitting: false,
            allow_quit: false,
            busy: false,
            pending_files: 0,
            render_tx,
            render_rx,
            file_tx,
            file_rx,
            thumbnails: HashMap::new(),
            canvas_rect: egui::Rect::NOTHING,
            resize_start: None,
            resize_preview: None,
            screenshot: None,
            screenshot_requested: false,
        };
        app.new_document();
        app
    }

    pub fn current(&self) -> &Session {
        &self.sessions[self.active]
    }
    pub fn current_mut(&mut self) -> &mut Session {
        &mut self.sessions[self.active]
    }
    pub fn close_pending_none(&self) -> bool {
        self.close_pending.is_none() && self.error.is_none()
    }

    pub fn new_document(&mut self) {
        self.stroke = None;
        let session = Session::new(
            self.next_id,
            "Untitled".into(),
            Document::new(819, 594).expect("default dimensions"),
        );
        self.next_id += 1;
        self.sessions.push(session);
        self.active = self.sessions.len() - 1;
    }

    pub fn open_dialog(&mut self, ctx: &Context) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Images and Palette projects",
                &[
                    "palette", "png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp",
                ],
            )
            .pick_files()
        {
            for path in paths {
                self.open_path(path, ctx);
            }
        }
    }
    pub fn open_path(&mut self, path: PathBuf, ctx: &Context) {
        if self.pending_files >= 4 {
            self.error = Some("At most four file operations can run at once. Please open the remaining files after these finish.".into());
            return;
        }
        self.pending_files += 1;
        self.busy = true;
        let tx = self.file_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = io::open(&path).map_err(|e| e.to_string());
            let _ = tx.send(FileResult::Opened(path, result));
            ctx.request_repaint();
        });
    }

    pub fn save(&mut self, ctx: &Context, save_as: bool, export: bool) {
        if self.busy {
            return;
        }
        let session = self.current();
        let path = if !save_as && !export {
            session.path.clone()
        } else {
            None
        };
        let path = path.or_else(|| {
            if export {
                rfd::FileDialog::new()
                    .set_title("Export flattened image")
                    .add_filter("PNG — preserves transparency", &["png"])
                    .add_filter("JPEG — white background", &["jpg", "jpeg"])
                    .add_filter("BMP", &["bmp"])
                    .add_filter("GIF — limited palette", &["gif"])
                    .add_filter("TIFF", &["tiff"])
                    .add_filter("WebP — lossless", &["webp"])
                    .set_file_name(format!("{}.png", session.name))
                    .save_file()
            } else {
                rfd::FileDialog::new()
                    .set_title("Save editable Palette project")
                    .add_filter("Palette project", &["palette"])
                    .set_file_name(format!("{}.palette", session.name))
                    .save_file()
            }
        });
        let Some(mut path) = path else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension(if export { "png" } else { "palette" });
        }
        let document = session.document.clone();
        let id = session.id;
        let revision = session.history.revision;
        let tx = self.file_tx.clone();
        let repaint = ctx.clone();
        self.pending_files += 1;
        self.busy = true;
        std::thread::spawn(move || {
            let result = if export {
                io::export(&path, &document)
            } else {
                io::save_project(&path, &document)
            }
            .map_err(|e| e.to_string());
            let _ = tx.send(FileResult::Saved {
                id,
                revision,
                path,
                export,
                result,
            });
            repaint.request_repaint();
        });
    }

    fn poll_workers(&mut self, ctx: &Context) {
        while let Ok(result) = self.render_rx.try_recv() {
            if let Some(session) = self
                .sessions
                .iter_mut()
                .find(|s| s.id == result.id && s.generation == result.generation)
            {
                let color = result.image;
                if let Some(texture) = &mut session.texture {
                    texture.set(color, TextureOptions::NEAREST);
                } else {
                    session.texture = Some(ctx.load_texture(
                        format!("canvas-{}", session.id),
                        color,
                        TextureOptions::NEAREST,
                    ));
                }
                session.rendered = Some(result.pixels);
            }
        }
        while let Ok(result) = self.file_rx.try_recv() {
            self.pending_files = self.pending_files.saturating_sub(1);
            self.busy = self.pending_files != 0;
            match result {
                FileResult::Opened(path, result) => match result {
                    Ok(document) => {
                        let is_project = io::extension(&path) == "palette";
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        let mut session = Session::new(self.next_id, name, document);
                        if is_project {
                            session.path = Some(path.clone());
                        }
                        self.next_id += 1;
                        self.sessions.push(session);
                        self.active = self.sessions.len() - 1;
                        if io::extension(&path) == "gif" {
                            self.message =
                                "Imported first GIF frame; animation is not preserved".into();
                        } else {
                            self.message = format!("Opened {}", path.display());
                        }
                    }
                    Err(error) => self.error = Some(error),
                },
                FileResult::Saved {
                    id,
                    revision,
                    path,
                    export,
                    result,
                } => match result {
                    Ok(()) => {
                        if !export
                            && let Some(session) = self.sessions.iter_mut().find(|s| s.id == id)
                        {
                            session.path = Some(path.clone());
                            session.name = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            session.saved_revision = revision;
                        }
                        self.message = format!(
                            "{} {}",
                            if export { "Exported" } else { "Saved" },
                            path.display()
                        );
                        if self.close_after_save == Some(id) {
                            self.close_after_save = None;
                            if self
                                .sessions
                                .iter()
                                .find(|s| s.id == id)
                                .is_some_and(|s| !s.modified())
                            {
                                self.remove_tab(id, ctx);
                            } else {
                                self.close_pending = Some(id);
                            }
                        }
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.close_after_save = None;
                        self.quitting = false;
                    }
                },
            }
        }
    }

    fn request_render(&mut self, ctx: &Context) {
        let session = self.current();
        if session.requested == Some(session.generation) {
            return;
        }
        let request = RenderRequest {
            id: session.id,
            generation: session.generation,
            document: session.document.clone(),
            max_texture_side: ctx.input(|i| i.max_texture_side),
        };
        if self.render_tx.try_send(request).is_ok() {
            let session = self.current_mut();
            session.requested = Some(session.generation);
        } else {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn request_close(&mut self, id: u64, ctx: &Context) {
        if self.busy {
            self.message = "Please wait for the file operation to finish".into();
            return;
        }
        if self
            .sessions
            .iter()
            .find(|s| s.id == id)
            .is_some_and(Session::modified)
        {
            self.close_pending = Some(id);
        } else {
            self.remove_tab(id, ctx);
        }
    }
    fn remove_tab(&mut self, id: u64, ctx: &Context) {
        if let Some(index) = self.sessions.iter().position(|s| s.id == id) {
            let active_id = self.current().id;
            self.sessions.remove(index);
            self.thumbnails
                .retain(|(session_id, _), _| *session_id != id);
            self.active = self
                .sessions
                .iter()
                .position(|s| s.id == active_id)
                .unwrap_or(index.min(self.sessions.len().saturating_sub(1)));
            self.stroke = None;
        }
        self.close_pending = None;
        if self.quitting {
            if let Some(session) = self.sessions.iter().find(|s| s.modified()) {
                self.close_pending = Some(session.id);
            } else {
                self.allow_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.sessions.is_empty() {
            self.new_document();
        }
    }

    fn shortcuts(&mut self, ctx: &Context) {
        if ctx.egui_wants_keyboard_input() || self.close_pending.is_some() {
            return;
        }
        let keys = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.ctrl,
                i.modifiers.shift,
                i.key_pressed(Key::N),
                i.key_pressed(Key::O),
                i.key_pressed(Key::S),
                i.key_pressed(Key::W),
                i.key_pressed(Key::Z),
                i.key_pressed(Key::Y),
                i.key_pressed(Key::Tab),
            )
        });
        if keys.0 && keys.3 {
            self.new_document();
        }
        if keys.0 && keys.4 {
            self.open_dialog(ctx);
        }
        if keys.0 && keys.5 {
            self.save(ctx, keys.2, false);
        }
        if keys.0 && keys.6 {
            self.request_close(self.current().id, ctx);
        }
        if keys.0 && keys.7 {
            if keys.2 {
                self.current_mut().redo();
            } else {
                self.current_mut().undo();
            }
        }
        if keys.0 && keys.8 {
            self.current_mut().redo();
        }
        if keys.1 && keys.9 {
            self.stroke = None;
            self.active = (self.active + if keys.2 { self.sessions.len() - 1 } else { 1 })
                % self.sessions.len();
        }
    }
}

impl eframe::App for PaletteApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

impl PaletteApp {
    fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.poll_workers(&ctx);
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_quit
            && (self.busy || self.sessions.iter().any(Session::modified))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.busy {
                self.quitting = true;
                self.close_pending = self.sessions.iter().find(|s| s.modified()).map(|s| s.id);
            }
        }
        self.shortcuts(&ctx);
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            self.open_path(file.path().to_path_buf(), &ctx);
        }
        egui::Panel::top("menu")
            .exact_size(34.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(31, 31, 39))
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ui, |ui| self.menus(ui));
        egui::Panel::top("tabs")
            .exact_size(31.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(31, 31, 39))
                    .inner_margin(egui::Margin::symmetric(10, 2)),
            )
            .show(ui, |ui| self.tabs(ui));
        egui::Panel::top("ribbon")
            .exact_size(110.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ui, |ui| self.ribbon(ui));
        egui::Panel::bottom("status")
            .exact_size(32.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(31, 31, 39))
                    .inner_margin(egui::Margin::symmetric(10, 2)),
            )
            .show(ui, |ui| self.status(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(WORKSPACE))
            .show(ui, |ui| self.canvas(ui));
        self.dialogs(&ctx);
        self.request_render(&ctx);
        let s = self.current();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}{} — Palette",
            s.name,
            if s.modified() { " •" } else { "" }
        )));
        if !self.screenshot_requested
            && self.current().texture.is_some()
            && !self.busy
            && let Some(path) = self.screenshot.clone()
        {
            self.screenshot_requested = true;
            let repaint = ctx.clone();
            ctx.request_screenshot(move |image| {
                let bytes: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|p| p.to_srgba_unmultiplied())
                    .collect();
                if let Err(error) = image::save_buffer(
                    &path,
                    &bytes,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Screenshot failed: {error}");
                }
                repaint.send_viewport_cmd(egui::ViewportCommand::Close);
                repaint.request_repaint();
            });
        }
    }
}

#[cfg(test)]
mod tests;
