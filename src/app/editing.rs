//! UI command routing. Expensive edits run on a snapshot and commit only if the
//! originating tab is still at the same generation; stale results cannot win.
use super::*;
use palette_core::{
    editing,
    transform::{self, Orthogonal, Resample},
};
use std::borrow::Cow;

pub(super) struct EditResult {
    id: u64,
    generation: u64,
    label: &'static str,
    result: Result<Option<Document>, String>,
    select_tool: Option<Tool>,
}

/// Image/selection edits available from menus, canvas and shortcuts.
pub enum Edit {
    Fill {
        point: [u32; 2],
        color: [u8; 4],
        opacity: f32,
        tolerance: u8,
    },
    Clear,
    Crop,
    Move([i32; 2]),
    Transform(Orthogonal),
    Rotate(f64),
    Path(palette_core::shapes::Path, palette_core::shapes::Style),
    Resize(u32, u32, Resample),
    Canvas(u32, u32),
    Shape(
        palette_core::shapes::Kind,
        [f32; 2],
        [f32; 2],
        palette_core::shapes::Style,
    ),
}

impl PaletteApp {
    pub fn undo_current(&mut self) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Undo);
            return;
        }
        self.stroke = None;
        self.current_mut().end_gesture();
        self.current_mut().undo();
    }
    pub fn redo_current(&mut self) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Redo);
            return;
        }
        self.stroke = None;
        self.current_mut().end_gesture();
        self.current_mut().redo();
    }
    pub(super) fn edit_shortcuts(&mut self, ctx: &Context) {
        if !self.close_pending_none()
            || self.interface.popup_was_open
            || egui::Popup::is_any_open(ctx)
        {
            return;
        }
        if self.current().construction.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Copy => self.copy_selection(false, ctx),
                egui::Event::Cut => self.copy_selection(true, ctx),
                egui::Event::PasteImage(image) => {
                    let bytes: Vec<u8> = image
                        .pixels
                        .iter()
                        .flat_map(|p| p.to_srgba_unmultiplied())
                        .collect();
                    let result = u32::try_from(image.size[0])
                        .ok()
                        .zip(u32::try_from(image.size[1]).ok())
                        .ok_or_else(|| "Clipboard dimensions overflow".to_string())
                        .and_then(|(w, h)| Raster::from_rgba(w, h, &bytes))
                        .and_then(|r| self.paste_pixels(r));
                    if let Err(error) = result {
                        self.error = Some(error);
                    }
                }
                egui::Event::Paste(_) => self.paste_clipboard(),
                _ => {}
            }
        }
        let (command, shift, a, d, c, x, v, delete, escape, arrows) = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.key_pressed(Key::A),
                i.key_pressed(Key::D),
                i.key_pressed(Key::C),
                i.key_pressed(Key::X),
                i.key_pressed(Key::V),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Escape),
                [
                    i.key_pressed(Key::ArrowRight) as i32 - i.key_pressed(Key::ArrowLeft) as i32,
                    i.key_pressed(Key::ArrowDown) as i32 - i.key_pressed(Key::ArrowUp) as i32,
                ],
            )
        });
        if command && a {
            self.select_all();
        }
        if command && d {
            self.deselect();
        }
        if command && c {
            self.copy_selection(false, ctx);
        }
        if command && x {
            self.copy_selection(true, ctx);
        }
        if command && v {
            self.paste_clipboard();
        }
        if delete && !command {
            self.edit(Edit::Clear, ctx);
        }
        if escape && !self.interface.popup_was_open {
            let cancelled_shape = self.stroke.take().is_some() && self.tool.shape();
            if !self.current_mut().cancel_selection_transform()
                && !self.current_mut().cancel_layer_move()
                && !cancelled_shape
                && self.tool != Tool::MoveLayer
            {
                self.deselect();
            }
        }
        if !command && arrows != [0, 0] && self.tool == Tool::MoveLayer {
            self.current_mut().end_gesture();
            let delta = arrows.map(|v| v * if shift { 10 } else { 1 });
            let mut document = self.current().document.clone();
            match editing::translate_layer(&mut document, delta) {
                Ok(true) => self
                    .current_mut()
                    .command("Nudge layer", |doc| *doc = document),
                Ok(false) => {}
                Err(error) => self.error = Some(error),
            }
            return;
        }
        if !command && arrows != [0, 0] && self.current().document.selection.is_some() {
            self.edit(
                Edit::Move(arrows.map(|v| v * if shift { 10 } else { 1 })),
                ctx,
            );
        }
    }
    pub fn edit(&mut self, operation: Edit, ctx: &Context) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Edit(operation));
            return;
        }
        if self.edit_pending {
            return;
        }
        self.stroke = None;
        self.current_mut().end_gesture();
        let session = self.current();
        let (id, generation) = (session.id, session.generation);
        let mut document = session.document.clone();
        let select_tool =
            matches!(operation, Edit::Shape(..) | Edit::Path(..)).then_some(self.tool);
        let label = match operation {
            Edit::Fill { .. } => "Fill",
            Edit::Clear => "Clear selection",
            Edit::Crop => "Crop",
            Edit::Move(_) => "Move selection",
            Edit::Transform(_) => {
                if document.selection.is_some() {
                    "Transform selection"
                } else {
                    "Transform image"
                }
            }
            Edit::Rotate(_) => "Rotate custom",
            Edit::Path(ref path, _) => {
                if path.closed {
                    "Polygon"
                } else {
                    "Curve"
                }
            }
            Edit::Resize(..) => "Resize image",
            Edit::Canvas(..) => "Canvas properties",
            Edit::Shape(kind, ..) => kind.label(),
        };
        let sender = self.edit_tx.clone();
        let repaint = ctx.clone();
        self.edit_pending = true;
        self.message = format!("{label}…");
        std::thread::spawn(move || {
            let result = (|| {
                let changed = match operation {
                    Edit::Fill {
                        point,
                        color,
                        opacity,
                        tolerance,
                    } => editing::flood_fill(&mut document, point, color, opacity, tolerance)?,
                    Edit::Shape(kind, from, to, style) => palette_core::shapes::paint_selected(
                        &mut document,
                        &palette_core::shapes::path(kind, from, to),
                        style,
                    )?,
                    Edit::Path(path, style) => {
                        palette_core::shapes::paint_selected(&mut document, &path, style)?
                    }
                    Edit::Rotate(degrees) => transform::rotate(&mut document, degrees)?,
                    Edit::Clear => editing::clear(&mut document),
                    Edit::Crop => transform::crop(&mut document)?,
                    Edit::Move(delta) => editing::move_selection(&mut document, delta)?,
                    Edit::Transform(operation) => {
                        if document.selection.is_some() {
                            transform::selected_orthogonal(&mut document, operation)?
                        } else {
                            transform::orthogonal(&mut document, operation)?;
                            true
                        }
                    }
                    Edit::Canvas(width, height) => {
                        if width == document.width && height == document.height {
                            false
                        } else {
                            transform::canvas_resize(&mut document, width, height)?;
                            true
                        }
                    }
                    Edit::Resize(width, height, method) => {
                        if width == document.width && height == document.height {
                            false
                        } else {
                            transform::resize(&mut document, width, height, method)?;
                            true
                        }
                    }
                };
                document.validate()?;
                Ok(changed.then_some(document))
            })();
            let _ = sender.send(EditResult {
                id,
                generation,
                label,
                result,
                select_tool,
            });
            repaint.request_repaint();
        });
    }

    pub(super) fn poll_edits(&mut self) {
        while let Ok(result) = self.edit_rx.try_recv() {
            self.edit_pending = false;
            let Some(session) = self
                .sessions
                .iter_mut()
                .find(|s| s.id == result.id && s.generation == result.generation)
            else {
                self.message = "Edit cancelled because its document changed or closed".into();
                continue;
            };
            match result.result {
                Ok(Some(document)) => {
                    session.command(result.label, |current| *current = document);
                    self.message = result.label.into();
                    if self.current().id == result.id && result.select_tool == Some(self.tool) {
                        self.tool = Tool::Select;
                    }
                }
                Ok(None) => self.message = "No pixels changed".into(),
                Err(error) => self.error = Some(error),
            }
        }
    }

    pub fn select_all(&mut self) {
        self.current_mut().command("Select all", |doc| {
            doc.selection = Some(editing::rectangle(
                doc.width,
                doc.height,
                editing::PixelRect {
                    x: 0,
                    y: 0,
                    width: doc.width,
                    height: doc.height,
                },
            ))
        });
    }
    pub fn deselect(&mut self) {
        self.current_mut().selection_drag = None;
        if self.current().document.selection.is_some() {
            self.current_mut()
                .command("Deselect", |doc| doc.selection = None);
        }
    }
    pub fn invert_selection(&mut self) {
        self.current_mut().command("Invert selection", |doc| {
            let mut mask = doc.selection.take().unwrap_or_else(|| {
                editing::rectangle(
                    doc.width,
                    doc.height,
                    editing::PixelRect {
                        x: 0,
                        y: 0,
                        width: doc.width,
                        height: doc.height,
                    },
                )
            });
            for value in std::sync::Arc::make_mut(&mut mask.values) {
                *value = 255 - *value;
            }
            mask.enabled = true;
            doc.selection = Some(mask);
        });
    }

    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, String> {
        if self.clipboard.is_none() {
            self.clipboard = Some(
                arboard::Clipboard::new()
                    .map_err(|error| format!("Cannot access the system clipboard: {error}"))?,
            );
        }
        Ok(self.clipboard.as_mut().expect("initialized clipboard"))
    }
    pub fn copy_selection(&mut self, cut: bool, ctx: &Context) {
        if cut && self.current().document.active().locked {
            return;
        }
        let result = (|| {
            let raster = editing::copy(&self.current().document)?;
            self.clipboard()?
                .set_image(arboard::ImageData {
                    width: raster.width() as usize,
                    height: raster.height() as usize,
                    bytes: Cow::Owned(raster.rgba()),
                })
                .map_err(|error| format!("Could not copy image: {error}"))
        })();
        match result {
            Ok(()) => {
                if cut {
                    self.edit(Edit::Clear, ctx);
                } else {
                    self.message = "Copied active-layer selection to the system clipboard".into();
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub fn paste_clipboard(&mut self) {
        let result = (|| {
            let image = self.clipboard()?.get_image().map_err(|error| {
                format!("The clipboard does not contain a readable image: {error}")
            })?;
            let width = u32::try_from(image.width).map_err(|_| "Clipboard image is too wide")?;
            let height = u32::try_from(image.height).map_err(|_| "Clipboard image is too tall")?;
            let pixels = Raster::from_rgba(width, height, image.bytes.as_ref())?;
            self.paste_pixels(pixels)
        })();
        if let Err(error) = result {
            self.error = Some(error);
        }
    }
    pub(super) fn paste_pixels(&mut self, pixels: Raster) -> Result<(), String> {
        let mut document = self.current().document.clone();
        editing::paste(&mut document, pixels)?;
        self.current_mut()
            .command("Paste image", |doc| *doc = document);
        self.tool = Tool::Select;
        self.message = "Pasted as a new layer. Drag the selection to move it.".into();
        Ok(())
    }

    pub fn show_resize(&mut self) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::ResizeDialog);
            return;
        }
        self.current_mut().end_gesture();
        let session = self.current();
        self.resize_dialog = Some(ResizeDialog {
            session: session.id,
            generation: session.generation,
            width: session.document.width,
            height: session.document.height,
            original: [session.document.width, session.document.height],
            linked: true,
            method: Resample::Bilinear,
        });
    }
    pub(super) fn resize_window(&mut self, ctx: &Context) {
        let Some(mut settings) = self.resize_dialog.take() else {
            return;
        };
        if self.current().id != settings.session || self.current().generation != settings.generation
        {
            return;
        }
        let mut apply = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("resize-image")).show(ctx, |ui| {
            ui.heading("Resize image");
            ui.label("Resample all layers and masks. Undo restores the original pixels.");
            ui.horizontal(|ui| {
                ui.label("Width");
                if ui
                    .add(
                        egui::DragValue::new(&mut settings.width)
                            .range(1..=16384)
                            .suffix(" px"),
                    )
                    .changed()
                    && settings.linked
                {
                    settings.height = ((f64::from(settings.width) * f64::from(settings.original[1])
                        / f64::from(settings.original[0]))
                    .round() as u32)
                        .max(1);
                }
                ui.label("Height");
                if ui
                    .add(
                        egui::DragValue::new(&mut settings.height)
                            .range(1..=16384)
                            .suffix(" px"),
                    )
                    .changed()
                    && settings.linked
                {
                    settings.width = ((f64::from(settings.height) * f64::from(settings.original[0])
                        / f64::from(settings.original[1]))
                    .round() as u32)
                        .max(1);
                }
            });
            ui.checkbox(&mut settings.linked, "Maintain aspect ratio");
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut settings.method,
                    Resample::Nearest,
                    "Nearest (pixel art)",
                );
                ui.selectable_value(
                    &mut settings.method,
                    Resample::Bilinear,
                    "Smooth (bilinear)",
                );
            });
            ui.horizontal(|ui| {
                apply = ui.button("Resize").clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if apply {
            self.edit(
                Edit::Resize(settings.width, settings.height, settings.method),
                ctx,
            );
        } else if !cancel {
            self.resize_dialog = Some(settings);
        }
    }
}

pub(super) struct ResizeDialog {
    session: u64,
    generation: u64,
    width: u32,
    height: u32,
    original: [u32; 2],
    linked: bool,
    method: Resample,
}

impl PaletteApp {
    pub(crate) fn show_rotation(&mut self) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::RotateDialog);
            return;
        }
        self.current_mut().end_gesture();
        self.rotation_dialog = Some(RotationDialog {
            session: self.current().id,
            generation: self.current().generation,
            degrees: "0".into(),
            focus: true,
        });
    }
    pub(super) fn rotation_window(&mut self, ctx: &Context) {
        let Some(mut draft) = self.rotation_dialog.take() else {
            return;
        };
        if draft.session != self.current().id || draft.generation != self.current().generation {
            return;
        }
        let angle = draft
            .degrees
            .trim()
            .trim_end_matches('°')
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|d| d.is_finite() && d.abs() <= 360000.);
        let mut apply = false;
        let mut cancel = false;
        let result = egui::Modal::new(egui::Id::unique("rotate-custom"))
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24))
            .backdrop_color(Color32::from_black_alpha(18))
            .show(ctx, |ui| {
                ui.set_width(270.);
                ui.heading("Rotate custom");
                ui.add_space(16.);
                let input = ui.add_sized(
                    vec2(270., 32.),
                    egui::TextEdit::singleline(&mut draft.degrees).hint_text("Angle in degrees"),
                );
                if draft.focus {
                    input.request_focus();
                    let mut state =
                        egui::text_edit::TextEditState::load(ctx, input.id).unwrap_or_default();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::select_all_str(
                            &draft.degrees,
                        )));
                    state.store(ctx, input.id);
                    draft.focus = false;
                }
                input.on_hover_text("Positive angles rotate clockwise");
                ui.add_space(24.);
                ui.separator();
                ui.add_space(16.);
                ui.horizontal(|ui| {
                    apply = ui
                        .add_enabled(
                            angle.is_some(),
                            egui::Button::new(egui::RichText::new("OK").color(Color32::BLACK))
                                .fill(ACCENT)
                                .min_size(vec2(131., 32.)),
                        )
                        .clicked();
                    cancel = ui
                        .add_sized(vec2(131., 32.), egui::Button::new("Cancel"))
                        .clicked();
                });
            });
        if result.is_top_modal {
            apply |= angle.is_some() && ctx.input(|i| i.key_pressed(Key::Enter));
            cancel |= ctx.input(|i| i.key_pressed(Key::Escape));
        }
        // Parse again after input so Enter/OK uses the current text.
        let current = draft
            .degrees
            .trim()
            .trim_end_matches('°')
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|d| d.is_finite() && d.abs() <= 360000.);
        if !cancel {
            if apply && let Some(degrees) = current {
                self.edit(Edit::Rotate(degrees), ctx);
            } else {
                self.rotation_dialog = Some(draft);
            }
        }
    }
}
