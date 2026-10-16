//! Transactional text placement. One bounded worker handles previews; tokens
//! reject obsolete parameter results. Commitment inserts the Fontdue raster;
//! egui supplies the interactive caret and text selection while editing.
use super::*;
use palette_core::text::{
    self, Alignment, MAX_TEXT_CHARACTERS, MAX_TEXT_DIMENSION, TextEngine, TextSpec,
};

pub(super) struct TextRequest {
    job: u64,
    spec: TextSpec,
    origin: [i32; 2],
    document: Document,
}

pub(super) struct TextResult {
    job: u64,
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
    Tab(u64),
    Edit(editing::Edit),
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
}
pub(super) struct TextEditor {
    session: u64,
    generation: u64,
    pub(super) spec: TextSpec,
    pub(super) origin: [i32; 2],
    requested: Option<(TextSpec, [i32; 2])>,
    previewed: Option<(TextSpec, [i32; 2])>,
    job: u64,
    pixels: Option<Raster>,
    texture: Option<TextureHandle>,
    error: Option<String>,
    focus: bool,
    commit_requested: bool,
    toolbar: egui::Rect,
    box_drag: Option<TextBoxDrag>,
}

impl TextEditor {
    pub(super) fn ready(&self) -> bool {
        self.previewed
            .as_ref()
            .is_some_and(|(spec, origin)| *spec == self.spec && *origin == self.origin)
            && self.error.is_none()
    }
}

pub(super) fn worker() -> (mpsc::SyncSender<TextRequest>, mpsc::Receiver<TextResult>) {
    let (tx, requests) = mpsc::sync_channel::<TextRequest>(1);
    let (results, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let engine = TextEngine::new(&[
            epaint_default_fonts::UBUNTU_LIGHT,
            epaint_default_fonts::HACK_REGULAR,
        ]);
        while let Ok(request) = requests.recv() {
            let result = (|| {
                let mut pixels = engine
                    .as_ref()
                    .map_err(Clone::clone)?
                    .render(&request.spec)?;
                text::constrain(&mut pixels, &request.document, request.origin);
                let image = ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    &pixels.rgba(),
                );
                Ok((pixels, image))
            })();
            if results
                .send(TextResult {
                    job: request.job,
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
        self.show_filters = false;
        self.text_editor = Some(TextEditor {
            session: self.current().id,
            generation: self.current().generation,
            spec: TextSpec {
                text: String::new(),
                font: 0,
                size: 32.0,
                line_height: 1.2,
                alignment: Alignment::Left,
                width: bounds.width.min(MAX_TEXT_DIMENSION),
                height: bounds.height.min(MAX_TEXT_DIMENSION),
                color: self.brush.color,
                opacity: self.brush.opacity,
                background: None,
            },
            origin: [bounds.x as i32, bounds.y as i32],
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
        if editor
            .requested
            .as_ref()
            .is_some_and(|(spec, origin)| *spec == editor.spec && *origin == editor.origin)
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
        };
        if self.text_tx.try_send(request).is_ok() {
            let editor = self.text_editor.as_mut().expect("text draft");
            editor.job = self.next_text_job;
            self.next_text_job += 1;
            editor.requested = Some((editor.spec.clone(), editor.origin));
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
        let origin = editor.origin;
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
            AfterText::Tab(id) => {
                if let Some(index) = self.sessions.iter().position(|s| s.id == id) {
                    self.active = index;
                }
            }
            AfterText::Edit(edit) => self.edit(edit, ctx),
            AfterText::Properties => self.show_canvas_properties(),
            AfterText::ResizeDialog => self.show_resize(),
            AfterText::RotateDialog => self.show_rotation(),
            AfterText::Filter(filter) => self.add_filter(filter),
            AfterText::Undo => self.undo_current(),
            AfterText::Redo => self.redo_current(),
            AfterText::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
    pub(crate) fn choose_tool(&mut self, tool: crate::session::Tool) {
        if tool != crate::session::Tool::Text && !self.finish_inline_text() {
            self.after_text = Some(AfterText::Tool(tool));
            return;
        }
        if tool != self.tool {
            self.current_mut().end_gesture();
            self.stroke = None;
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
        let outside = ctx.input(|i| {
            i.pointer.primary_pressed()
                && i.pointer.interact_pos().is_some_and(|point| {
                    point.y >= self.canvas_rect.top()
                        && !rect.expand(8.0).contains(point)
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
                egui::Frame::popup(ui.style())
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            egui::ComboBox::from_id_salt("text-font")
                                .width(145.0)
                                .selected_text(if editor.spec.font == 0 {
                                    "Ubuntu Light"
                                } else {
                                    "Hack"
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut editor.spec.font, 0, "Ubuntu Light");
                                    ui.selectable_value(&mut editor.spec.font, 1, "Hack");
                                });
                            ui.add(
                                egui::DragValue::new(&mut editor.spec.size)
                                    .range(6.0..=256.0)
                                    .suffix(" px"),
                            )
                            .on_hover_text("Font size in image pixels");
                            ui.separator();
                            for label in ["B", "I", "U", "S"] {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(label).min_size(vec2(26.0, 28.0)),
                                )
                                .on_hover_text("This text style is not implemented yet");
                            }
                            ui.separator();
                            for (label, alignment) in [
                                ("Left", Alignment::Left),
                                ("Center", Alignment::Center),
                                ("Right", Alignment::Right),
                            ] {
                                ui.selectable_value(&mut editor.spec.alignment, alignment, label);
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
                                ui.label("Text is committed as a separate raster layer.");
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
        let family = egui::FontFamily::Name(
            if editor.spec.font == 0 {
                "Palette Sans"
            } else {
                "Palette Mono"
            }
            .into(),
        );
        let text_color = Color32::from_rgba_unmultiplied(
            editor.spec.color[0],
            editor.spec.color[1],
            editor.spec.color[2],
            (f32::from(editor.spec.color[3]) * editor.spec.opacity).round() as u8,
        );
        egui::Area::new(egui::Id::unique("inline-text-box"))
            .fixed_pos(rect.min)
            .order(egui::Order::Middle)
            .movable(false)
            .constrain(false)
            .show(ctx, |ui| {
                ui.set_clip_rect(rect.intersect(self.canvas_rect).intersect(canvas));
                ui.spacing_mut().extra_text_line_spacing =
                    (editor.spec.line_height - 1.0) * editor.spec.size * zoom;
                if let Some(color) = editor.spec.background {
                    ui.painter().rect_filled(
                        rect,
                        0,
                        Color32::from_rgba_unmultiplied(
                            color[0],
                            color[1],
                            color[2],
                            (f32::from(color[3]) * editor.spec.opacity).round() as u8,
                        ),
                    );
                }
                let response = ui.add_sized(
                    rect.size(),
                    egui::TextEdit::multiline(&mut editor.spec.text)
                        .id_salt((editor.session, "text-content"))
                        .char_limit(MAX_TEXT_CHARACTERS)
                        .font(egui::FontId::new(editor.spec.size * zoom, family))
                        .text_color(text_color)
                        .frame(egui::Frame::NONE)
                        .margin(0.0)
                        .desired_width(rect.width())
                        .align(match editor.spec.alignment {
                            Alignment::Left => egui::Align2::LEFT_TOP,
                            Alignment::Center => egui::Align2::CENTER_TOP,
                            Alignment::Right => egui::Align2::RIGHT_TOP,
                        }),
                );
                if editor.focus {
                    response.request_focus();
                    editor.focus = false;
                }
            });
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::unique("text-boundary"),
        ));
        let outline = rect.intersect(self.canvas_rect);
        painter.rect_stroke(
            outline,
            0,
            egui::Stroke::new(1.0, Color32::from_gray(180)),
            egui::StrokeKind::Outside,
        );
        let handles = [
            (rect.left_top(), [-1, -1]),
            (rect.center_top(), [0, -1]),
            (rect.right_top(), [1, -1]),
            (rect.left_center(), [-1, 0]),
            (rect.right_center(), [1, 0]),
            (rect.left_bottom(), [-1, 1]),
            (rect.center_bottom(), [0, 1]),
            (rect.right_bottom(), [1, 1]),
        ];
        for (index, (point, edges)) in handles.into_iter().enumerate() {
            if !self.canvas_rect.contains(point) {
                continue;
            }
            let handle = egui::Rect::from_center_size(point, vec2(5.0, 5.0));
            painter.rect_filled(handle, 0, Color32::WHITE);
            painter.rect_stroke(
                handle,
                0,
                egui::Stroke::new(0.75, Color32::BLACK),
                egui::StrokeKind::Inside,
            );
            egui::Area::new(egui::Id::unique(("text-handle", index)))
                .fixed_pos(point - vec2(5.0, 5.0))
                .order(egui::Order::Middle)
                .movable(false)
                .show(ctx, |ui| {
                    let cursor = if edges[0] == 0 {
                        egui::CursorIcon::ResizeVertical
                    } else if edges[1] == 0 {
                        egui::CursorIcon::ResizeHorizontal
                    } else if edges[0] == edges[1] {
                        egui::CursorIcon::ResizeNwSe
                    } else {
                        egui::CursorIcon::ResizeNeSw
                    };
                    let response = ui
                        .allocate_response(vec2(10.0, 10.0), Sense::drag())
                        .on_hover_cursor(cursor);
                    if response.drag_started() {
                        editor.box_drag = Some(TextBoxDrag {
                            edges,
                            from: ctx.input(|i| i.pointer.press_origin()).unwrap_or(point),
                            origin: editor.origin,
                            size: [editor.spec.width, editor.spec.height],
                            moving: false,
                        });
                    }
                });
        }
        egui::Area::new(egui::Id::unique("text-move-edge"))
            .fixed_pos(rect.left_top() - vec2(0.0, 4.0))
            .order(egui::Order::Middle)
            .movable(false)
            .show(ctx, |ui| {
                let response = ui
                    .allocate_response(vec2(rect.width(), 4.0), Sense::drag())
                    .on_hover_cursor(egui::CursorIcon::Move);
                if response.drag_started() {
                    editor.box_drag = Some(TextBoxDrag {
                        edges: [0, 0],
                        from: ctx
                            .input(|i| i.pointer.press_origin())
                            .unwrap_or(rect.left_top()),
                        origin: editor.origin,
                        size: [editor.spec.width, editor.spec.height],
                        moving: true,
                    });
                }
            });
        if let Some(drag) = editor.box_drag {
            if let Some(point) = ctx.input(|i| i.pointer.interact_pos()) {
                let delta = (point - drag.from) / zoom;
                let mut origin = drag.origin;
                let mut size = drag.size;
                let limits = [
                    self.current().document.width,
                    self.current().document.height,
                ];
                for axis in 0..2 {
                    let change = if axis == 0 {
                        delta.x.round() as i32
                    } else {
                        delta.y.round() as i32
                    };
                    if drag.moving {
                        origin[axis] = (drag.origin[axis] + change)
                            .clamp(0, (limits[axis] - size[axis]) as i32);
                    } else if drag.edges[axis] < 0 {
                        let end = drag.origin[axis] + drag.size[axis] as i32;
                        origin[axis] = (drag.origin[axis] + change)
                            .clamp((end - MAX_TEXT_DIMENSION as i32).max(0), end - 1);
                        size[axis] = (end - origin[axis]) as u32;
                    } else if drag.edges[axis] > 0 {
                        size[axis] = (drag.size[axis] as i32 + change).clamp(
                            1,
                            (limits[axis] - origin[axis] as u32).min(MAX_TEXT_DIMENSION) as i32,
                        ) as u32;
                    }
                }
                editor.origin = origin;
                editor.spec.width = size[0];
                editor.spec.height = size[1];
            }
            if !ctx.input(|i| i.pointer.primary_down()) {
                editor.box_drag = None;
            }
        }
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
