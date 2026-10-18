mod advanced;
pub(crate) mod appearance;
mod capture;
mod close;
pub(crate) mod editing;
mod feather;
mod files;
mod panels;
pub(crate) mod processing;
mod selection_brush;
mod selection_toolbar;
mod text;

use crate::{
    icons::{self, Icon},
    session::{Session, Tool},
};
use eframe::egui::{
    self, Color32, ColorImage, Context, Key, Sense, TextureHandle, TextureOptions, Ui, Vec2, vec2,
};
use palette_core::{
    brush::Brush,
    composite,
    document::{BlendMode, Document, Filter, FilterInstance},
    io,
    raster::Raster,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

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
    ticket: u64,
    id: u64,
    generation: u64,
    epoch: u64,
    document: Document,
    max_texture_side: usize,
    selection: Option<palette_core::transform::SelectionTransform>,
}
struct RenderResult {
    id: u64,
    generation: u64,
    epoch: u64,
    pixels: Raster,
    image: ColorImage,
    thumbnails: Vec<(u64, ColorImage)>,
    selection: Option<(
        palette_core::transform::SelectionTransform,
        Result<Document, String>,
    )>,
}
enum FileResult {
    Imported {
        session: u64,
        generation: u64,
        path: PathBuf,
        result: Result<io::ImportReport, String>,
    },
    Autosaved {
        session: u64,
        revision: u64,
        result: Result<(), String>,
    },
    Recovered(crate::persistence::Recovery, Result<Document, String>),
    Opened(PathBuf, Result<io::ImportReport, String>),
    Saved {
        id: u64,
        revision: u64,
        path: PathBuf,
        export: bool,
        result: Result<(), String>,
    },
}

struct RotationDialog {
    session: u64,
    generation: u64,
    degrees: String,
    focus: bool,
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
    pub shape_outline: bool,
    pub stroke_background: bool,
    pub error: Option<String>,
    pub(crate) message: String,
    pub(crate) processing_toolbar: Option<processing::Toolbar>,
    pub processing_toolbar_rect: egui::Rect,
    pub layer_processing_buttons: [egui::Rect; 3],
    close_pending: Option<u64>,
    close_after_save: Option<u64>,
    quitting: bool,
    allow_quit: bool,
    busy: bool,
    pending_files: usize,
    render_tx: mpsc::SyncSender<RenderRequest>,
    render_rx: mpsc::Receiver<RenderResult>,
    render_ticket: Arc<AtomicU64>,
    render_target: Option<(u64, u64, u64)>,
    file_tx: mpsc::Sender<FileResult>,
    file_rx: mpsc::Receiver<FileResult>,
    thumbnails: HashMap<(u64, u64), (u64, TextureHandle)>,
    pub canvas_rect: egui::Rect,
    pub selection_toolbar_rect: egui::Rect,
    pub canvas_scrollbars: [Option<egui::Rect>; 2],
    pub image_rect: egui::Rect,
    pub resize_start: Option<([u32; 2], egui::Pos2)>,
    pub resize_preview: Option<[u32; 2]>,
    screenshot: Option<PathBuf>,
    screenshot_requested: bool,
    capture: capture::CaptureState,
    clipboard: Option<arboard::Clipboard>,
    edit_tx: mpsc::Sender<editing::EditResult>,
    edit_rx: mpsc::Receiver<editing::EditResult>,
    pub edit_pending: bool,
    resize_dialog: Option<editing::ResizeDialog>,
    rotation_dialog: Option<RotationDialog>,
    pub fill_tolerance: u8,
    store: Option<std::sync::Arc<crate::persistence::Store>>,
    recent: Vec<PathBuf>,
    recoveries: Vec<crate::persistence::Recovery>,
    restored: HashMap<u64, crate::persistence::Recovery>,
    recovery_prompt: bool,
    recovery_pending: Option<u64>,
    recovery_seen: HashMap<u64, u64>,
    recovery_timer: std::time::Instant,
    deferred_close: bool,
    avif_export: Option<files::AvifExport>,
    paint_export: Option<files::PaintExport>,
    feather_dialog: Option<advanced::FeatherDialog>,
    feather_worker: feather::Worker,
    selection_brush_worker: selection_brush::Worker,
    processing_dialog: Option<processing::Dialog>,
    processing_worker: processing::Worker,
    text_editor: Option<text::TextEditor>,
    text_tx: mpsc::SyncSender<text::TextRequest>,
    text_rx: mpsc::Receiver<text::TextResult>,
    next_text_job: u64,
    after_text: Option<text::AfterText>,
    pub interface: crate::interface::InterfaceState,
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
            } else if cfg!(debug_assertions) && arg == "--screenshot-scene" {
                app.capture.scene = args.next().map(|arg| arg.to_string_lossy().into_owned());
            } else {
                let path = PathBuf::from(arg);
                if path.is_file() {
                    app.open_path(path, ctx);
                }
            }
        }
        app.prepare_capture();
        if app.screenshot.is_none() {
            if let Some(root) = crate::persistence::data_directory() {
                app.init_store(root);
            } else {
                app.message = "Recovery is unavailable: no application data directory".into();
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
        crate::interface::configure(ctx);
        let (render_tx, requests) = mpsc::sync_channel::<RenderRequest>(1);
        let (results, render_rx) = mpsc::channel();
        let repaint = ctx.clone();
        let render_ticket = Arc::new(AtomicU64::new(0));
        let worker_ticket = render_ticket.clone();
        std::thread::spawn(move || {
            let mut renderer = composite::Renderer::default();
            while let Ok(request) = requests.recv() {
                let cancelled = || worker_ticket.load(Ordering::Relaxed) != request.ticket;
                if cancelled() {
                    continue;
                }
                let mut document = request.document;
                let selection = request.selection.map(|operation| {
                    let result = operation
                        .apply(&mut document)
                        .and_then(|_| document.validate())
                        .map(|_| document.clone());
                    (operation, result)
                });
                let frame = renderer.render_frame_cancel(&document, &cancelled);
                if cancelled() {
                    continue;
                }
                let frame = frame.expect("validated render request");
                let pixels = frame.pixels;
                let thumbnails = frame
                    .thumbnails
                    .into_iter()
                    .map(|(id, pixels)| (id, texture_image(&pixels, 48)))
                    .collect();
                let image = texture_image(&pixels, request.max_texture_side);
                if results
                    .send(RenderResult {
                        id: request.id,
                        generation: request.generation,
                        epoch: request.epoch,
                        pixels,
                        image,
                        thumbnails,
                        selection,
                    })
                    .is_err()
                {
                    break;
                }
                repaint.request_repaint();
            }
        });
        let (file_tx, file_rx) = mpsc::channel();
        let (edit_tx, edit_rx) = mpsc::channel();
        let (text_tx, text_rx) = text::worker();
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
            shape_outline: true,
            stroke_background: false,
            error: None,
            message: "Ready".into(),
            processing_toolbar: None,
            processing_toolbar_rect: egui::Rect::NOTHING,
            layer_processing_buttons: [egui::Rect::NOTHING; 3],
            close_pending: None,
            close_after_save: None,
            quitting: false,
            allow_quit: false,
            busy: false,
            pending_files: 0,
            render_tx,
            render_rx,
            render_ticket,
            render_target: None,
            file_tx,
            file_rx,
            thumbnails: HashMap::new(),
            canvas_rect: egui::Rect::NOTHING,
            selection_toolbar_rect: egui::Rect::NOTHING,
            canvas_scrollbars: [None, None],
            image_rect: egui::Rect::NOTHING,
            resize_start: None,
            resize_preview: None,
            screenshot: None,
            screenshot_requested: false,
            capture: Default::default(),
            clipboard: None,
            text_editor: None,
            text_tx,
            text_rx,
            next_text_job: 1,
            after_text: None,
            interface: Default::default(),
            edit_tx,
            edit_rx,
            edit_pending: false,
            resize_dialog: None,
            rotation_dialog: None,
            fill_tolerance: 0,
            store: None,
            recent: vec![],
            recoveries: vec![],
            restored: HashMap::new(),
            recovery_prompt: false,
            recovery_pending: None,
            recovery_seen: HashMap::new(),
            recovery_timer: std::time::Instant::now(),
            deferred_close: false,
            avif_export: None,
            paint_export: None,
            feather_dialog: None,
            feather_worker: feather::Worker::new(ctx.clone()),
            selection_brush_worker: selection_brush::Worker::new(ctx.clone()),
            processing_dialog: None,
            processing_worker: processing::Worker::new(ctx.clone()),
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
        !self.recovery_prompt
            && self.avif_export.is_none()
            && self.paint_export.is_none()
            && self.feather_dialog.is_none()
            && self.processing_dialog.is_none()
            && self.close_pending.is_none()
            && self.error.is_none()
            && self.resize_dialog.is_none()
            && self.rotation_dialog.is_none()
            && !self.edit_pending
            && self.text_editor.is_none()
            && self.interface.color_editor.is_none()
            && self.interface.canvas_properties.is_none()
    }

    pub fn new_document(&mut self) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::New);
            return;
        }
        if !self.sessions.is_empty() {
            self.current_mut().end_gesture();
        }
        self.stroke = None;
        let session = Session::new(
            self.next_id,
            "Untitled".into(),
            Document::new(
                crate::interface::DEFAULT_CANVAS[0],
                crate::interface::DEFAULT_CANVAS[1],
            )
            .expect("default dimensions"),
        );
        self.next_id += 1;
        self.sessions.push(session);
        self.active = self.sessions.len() - 1;
    }

    pub fn open_dialog(&mut self, ctx: &Context) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Open);
            return;
        }
        self.current_mut().end_gesture();
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Images and Palette projects",
                &[
                    "palette", "png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp", "avif",
                    "paint",
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
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::OpenPath(path));
            return;
        }
        if self.pending_files >= 4 {
            self.error = Some("At most four file operations can run at once. Please open the remaining files after these finish.".into());
            return;
        }
        self.pending_files += 1;
        self.busy = true;
        let tx = self.file_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = io::open_with_report(&path).map_err(|e| e.to_string());
            let _ = tx.send(FileResult::Opened(path, result));
            ctx.request_repaint();
        });
    }

    pub fn save(&mut self, ctx: &Context, save_as: bool, export: bool) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Save(save_as, export));
            return;
        }
        if self.busy || self.edit_pending {
            return;
        }
        self.current_mut().end_gesture();
        let session = self.current();
        let path = if !save_as && !export {
            session.path.clone()
        } else {
            None
        };
        let path = path.or_else(|| {
            if export {
                rfd::FileDialog::new()
                    .set_title("Export image or Microsoft Paint project")
                    .add_filter("PNG — preserves transparency", &["png"])
                    .add_filter("JPEG — white background", &["jpg", "jpeg"])
                    .add_filter("BMP", &["bmp"])
                    .add_filter("GIF — limited palette", &["gif"])
                    .add_filter("TIFF", &["tiff"])
                    .add_filter("WebP — lossless", &["webp"])
                    .add_filter("AVIF — quality options", &["avif"])
                    .add_filter("Microsoft Paint project — experimental", &["paint"])
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
        if export && io::extension(&path) == "paint" {
            self.paint_export = Some(files::PaintExport {
                path,
                session: self.current().id,
                revision: self.current().history.revision,
                backup_complete: false,
            });
        } else if export && io::extension(&path) == "avif" {
            self.avif_export = Some(files::AvifExport {
                path,
                options: io::AvifOptions::default(),
            });
        } else {
            self.start_save(path, export, None, ctx);
        }
    }

    fn start_save(
        &mut self,
        path: PathBuf,
        export: bool,
        options: Option<io::AvifOptions>,
        ctx: &Context,
    ) {
        self.current_mut().end_gesture();
        let session = self.current();
        let document = session.document.clone();
        let id = session.id;
        let revision = session.history.revision;
        let tx = self.file_tx.clone();
        let repaint = ctx.clone();
        self.pending_files += 1;
        self.busy = true;
        std::thread::spawn(move || {
            let result = if let Some(options) = options {
                io::export_avif(&path, &document, options)
            } else if export {
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
            if let Some(session) = self.sessions.iter_mut().find(|s| {
                s.id == result.id
                    && s.render_epoch == result.epoch
                    && s.rendered_generation
                        .is_none_or(|generation| result.generation > generation)
            }) {
                let mut generation = result.generation;
                let mut epoch = result.epoch;
                if let Some((operation, preview)) = result.selection {
                    if session.selection_transform.is_none() {
                        continue;
                    }
                    match preview {
                        Ok(document) => {
                            session.document = document;
                            let gesture = session
                                .selection_transform
                                .as_mut()
                                .expect("checked gesture");
                            gesture.applied = Some(operation);
                            if gesture.released && gesture.desired == operation {
                                session.finish_selection_transform();
                                generation = session.generation;
                                epoch = session.render_epoch;
                                session.requested = Some(generation);
                            }
                        }
                        Err(error) => {
                            // A rejected oversized intermediate target must not
                            // cancel a newer, valid drag target already queued.
                            if session
                                .selection_transform
                                .as_ref()
                                .is_some_and(|g| g.desired != operation)
                            {
                                continue;
                            }
                            session.cancel_selection_transform();
                            self.error = Some(error);
                            continue;
                        }
                    }
                }
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
                session.rendered_generation = Some(generation);
                session.rendered_epoch = Some(epoch);
                for (id, image) in result.thumbnails {
                    self.thumbnails.insert(
                        (result.id, id),
                        (
                            generation,
                            ctx.load_texture(
                                format!("thumb-{}-{id}", result.id),
                                image,
                                TextureOptions::NEAREST,
                            ),
                        ),
                    );
                }
            }
        }
        while let Ok(result) = self.file_rx.try_recv() {
            if matches!(result, FileResult::Autosaved { .. }) {
                self.handle_auxiliary_file_result(result, ctx);
                continue;
            }
            self.pending_files = self.pending_files.saturating_sub(1);
            self.busy = self.pending_files != 0;
            match result {
                result @ (FileResult::Imported { .. } | FileResult::Recovered(..)) => {
                    self.handle_auxiliary_file_result(result, ctx)
                }
                FileResult::Autosaved { .. } => unreachable!("handled above"),
                FileResult::Opened(path, result) => match result {
                    Ok(report) => {
                        self.remember(&path);
                        self.current_mut().end_gesture();
                        let is_project = io::extension(&path) == "palette";
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        let mut session = Session::new(self.next_id, name, report.document);
                        if is_project {
                            session.path = Some(path.clone());
                        }
                        self.next_id += 1;
                        self.sessions.push(session);
                        self.active = self.sessions.len() - 1;
                        if let Some(notice) = report.notice {
                            self.message = notice.into();
                        } else if io::extension(&path) == "paint" {
                            self.message =
                                "Opened editable Paint layers (supported HEIF subset)".into();
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
                            && io::extension(&path) == "palette"
                            && let Some(dialog) = &mut self.paint_export
                            && dialog.session == id
                            && dialog.revision == revision
                        {
                            dialog.backup_complete = true;
                        }
                        if !export {
                            self.remember(&path);
                        }
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
                        self.clean_saved_recovery(id);
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
        let s = self.current();
        let target = (s.id, s.generation, s.render_epoch);
        if self.render_target != Some(target) {
            self.render_target = Some(target);
            self.render_ticket.fetch_add(1, Ordering::Relaxed);
            // A tab's cancelled request must be eligible again when revisited.
            // Complete cached frames remain usable across tab switches.
            for session in &mut self.sessions {
                if session.requested == Some(session.generation)
                    && session.rendered_generation != Some(session.generation)
                {
                    session.requested = None;
                }
            }
        }
        let session = self.current();
        if session.requested == Some(session.generation) {
            return;
        }
        let request = RenderRequest {
            ticket: self.render_ticket.load(Ordering::Relaxed),
            id: session.id,
            generation: session.generation,
            epoch: session.render_epoch,
            document: session
                .selection_transform
                .as_ref()
                .map_or_else(|| session.document.clone(), |g| g.base.clone()),
            selection: session.selection_transform.as_ref().map(|g| g.desired),
            max_texture_side: ctx.input(|i| i.max_texture_side),
        };
        if self.render_tx.try_send(request).is_ok() {
            let session = self.current_mut();
            session.requested = Some(session.generation);
        } else {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    /// Painting updates only changed canvas pixels and the matching GPU subimage.
    /// A pending full render is usable only within the same paint-only epoch;
    /// accumulated dirty bounds bring that snapshot all the way up to date.
    fn refresh_live_paint(&mut self) {
        let session = self.current_mut();
        let Some(region) = session.paint_dirty else {
            return;
        };
        if session
            .document
            .layers
            .iter()
            .any(palette_core::processing::has_neighborhood)
        {
            return;
        }
        if session.rendered_epoch != Some(session.render_epoch) {
            return;
        }
        let (Some(rendered), Some(texture)) = (&mut session.rendered, &mut session.texture) else {
            return;
        };
        // Oversized images use a reduced display texture; keep their background
        // snapshots progressive until tiled display textures are implemented.
        if texture.size()
            != [
                session.document.width as usize,
                session.document.height as usize,
            ]
        {
            return;
        }
        let patch = composite::render_region(&session.document, region)
            .expect("clipped brush dirty bounds");
        for y in 0..region.height {
            for x in 0..region.width {
                rendered.set(region.x + x, region.y + y, patch.get(x, y));
            }
        }
        texture.set_partial(
            [region.x as usize, region.y as usize],
            ColorImage::from_rgba_unmultiplied(
                [region.width as usize, region.height as usize],
                &patch.rgba(),
            ),
            TextureOptions::NEAREST,
        );
        session.paint_dirty = None;
        session.rendered_generation = Some(session.generation);
        session.requested = Some(session.generation);
    }

    fn request_close(&mut self, id: u64, ctx: &Context) {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == id) {
            session.end_gesture();
        }
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Close(id));
            return;
        }
        if self.busy || self.edit_pending {
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
            self.forget_recovery(id);
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
        if self.recovery_prompt
            || self.avif_export.is_some()
            || self.paint_export.is_some()
            || self.feather_dialog.is_some()
            || self.processing_dialog.is_some()
            || ctx.egui_wants_keyboard_input()
            || self.close_pending.is_some()
            || self.resize_dialog.is_some()
            || self.rotation_dialog.is_some()
            || self.error.is_some()
            || self.text_editor.is_some()
            || self.interface.color_editor.is_some()
            || self.interface.canvas_properties.is_some()
        {
            return;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::E)) {
            self.show_canvas_properties();
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
                self.redo_current();
            } else {
                self.undo_current();
            }
        }
        if keys.0 && keys.8 {
            self.redo_current();
        }
        if keys.1 && keys.9 {
            self.current_mut().end_gesture();
            self.stroke = None;
            self.active = (self.active + if keys.2 { self.sessions.len() - 1 } else { 1 })
                % self.sessions.len();
        }
        self.edit_shortcuts(ctx);
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
        self.poll_edits();
        self.poll_feather();
        self.poll_selection_brush(&ctx);
        self.poll_processing(&ctx);
        self.layer_processing_buttons = [egui::Rect::NOTHING; 3];
        self.poll_text(&ctx);
        self.finish_requested_text();
        self.resume_text_action(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) && self.recovery_pending.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.deferred_close = true;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.cancel_processing();
            self.current_mut().end_gesture();
        }
        if ctx.input(|i| i.viewport().close_requested())
            && self.recovery_pending.is_none()
            && self.text_editor.is_some()
            && !self.allow_quit
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.finish_inline_text();
            self.after_text = Some(text::AfterText::Quit);
        }
        if ctx.input(|i| i.viewport().close_requested())
            && self.recovery_pending.is_none()
            && !self.allow_quit
            && self.text_editor.is_none()
            && (self.busy || self.edit_pending || self.sessions.iter().any(Session::modified))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.busy && !self.edit_pending {
                self.quitting = true;
                self.close_pending = self.sessions.iter().find(|s| s.modified()).map(|s| s.id);
            }
        }
        self.interface.popup_was_open = egui::Popup::is_any_open(&ctx);
        self.shortcuts(&ctx);
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if self.close_pending_none() {
                self.open_path(file.path().to_path_buf(), &ctx);
            } else {
                self.message =
                    "Finish the current dialog or text before opening dropped files".into();
            }
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
        let mut ribbon_expanded = !self.interface.ribbon_collapsed;
        let mut status_visible = self.interface.status;
        egui::Panel::top("ribbon")
            .resizable(false)
            .exact_size(110.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show_collapsible(ui, &mut ribbon_expanded, |ui| self.ribbon(ui));
        egui::Panel::bottom("status")
            .resizable(false)
            .exact_size(32.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(31, 31, 39))
                    .inner_margin(egui::Margin::symmetric(10, 2)),
            )
            .show_collapsible(ui, &mut status_visible, |ui| self.status(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(WORKSPACE))
            .show(ui, |ui| self.canvas(ui));
        self.dialogs(&ctx);
        self.file_windows(&ctx);
        self.resize_window(&ctx);
        self.text_window(&ctx);
        self.appearance_windows(&ctx);
        self.request_text(&ctx);
        self.request_feather();
        self.request_selection_brush(&ctx);
        self.request_processing(&ctx);
        if self.after_text.is_some() {
            ctx.request_repaint();
        }
        if let Some(error) = self
            .sessions
            .iter_mut()
            .find_map(|s| s.gesture_error.take())
        {
            self.error = Some(error);
        }
        self.refresh_live_paint();
        self.request_render(&ctx);
        self.autosave_tick(&ctx);
        let s = self.current();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}{} — Palette",
            s.name,
            if s.modified() { " •" } else { "" }
        )));
        if !self.screenshot_requested
            && self.capture_is_settled(&ctx)
            && let Some(path) = self.screenshot.clone()
        {
            self.screenshot_requested = true;
            self.allow_quit = true;
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

impl Drop for PaletteApp {
    fn drop(&mut self) {
        self.render_ticket.fetch_add(1, Ordering::Relaxed);
        self.processing_worker
            .ticket
            .fetch_add(1, Ordering::Relaxed);
    }
}
