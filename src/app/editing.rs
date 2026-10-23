//! UI command routing. Expensive edits run on a snapshot and commit only if the
//! originating tab is still at the same generation; stale results cannot win.
use super::*;
use palette_core::{
    editing,
    transform::{self, Orthogonal, Resample},
};

pub(super) struct EditResult {
    id: u64,
    generation: u64,
    label: &'static str,
    result: Result<Option<Document>, String>,
    select_tool: Option<Tool>,
}

/// Image/selection edits available from menus, canvas and shortcuts.
pub enum Edit {
    Rectangle(editing::PixelRect, palette_core::selection::Combine),
    Freeform(Vec<[f32; 2]>, palette_core::selection::Combine),
    PolygonSelection(Vec<[f32; 2]>, palette_core::selection::Combine),
    Wand(
        [u32; 2],
        palette_core::selection::Wand,
        palette_core::selection::Combine,
    ),
    Feather {
        radius: f32,
        mask: bool,
    },
    MaskFromSelection,
    MergeLayers(Vec<u64>),
    InvertMask,
    ToggleMask,
    DeleteMask,
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
            || crate::canvas::navigation::has_focus(ctx, self.current().id)
            || self.interface.popup_was_open
            || egui::Popup::is_any_open(ctx)
        {
            return;
        }
        if self.current().construction.is_some() || self.current().polygon_selection.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        if let Some(target) =
            super::clipboard::copy_shortcut(&events, ctx.input(|input| input.modifiers))
        {
            self.copy_pixels(target, ctx);
        }
        if self.tool == Tool::SelectBrush && self.current().selection_brush_draft.is_none() {
            let change = ctx.input(|i| {
                i.key_pressed(Key::CloseBracket) as i32 - i.key_pressed(Key::OpenBracket) as i32
            });
            if change != 0 {
                let size = &mut self.current_mut().selection_brush.diameter;
                *size = (*size + change as f32 * if *size < 20. { 1. } else { 5. }).clamp(1., 256.);
            }
        }
        for event in events {
            match event {
                egui::Event::Cut => self.copy_selection(true, ctx),
                egui::Event::PasteImage(image) => {
                    if self.queue_paste_after_copy() {
                        continue;
                    }
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
        let (command, shift, a, d, x, v, delete, escape, arrows) = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.key_pressed(Key::A),
                i.key_pressed(Key::D),
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
            if self.current_mut().cancel_brush_selection() {
                ctx.stop_dragging();
                return;
            }
            if self.current_mut().cancel_selection_feather() {
                ctx.stop_dragging();
                return;
            }
            if self.current_mut().lasso_drag.take().is_some() {
                return;
            }
            if self.current().document.floating_shape.is_some()
                && self.current().selection_transform.is_none()
            {
                self.edit(Edit::Clear, ctx);
                return;
            }
            let cancelled_shape = self.stroke.take().is_some() && self.tool.shape();
            if !self.current_mut().cancel_selection_transform()
                && !self.current_mut().cancel_layer_move()
                && !cancelled_shape
                && self.tool != Tool::Move
            {
                self.deselect();
            }
        }
        if !command
            && arrows != [0, 0]
            && self.tool == Tool::Move
            && self.current().document.selection.is_none()
        {
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
        if !matches!(
            operation,
            Edit::Move(_) | Edit::Transform(_) | Edit::Rotate(_) | Edit::Clear
        ) {
            // Ordinary edits finish adjustment; transforms retain the isolated
            // shape and its immutable original instead of cutting the display.
            document.floating_shape = None;
        }
        // Feather each incoming region before set algebra, rather than blurring
        // the previous selection repeatedly when adding/subtracting regions.
        let selection_feather = session.selection_feather;
        let select_tool =
            matches!(operation, Edit::Shape(..) | Edit::Path(..)).then_some(self.tool);
        let label = match operation {
            Edit::Rectangle(..) => "Rectangular selection",
            Edit::Freeform(..) => "Freeform selection",
            Edit::PolygonSelection(..) => "Polygonal selection",
            Edit::Wand(..) => "Magic Wand selection",
            Edit::Feather { mask: true, .. } => "Feather layer mask",
            Edit::Feather { mask: false, .. } => "Feather selection",
            Edit::MaskFromSelection => "Add layer mask",
            Edit::MergeLayers(_) => "Merge layers",
            Edit::InvertMask => "Invert layer mask",
            Edit::ToggleMask => "Toggle layer mask",
            Edit::DeleteMask => "Delete layer mask",
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
                    Edit::MergeLayers(ids) => {
                        palette_core::layers::merge_selected(&mut document, &ids, &|| false)?
                    }
                    Edit::Rectangle(rect, mode) => {
                        let incoming = editing::rectangle(document.width, document.height, rect);
                        palette_core::selection::apply_region(
                            &mut document,
                            &incoming,
                            mode,
                            selection_feather,
                        )?
                    }
                    Edit::Freeform(points, mode) | Edit::PolygonSelection(points, mode) => {
                        let mask = palette_core::selection::freeform(
                            document.width,
                            document.height,
                            &points,
                        )?;
                        palette_core::selection::apply_region(
                            &mut document,
                            &mask,
                            mode,
                            selection_feather,
                        )?
                    }
                    Edit::Wand(point, options, mode) => {
                        let mask = palette_core::selection::wand(&document, point, options)?;
                        palette_core::selection::apply_region(
                            &mut document,
                            &mask,
                            mode,
                            selection_feather,
                        )?
                    }
                    Edit::Feather { radius, mask } => {
                        if mask && document.active().locked {
                            return Err("Unlock the layer before editing its mask".into());
                        }
                        if mask {
                            let plane = &mut document.active_mut().mask;
                            let before = plane.as_ref().ok_or("No mask to feather")?;
                            let after = palette_core::selection::feather(before, radius)?;
                            let changed = before != &after;
                            *plane = Some(after);
                            changed
                        } else {
                            palette_core::selection::feather_selection(&mut document, radius)?
                        }
                    }
                    Edit::MaskFromSelection => {
                        palette_core::selection::mask_from_selection(&mut document)?
                    }
                    Edit::DeleteMask | Edit::ToggleMask | Edit::InvertMask => {
                        if document.active().locked {
                            return Err("Unlock the layer before editing its mask".into());
                        }
                        let plane = &mut document.active_mut().mask;
                        match operation {
                            Edit::DeleteMask => plane.take().is_some(),
                            Edit::ToggleMask => {
                                if let Some(m) = plane {
                                    m.enabled = !m.enabled;
                                    true
                                } else {
                                    false
                                }
                            }
                            _ => {
                                if let Some(m) = plane {
                                    for v in std::sync::Arc::make_mut(&mut m.values) {
                                        *v = 255 - *v;
                                    }
                                    true
                                } else {
                                    false
                                }
                            }
                        }
                    }
                    Edit::Fill {
                        point,
                        color,
                        opacity,
                        tolerance,
                    } => editing::flood_fill(&mut document, point, color, opacity, tolerance)?,
                    Edit::Shape(kind, from, to, style) => palette_core::floating::create(
                        &mut document,
                        &palette_core::shapes::path(kind, from, to),
                        style,
                    )?,
                    Edit::Path(path, style) => {
                        palette_core::floating::create(&mut document, &path, style)?
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
                        self.current_mut().selection_mode =
                            palette_core::selection::Combine::Replace;
                        self.choose_tool(Tool::Select);
                    }
                }
                Ok(None) => self.message = "No pixels changed".into(),
                Err(error) => self.error = Some(error),
            }
        }
    }

    pub fn select_all(&mut self) {
        self.current_mut().command("Select all", |doc| {
            doc.selection_base = None;
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
            self.current_mut().command("Deselect", |doc| {
                doc.selection = None;
                doc.selection_base = None;
            });
        }
    }
    pub fn invert_selection(&mut self) {
        self.current_mut().command("Invert selection", |doc| {
            let mask = doc.selection.take().unwrap_or_else(|| {
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
            doc.selection = Some(mask);
            palette_core::selection::invert(doc);
        });
    }

    pub(super) fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, String> {
        if self.clipboard.is_none() {
            self.clipboard = Some(
                arboard::Clipboard::new()
                    .map_err(|error| format!("Cannot access the system clipboard: {error}"))?,
            );
        }
        Ok(self.clipboard.as_mut().expect("initialized clipboard"))
    }
    pub fn copy_selection(&mut self, cut: bool, ctx: &Context) {
        self.current_mut().end_gesture();
        if cut && self.current().document.active().locked {
            return;
        }
        let result = (|| {
            let raster = editing::copy(&self.current().document)?;
            self.write_clipboard(&raster)
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
        if self.queue_paste_after_copy() {
            return;
        }
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
        self.choose_tool(Tool::Select);
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
