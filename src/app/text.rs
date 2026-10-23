//! Transactional text placement. One bounded worker handles previews; tokens
//! reject obsolete parameter results. Commitment inserts the Fontdue raster;
//! egui supplies the interactive caret and text selection while editing.
use super::*;
mod box_ui;
pub(super) mod fonts;
mod formatting;
mod geometry;
mod rich;
use palette_core::text::{
    self, Alignment, MAX_TEXT_CHARACTERS, MAX_TEXT_DIMENSION, TextEngine, TextSpec,
};

pub(super) struct TextRequest {
    job: u64,
    spec: TextSpec,
    origin: [i32; 2],
    document: Document,
    angle: f64,
}

pub(super) struct TextResult {
    job: u64,
    origin: [i32; 2],
    result: Result<(Raster, ColorImage), String>,
}

/// Commands clicked while a text preview is pending resume on the same tab
/// after its pixels are committed, rather than silently losing the command.
pub(super) enum AfterText {
    New,
    Open,
    OpenPath(PathBuf),
    Import(Option<PathBuf>),
    Save(bool, bool),
    Close(u64),
    Tool(Tool),
    Brush(palette_core::brush::Style),
    Tab(u64),
    Edit(editing::Edit),
    Copy(super::clipboard::CopyTarget),
    Properties,
    ResizeDialog,
    RotateDialog,
    Filter(Filter),
    Undo,
    Redo,
    Quit,
}
#[derive(Clone, Copy)]
struct TextBoxDrag {
    edges: [i32; 2],
    from: egui::Pos2,
    origin: [i32; 2],
    size: [u32; 2],
    moving: bool,
    rotating: bool,
    angle: f64,
}
pub(super) struct TextEditor {
    session: u64,
    generation: u64,
    pub(super) spec: TextSpec,
    pub(super) origin: [i32; 2],
    requested: Option<(TextSpec, [i32; 2], f64)>,
    previewed: Option<(TextSpec, [i32; 2], f64)>,
    pub(super) angle: f64,
    pixels_origin: [i32; 2],
    screen_pointer: Option<egui::Pos2>,
    input_mapped: bool,
    body_dragging: bool,
    pub(super) overflow: bool,
    job: u64,
    pixels: Option<Raster>,
    texture: Option<TextureHandle>,
    error: Option<String>,
    focus: bool,
    commit_requested: bool,
    toolbar: egui::Rect,
    box_drag: Option<TextBoxDrag>,
    cursor: egui::text::CCursorRange,
    undo: Vec<rich::Snapshot>,
    redo: Vec<rich::Snapshot>,
    font_catalog_ready: bool,
}

impl TextEditor {
    pub(super) fn ready(&self) -> bool {
        self.previewed
            .as_ref()
            .is_some_and(|(spec, origin, angle)| {
                *spec == self.spec && *origin == self.origin && *angle == self.angle
            })
            && self.error.is_none()
    }
}

pub(super) fn worker() -> (mpsc::SyncSender<TextRequest>, mpsc::Receiver<TextResult>) {
    let (tx, requests) = mpsc::sync_channel::<TextRequest>(1);
    let (results, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(request) = requests.recv() {
            let mut origin = request.origin;
            let result = (|| {
                let library = fonts::Library::shared();
                let mut runs = request.spec.runs();
                let mut ids = Vec::new();
                let mut engines = Vec::new();
                for run in &mut runs {
                    let resolved =
                        library.resolve(request.spec.font, run.style.bold, run.style.italic);
                    let index = if let Some(index) = ids.iter().position(|id| *id == resolved.id) {
                        index
                    } else {
                        ids.push(resolved.id);
                        engines.push(library.engine(resolved.id)?);
                        ids.len() - 1
                    };
                    run.font = index;
                    run.style.bold = resolved.bold;
                    run.style.italic = resolved.italic;
                }
                // Empty drafts still need a valid metrics face.
                if engines.is_empty() {
                    engines.push(library.engine(request.spec.font)?);
                }
                let engine = TextEngine::combine(&engines);
                let mut spec = request.spec.clone();
                spec.font = 0;
                let raw = engine.render_runs(&spec, &runs)?;
                let (mut pixels, offset) =
                    palette_core::transform::rotate_raster(&raw, origin, request.angle)?;
                origin = offset;
                text::constrain(&mut pixels, &request.document, origin);
                let image = ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    &pixels.rgba(),
                );
                Ok((pixels, image))
            })();
            if results
                .send(TextResult {
                    job: request.job,
                    origin,
                    result,
                })
                .is_err()
            {
                break;
            }
        }
    });
    (tx, rx)
}

impl PaletteApp {
    pub(crate) fn start_text(&mut self, bounds: palette_core::editing::PixelRect) {
        if self.busy || self.edit_pending || self.text_editor.is_some() {
            return;
        }
        self.current_mut().end_gesture();
        self.stroke = None;
        self.processing_toolbar = None;
        self.text_editor = Some(TextEditor {
            session: self.current().id,
            generation: self.current().generation,
            spec: TextSpec {
                character_styles: Vec::new(),
                text: String::new(),
                font: 0,
                size: 32.0,
                line_height: 1.2,
                style: Default::default(),
                scroll_y: 0.,
                alignment: Alignment::Left,
                width: bounds.width.min(MAX_TEXT_DIMENSION),
                height: bounds.height.min(MAX_TEXT_DIMENSION),
                color: self.brush.color,
                opacity: self.brush.opacity,
                background: None,
            },
            origin: [bounds.x as i32, bounds.y as i32],
            pixels_origin: [bounds.x as i32, bounds.y as i32],
            angle: 0.,
            screen_pointer: None,
            input_mapped: false,
            body_dragging: false,
            overflow: false,
            requested: None,
            previewed: None,
            job: 0,
            pixels: None,
            texture: None,
            error: None,
            focus: true,
            commit_requested: false,
            toolbar: egui::Rect::NOTHING,
            box_drag: None,
            cursor: Default::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            font_catalog_ready: false,
        });
    }

    pub(super) fn poll_text(&mut self, ctx: &Context) {
        while let Ok(result) = self.text_rx.try_recv() {
            let Some(editor) = &mut self.text_editor else {
                continue;
            };
            if result.job != editor.job {
                continue;
            }
            match result.result {
                Ok((pixels, image)) => {
                    if let Some(texture) = &mut editor.texture {
                        texture.set(image, TextureOptions::NEAREST);
                    } else {
                        editor.texture =
                            Some(ctx.load_texture("text-preview", image, TextureOptions::NEAREST));
                    }
                    editor.pixels_origin = result.origin;
                    editor.pixels = Some(pixels);
                    editor.previewed = editor.requested.clone();
                    editor.error = None;
                }
                Err(error) => {
                    editor.error = Some(error);
                }
            }
        }
    }

    pub(super) fn request_text(&mut self, ctx: &Context) {
        let Some(editor) = &self.text_editor else {
            return;
        };
        if editor.box_drag.is_some() {
            ctx.request_repaint();
            return;
        }
        if editor
            .requested
            .as_ref()
            .is_some_and(|(spec, origin, angle)| {
                *spec == editor.spec && *origin == editor.origin && *angle == editor.angle
            })
        {
            if !editor.ready() && editor.error.is_none() {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
            return;
        }
        let request = TextRequest {
            job: self.next_text_job,
            spec: editor.spec.clone(),
            origin: editor.origin,
            document: self.current().document.clone(),
            angle: editor.angle,
        };
        if self.text_tx.try_send(request).is_ok() {
            let editor = self.text_editor.as_mut().expect("text draft");
            editor.job = self.next_text_job;
            self.next_text_job += 1;
            editor.requested = Some((editor.spec.clone(), editor.origin, editor.angle));
            editor.error = None;
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    pub(crate) fn apply_text(&mut self) {
        let Some(editor) = &self.text_editor else {
            return;
        };
        if !editor.ready() {
            return;
        }
        if editor.session != self.current().id || editor.generation != self.current().generation {
            self.text_editor.as_mut().expect("draft").error =
                Some("The document changed. Cancel and place the text again.".into());
            return;
        }
        let origin = editor.pixels_origin;
        let name = format!(
            "Text: {}",
            editor.spec.text.lines().next().unwrap_or_default()
        );
        let pixels = editor.pixels.clone().expect("ready preview");
        let mut document = self.current().document.clone();
        match text::insert(&mut document, pixels, origin, &name) {
            Ok(true) => self.current_mut().command("Text", |doc| *doc = document),
            Ok(false) => {}
            Err(error) => {
                self.text_editor.as_mut().expect("draft").error = Some(error);
                return;
            }
        }
        self.text_editor = None;
    }

    /// Commit before a document/tool action. If rasterization is pending, keep
    /// the draft alive and complete on its own result instead of dropping text.
    pub(crate) fn finish_inline_text(&mut self) -> bool {
        if let Some(editor) = &mut self.text_editor {
            editor.commit_requested = true;
        }
        self.finish_requested_text();
        self.text_editor.is_none()
    }
    pub(super) fn finish_requested_text(&mut self) {
        if self
            .text_editor
            .as_ref()
            .is_some_and(|e| e.commit_requested && e.ready())
        {
            self.apply_text();
        }
    }
    pub(super) fn resume_text_action(&mut self, ctx: &Context) {
        if self.text_editor.is_some() {
            return;
        }
        let Some(action) = self.after_text.take() else {
            return;
        };
        match action {
            AfterText::New => self.new_document(),
            AfterText::Open => self.open_dialog(ctx),
            AfterText::OpenPath(path) => self.open_path(path, ctx),
            AfterText::Import(path) => self.import_image(path, ctx),
            AfterText::Save(save_as, export) => self.save(ctx, save_as, export),
            AfterText::Close(id) => self.request_close(id, ctx),
            AfterText::Tool(tool) => self.choose_tool(tool),
            AfterText::Brush(style) => self.choose_brush(style),
            AfterText::Tab(id) => {
                if let Some(index) = self.sessions.iter().position(|s| s.id == id) {
                    self.active = index;
                }
            }
            AfterText::Edit(edit) => self.edit(edit, ctx),
            AfterText::Copy(target) => self.copy_pixels(target, ctx),
            AfterText::Properties => self.show_canvas_properties(),
            AfterText::ResizeDialog => self.show_resize(),
            AfterText::RotateDialog => self.show_rotation(),
            AfterText::Filter(filter) => self.add_filter(filter),
            AfterText::Undo => self.undo_current(),
            AfterText::Redo => self.redo_current(),
            AfterText::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
    pub(crate) fn choose_brush(&mut self, style: palette_core::brush::Style) {
        if !self.finish_inline_text() {
            self.after_text = Some(AfterText::Brush(style));
            return;
        }
        self.choose_tool(Tool::Brush);
        self.brush_sizes[self.brush_style as usize] = self.brush.diameter;
        self.brush_style = style;
        self.brush.diameter = self.brush_sizes[style as usize];
        self.brush_size_preview_until = None;
    }

    pub(crate) fn choose_tool(&mut self, tool: crate::session::Tool) {
        self.cancel_processing();
        self.processing_toolbar = None;
        if tool != crate::session::Tool::Text && !self.finish_inline_text() {
            self.after_text = Some(AfterText::Tool(tool));
            return;
        }
        if tool != self.tool {
            if !matches!(tool, Tool::Select | Tool::Move) {
                // Another editing tool accepts the shape but retains the
                // selection as its clipping region, as ordinary Paint tools do.
                self.current_mut().document.floating_shape = None;
            }
            if self.tool == Tool::Brush {
                self.brush_sizes[self.brush_style as usize] = self.brush.diameter;
                self.brush.diameter = self.other_tool_diameter;
            } else if tool == Tool::Brush {
                self.other_tool_diameter = self.brush.diameter;
                self.brush.diameter = self.brush_sizes[self.brush_style as usize];
            }
            self.brush_size_preview_until = None;
            self.current_mut().end_gesture();
            self.stroke = None;
            if tool == Tool::SelectBrush
                && self.current().document.selection.is_some()
                && self.current().selection_mode == palette_core::selection::Combine::Replace
            {
                self.current_mut().selection_mode = palette_core::selection::Combine::Add;
            }
        }
        self.tool = tool;
    }
    pub(super) fn text_window(&mut self, ctx: &Context) {
        let Some(mut editor) = self.text_editor.take() else {
            return;
        };
        if editor.session != self.current().id {
            self.text_editor = Some(editor);
            return;
        }
        let zoom = self.current().zoom;
        let canvas = self.image_rect;
        let rect = egui::Rect::from_min_size(
            canvas.min + vec2(editor.origin[0] as f32, editor.origin[1] as f32) * zoom,
            vec2(editor.spec.width as f32, editor.spec.height as f32) * zoom,
        );
        let popup_open = self.interface.popup_was_open || egui::Popup::is_any_open(ctx);
        let modal_open =
            self.interface.color_editor.is_some() || self.interface.canvas_properties.is_some();
        rich::normalize(&mut editor.spec);
        let before = rich::Snapshot::of(&editor);
        let history_action = !popup_open && !modal_open && rich::history(ctx, &mut editor);
        if !popup_open && !modal_open {
            formatting::shortcuts(ctx, &mut editor);
        }
        let outside = ctx.input(|i| {
            i.pointer.primary_pressed()
                && i.pointer.interact_pos().is_some_and(|point| {
                    let point = if editor.input_mapped {
                        editor.screen_pointer.unwrap_or(point)
                    } else {
                        point
                    };
                    point.y >= self.canvas_rect.top()
                        && !geometry::BoxGeometry::new(rect, editor.angle).near(point)
                        && !editor.toolbar.contains(point)
                })
        });
        if outside && !editor.focus && !popup_open && !modal_open {
            editor.commit_requested = true;
        }
        editor.spec.color = self.brush.color;
        if editor.spec.background.is_some() {
            editor.spec.background = Some(self.background.to_srgba_unmultiplied());
        }
        let mut apply = false;
        let mut cancel = false;
        let toolbar = egui::Area::new(egui::Id::unique("text-toolbar"))
            .anchor(
                egui::Align2::CENTER_TOP,
                vec2(0.0, self.canvas_rect.top() + 4.0),
            )
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 32.0).min(980.0));
                crate::interface::context_toolbar_frame(ui.style()).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.set_min_height(crate::interface::CONTEXT_CONTROL_HEIGHT);
                        let before = editor.spec.font;
                        self.text_fonts.show(ui, &mut editor.spec.font);
                        if before != editor.spec.font {
                            let (bold, italic) = fonts::Library::shared().traits(editor.spec.font);
                            editor.spec.style.bold = bold;
                            editor.spec.style.italic = italic;
                            for style in &mut editor.spec.character_styles {
                                style.bold = bold;
                                style.italic = italic;
                            }
                        }
                        ui.add(
                            egui::DragValue::new(&mut editor.spec.size)
                                .range(6.0..=256.0)
                                .suffix(" px"),
                        )
                        .on_hover_text("Font size in image pixels");
                        ui.separator();
                        if formatting::buttons(ui, &mut editor) {
                            editor.focus = true;
                        }
                        ui.separator();
                        for (label, alignment) in [
                            ("Align left", Alignment::Left),
                            ("Align center", Alignment::Center),
                            ("Align right", Alignment::Right),
                        ] {
                            let response = icons::button(
                                ui,
                                Icon::TextAlign(alignment),
                                label,
                                editor.spec.alignment == alignment,
                                true,
                                vec2(30., 28.),
                            );
                            ui.interact(
                                response.rect,
                                egui::Id::unique(("text-alignment", alignment as u8)),
                                Sense::hover(),
                            );
                            if response.clicked() {
                                editor.spec.alignment = alignment;
                            }
                        }
                        ui.separator();
                        let mut opaque = editor.spec.background.is_some();
                        if ui.checkbox(&mut opaque, "Background fill").changed() {
                            editor.spec.background =
                                opaque.then_some(self.background.to_srgba_unmultiplied());
                        }
                        ui.menu_button("More", |ui| {
                            ui.label("Line spacing");
                            ui.add(
                                egui::DragValue::new(&mut editor.spec.line_height)
                                    .range(1.0..=3.0)
                                    .speed(0.05),
                            );
                            ui.add(
                                egui::Slider::new(&mut editor.spec.opacity, 0.0..=1.0)
                                    .text("Opacity"),
                            );
                            ui.label("Rotation");
                            ui.add(
                                egui::DragValue::new(&mut editor.angle)
                                    .range(-180.0..=180.0)
                                    .suffix("°"),
                            );
                            ui.label(
                                "Only text visible inside the box is committed as a raster layer.",
                            );
                        });
                        ui.separator();
                        apply = ui
                            .add_enabled(editor.ready(), egui::Button::new("Done"))
                            .on_hover_text(
                                "Commit text (Ctrl+Enter), or click outside the text box",
                            )
                            .clicked();
                        cancel = ui
                            .button("Cancel")
                            .on_hover_text("Discard text (Escape)")
                            .clicked();
                    });
                });
            });
        editor.toolbar = toolbar.response.rect;
        let families = rich::families(&mut self.text_fonts, &editor.spec, ctx);
        let catalog_ready = fonts::Library::shared().ready();
        if catalog_ready != editor.font_catalog_ready {
            editor.font_catalog_ready = catalog_ready;
            editor.requested = None;
            editor.previewed = None;
        }
        self.text_box_ui(ctx, &mut editor, families, popup_open || modal_open);
        if !history_action {
            rich::record(&mut editor, before);
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::unique("text-boundary"),
        ));
        if !popup_open && !modal_open {
            cancel |= ctx.input(|i| i.key_pressed(Key::Escape));
            editor.commit_requested |=
                ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Enter));
        }
        if let Some(error) = &editor.error {
            painter.text(
                rect.left_bottom() + vec2(0.0, 12.0),
                egui::Align2::LEFT_TOP,
                error,
                egui::FontId::proportional(12.0),
                Color32::LIGHT_RED,
            );
        }
        if cancel {
            self.after_text = None;
        }
        if !cancel {
            editor.commit_requested |= apply;
            self.text_editor = Some(editor);
            self.finish_requested_text();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obsolete_text_jobs_cannot_replace_new_parameters_or_a_new_draft() {
        let ctx = Context::default();
        let mut app = PaletteApp::with_context(&ctx);
        let (requests, receive_request) = mpsc::sync_channel(1);
        let (results, receive_result) = mpsc::channel();
        app.text_tx = requests;
        app.text_rx = receive_result;
        let bounds = palette_core::editing::PixelRect {
            x: 10,
            y: 10,
            width: 200,
            height: 100,
        };
        app.start_text(bounds);
        app.text_editor.as_mut().unwrap().spec.text = "Old".into();
        app.request_text(&ctx);
        let old: TextRequest = receive_request.recv().unwrap();
        app.text_editor.as_mut().unwrap().spec.text = "New".into();
        app.request_text(&ctx);
        let new: TextRequest = receive_request.recv().unwrap();
        let send = |job| {
            let pixels = Raster::new(200, 100, [0; 4]).unwrap();
            results
                .send(TextResult {
                    job,
                    origin: [10, 10],
                    result: Ok((pixels.clone(), texture_image(&pixels, 16384))),
                })
                .unwrap();
        };
        send(old.job);
        app.poll_text(&ctx);
        assert!(!app.text_editor.as_ref().unwrap().ready());
        send(new.job);
        app.poll_text(&ctx);
        assert!(app.text_editor.as_ref().unwrap().ready());
        app.text_editor = None;
        app.start_text(bounds);
        send(new.job);
        app.poll_text(&ctx);
        assert!(!app.text_editor.as_ref().unwrap().ready());
        assert!(app.text_editor.as_ref().unwrap().pixels.is_none());
    }
}
