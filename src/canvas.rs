mod brush_preview;
mod construction;
mod context_menu;
pub(crate) mod navigation;
mod overlays;
mod polygon_selection;
mod selection;
mod selection_brush;
pub(crate) mod zoom;
use crate::{
    app::{ACCENT, PaletteApp},
    icons::{self, Icon},
    session::{SelectionDrag, Tool},
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use palette_core::{
    brush::{Sample, Stroke as BrushStroke, Style},
    editing::{self, PixelRect},
};

/// Convert logical display points to image coordinates. egui already accounts
/// for OS DPI; multiplying by pixels_per_point here would apply scaling twice.
fn image_point(point: Pos2, canvas: Rect, zoom: f32) -> [f32; 2] {
    [
        (point.x - canvas.left()) / zoom,
        (point.y - canvas.top()) / zoom,
    ]
}

impl PaletteApp {
    pub fn canvas(&mut self, ui: &mut Ui) {
        let bounds = ui.max_rect();
        let contextual = self.tool.paints()
            || self.tool.shape()
            || matches!(self.tool, Tool::Fill | Tool::SelectBrush);
        let left_width = 62.0;
        let tray_visibility = ui.ctx().animate_bool_with_time(
            egui::Id::unique("layers-visible"),
            self.show_layers,
            crate::interface::PANEL_SECONDS,
        );
        let property_visibility = ui.ctx().animate_bool_with_time(
            egui::Id::unique("layer-properties-expanded"),
            self.interface.layer_properties,
            crate::interface::PANEL_SECONDS,
        );
        let tray_width = crate::interface::LAYER_TRAY_WIDTH + 68. * property_visibility;
        let right_width = 12.0 + (tray_width + 12.0) * tray_visibility;
        let workspace = Rect::from_min_max(
            pos2(bounds.left() + left_width, bounds.top()),
            pos2(bounds.right() - right_width, bounds.bottom()),
        );
        let image_size = vec2(
            self.current().document.width as f32,
            self.current().document.height as f32,
        ) * self.current().zoom;
        let mut viewport = navigation::layout(workspace, image_size, self.current().fit);
        let mut view = viewport.view;
        self.canvas_rect = view;
        self.processing_toolbar_window(ui.ctx());
        self.selection_toolbar(ui.ctx());
        let response = ui.interact(
            view,
            ui.scope_id().with("canvas-view"),
            Sense::click_and_drag(),
        );
        if self.current().fit {
            self.current_mut().zoom_animation.cancel();
            self.current_mut().pan = Vec2::ZERO;
            let size = vec2(
                self.current().document.width as f32,
                self.current().document.height as f32,
            );
            self.current_mut().zoom = ((view.width() - 64.0) / size.x)
                .min((view.height() - 64.0) / size.y)
                .clamp(0.05, 32.0);
        }
        let ctx = ui.ctx().clone();
        if let Some(pressure) = ctx.input(|i| {
            i.raw.events.iter().rev().find_map(|event| {
                if let egui::Event::Touch {
                    force: Some(force), ..
                } = event
                {
                    Some(*force)
                } else {
                    None
                }
            })
        }) {
            self.pointer_pressure = pressure.clamp(0., 1.);
        } else if ctx.input(|i| i.pointer.any_pressed()) {
            self.pointer_pressure = 1.;
        }
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let (scroll, ctrl, space, middle, delta, pressed, down, released) = ctx.input(|i| {
            (
                i.smooth_scroll_delta,
                i.modifiers.command || i.modifiers.ctrl,
                i.key_down(egui::Key::Space),
                i.pointer.button_down(egui::PointerButton::Middle),
                i.pointer.delta(),
                i.pointer.primary_pressed() || i.pointer.secondary_pressed(),
                i.pointer.primary_down() || i.pointer.secondary_down(),
                i.pointer.primary_released() || i.pointer.secondary_released(),
            )
        });
        let over_toolbar = pointer.is_some_and(|p| {
            self.selection_toolbar_rect.contains(p) || self.processing_toolbar_rect.contains(p)
        });
        let hovered = response.hovered() && !over_toolbar;
        if hovered && (pressed || middle) {
            for axis in 0..2 {
                ctx.memory_mut(|m| m.surrender_focus(navigation::id(self.current().id, axis)));
            }
        }
        let options = ctx.memory(|memory| memory.options.input_options);
        let (factor, anchor) = ctx.input(|input| zoom::gesture(input, view, &options));
        if hovered {
            if !ctrl && factor == 1. && scroll != Vec2::ZERO {
                self.current_mut().zoom_animation.cancel();
                self.current_mut().pan += scroll;
            }
            if middle || (space && down) {
                self.current_mut().pan += delta;
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        let blocked = egui::Popup::is_any_open(&ctx)
            || self.interface.popup_was_open
            || ctx.memory(|m| m.top_modal_layer().is_some())
            || ctx.input(|i| i.pointer.any_down());
        let zoom_input = anchor
            .filter(|point| {
                !blocked
                    && view.contains(*point)
                    && !self.selection_toolbar_rect.contains(*point)
                    && !self.processing_toolbar_rect.contains(*point)
            })
            .filter(|_| factor != 1.);
        if blocked {
            self.current_mut().zoom_animation.cancel();
        }
        let current_zoom = self.current().zoom;
        let current_size = vec2(
            self.current().document.width as f32,
            self.current().document.height as f32,
        ) * current_zoom;
        let mut old_origin = view.center() - current_size * 0.5 + self.current().pan;
        if !self.current().zoom_animation.is_active() {
            let dpi = ctx.pixels_per_point();
            old_origin = pos2(
                (old_origin.x * dpi).round() / dpi,
                (old_origin.y * dpi).round() / dpi,
            );
        }
        let old_canvas = Rect::from_min_size(old_origin, current_size);
        let input = zoom_input.map(|at| (factor, at, old_canvas));
        let step = self.current_mut().zoom_animation.advance(
            current_zoom,
            input,
            ctx.input(|i| i.stable_dt),
        );
        let mut zoom_animating = false;
        if let Some(step) = step {
            self.current_mut().zoom = step.zoom;
            self.current_mut().fit = false;
            let size = vec2(
                self.current().document.width as f32,
                self.current().document.height as f32,
            ) * step.zoom;
            viewport = navigation::layout(workspace, size, false);
            view = viewport.view;
            self.canvas_rect = view;
            self.current_mut().pan = step.origin + size * 0.5 - view.center();
            zoom_animating = step.animating;
            if step.animating {
                ctx.request_repaint();
            }
        }
        if zoom_input.is_some() {
            // In particular Ctrl+wheel on macOS isn't egui's default Cmd
            // modifier. Don't let its scroll delta reach an inline text box.
            ctx.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
        let zoom = self.current().zoom;
        let size = vec2(
            self.current().document.width as f32,
            self.current().document.height as f32,
        ) * zoom;
        self.canvas_scrollbars(ui, &viewport, size);
        let mut origin = view.center() - size * 0.5 + self.current().pan;
        // Snap display origin to physical pixels; coordinates still use logical points.
        let dpi = ctx.pixels_per_point();
        if !zoom_animating {
            origin.x = (origin.x * dpi).round() / dpi;
            origin.y = (origin.y * dpi).round() / dpi;
        }
        let canvas = Rect::from_min_size(origin, size);
        self.image_rect = canvas;
        let menu_blocked = self.selection_context_menu(ui, &response, canvas, pointer);
        let hovered = hovered && !menu_blocked;
        let painter = ui.painter().with_clip_rect(view);
        painter.rect_filled(
            canvas.expand(3.0).translate(vec2(2.0, 2.0)),
            0,
            Color32::from_black_alpha(75),
        );
        painter.rect_filled(canvas, 0, Color32::WHITE);
        if self.checkerboard {
            let clipped = canvas.intersect(view);
            if clipped.is_positive() {
                let tile = 12.0;
                let x0 = ((clipped.left() - origin.x) / tile).floor().max(0.0) as u32;
                let y0 = ((clipped.top() - origin.y) / tile).floor().max(0.0) as u32;
                let x1 = ((clipped.right() - origin.x) / tile).ceil().max(0.0) as u32;
                let y1 = ((clipped.bottom() - origin.y) / tile).ceil().max(0.0) as u32;
                for y in y0..y1 {
                    for x in x0..x1 {
                        if (x + y) % 2 == 0 {
                            let square = Rect::from_min_size(
                                origin + vec2(x as f32 * tile, y as f32 * tile),
                                vec2(tile, tile),
                            )
                            .intersect(canvas);
                            painter.rect_filled(square, 0, Color32::from_gray(213));
                        }
                    }
                }
            }
        }
        if let Some(texture) = self
            .processing_preview()
            .map(|t| t.id())
            .or_else(|| self.gpu_texture())
            .or_else(|| self.current().texture.as_ref().map(|t| t.id()))
        {
            painter.image(
                texture,
                canvas,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if self.interface.grid && zoom >= 4.0 {
            let clipped = canvas.intersect(view);
            let from = image_point(clipped.min, canvas, zoom);
            let to = image_point(clipped.max, canvas, zoom);
            let stroke = Stroke::new(0.6, Color32::from_black_alpha(75));
            for x in from[0].floor().max(0.0) as u32
                ..=to[0].ceil().min(self.current().document.width as f32) as u32
            {
                let px = canvas.left() + x as f32 * zoom;
                painter.line_segment(
                    [pos2(px, clipped.top()), pos2(px, clipped.bottom())],
                    stroke,
                );
            }
            for y in from[1].floor().max(0.0) as u32
                ..=to[1].ceil().min(self.current().document.height as f32) as u32
            {
                let py = canvas.top() + y as f32 * zoom;
                painter.line_segment(
                    [pos2(clipped.left(), py), pos2(clipped.right(), py)],
                    stroke,
                );
            }
        }
        painter.rect_stroke(
            canvas,
            0,
            Stroke::new(1.0, Color32::from_gray(107)),
            StrokeKind::Outside,
        );

        self.cursor = pointer
            .filter(|p| view.contains(*p) && canvas.contains(*p) && !over_toolbar)
            .map(|p| {
                let point = image_point(p, canvas, zoom);
                [point[0].floor() as u32, point[1].floor() as u32]
            });
        self.processing_origin_interaction(
            &painter,
            canvas,
            pointer,
            [
                hovered,
                ctx.input(|i| i.pointer.primary_pressed()),
                ctx.input(|i| i.pointer.primary_down()),
                space || middle,
            ],
        );
        let drawing_allowed = !menu_blocked
            && self.close_pending_none()
            && self.current().feather_preview.is_none()
            && self.current().selection_brush_draft.is_none()
            && (!self.current().document.active().locked
                || (!self.tool.paints()
                    && !self.tool.shape()
                    && self.tool != Tool::Fill
                    && self.tool != Tool::Move));
        if !matches!(self.tool, Tool::Select | Tool::Move) {
            self.current_mut().document.floating_shape = None;
        }
        let selection_handled = self.selection_handle_press(
            &ctx,
            &painter,
            canvas,
            pointer.filter(|_| !over_toolbar),
            hovered && pressed && !space && !middle && drawing_allowed,
        );
        let inside_shape = self.cursor.is_some_and(|[x, y]| {
            self.current()
                .document
                .selection
                .as_ref()
                .is_some_and(|m| m.at(x, y) > 0.)
        });
        let shape_committed = hovered
            && !selection_handled
            && !inside_shape
            && drawing_allowed
            && ctx.input(|i| i.pointer.primary_pressed())
            && !space
            && !middle
            && self.current_mut().commit_floating_shape();
        if hovered
            && !selection_handled
            && !shape_committed
            && (self.current().document.floating_shape.is_none()
                || (self.tool == Tool::Move && ctx.input(|i| i.pointer.primary_pressed())))
            && self.current().selection_transform.is_none()
            && self.current().feather_preview.is_none()
            && pressed
            && !space
            && !middle
            && canvas.contains(pointer.unwrap_or(Pos2::ZERO))
            && drawing_allowed
        {
            self.stroke_background = ctx.input(|i| i.pointer.secondary_pressed());
            let point = image_point(pointer.expect("hovered pointer"), canvas, zoom);
            match self.tool {
                Tool::Move => {
                    if self.current().document.selection.is_some() {
                        self.current_mut().begin_selection_transform(
                            crate::session::SelectionHandle::Move,
                            point,
                        );
                    } else if !self.current().document.active().locked {
                        let original = self.current().document.active().offset;
                        self.current_mut().invalidate();
                        self.current_mut().layer_drag = Some(crate::session::LayerDrag {
                            start: point,
                            original,
                        });
                    }
                }
                Tool::Polygon | Tool::Curve | Tool::PolygonSelect => {}
                Tool::Text => {
                    self.current_mut().text_drag = Some(point);
                }
                Tool::Select => {
                    let mode = self.selection_mode(&ctx);
                    self.current_mut().selection_drag = Some(SelectionDrag {
                        start: point,
                        current: point,
                        mode,
                    });
                }
                Tool::Lasso => {
                    let mode = self.selection_mode(&ctx);
                    self.current_mut().lasso_drag = Some(crate::session::LassoDrag {
                        points: vec![point],
                        mode,
                    });
                }
                Tool::Wand => {
                    self.edit(
                        crate::app::editing::Edit::Wand(
                            [point[0] as u32, point[1] as u32],
                            self.current().wand,
                            self.selection_mode(&ctx),
                        ),
                        &ctx,
                    );
                }
                Tool::SelectBrush => {
                    let mode = self.selection_mode(&ctx);
                    self.begin_brush_selection(point, mode);
                }
                Tool::Fill => {
                    let color = if self.stroke_background {
                        self.background.to_srgba_unmultiplied()
                    } else {
                        self.brush.color
                    };
                    self.edit(
                        crate::app::editing::Edit::Fill {
                            point: [point[0] as u32, point[1] as u32],
                            color,
                            opacity: self.brush.opacity,
                            tolerance: self.fill_tolerance,
                        },
                        &ctx,
                    );
                }
                Tool::Magnifier => {
                    self.current_mut().zoom = (zoom * 2.0).min(32.0);
                    self.current_mut().fit = false;
                }
                Tool::Picker => {
                    self.brush.color = self.sample_color(point[0] as u32, point[1] as u32);
                }
                tool if tool.paints() || tool.shape() => {
                    if tool.paints() {
                        let style = if tool == Tool::Brush {
                            self.brush_style
                        } else {
                            Style::Round
                        };
                        let session = self.current_mut();
                        session.history.begin(tool.label(), &session.document);
                        if let Err(error) = editing::ensure_canvas_extent(&mut session.document) {
                            self.error = Some(error);
                            return;
                        }
                        session.paint_stroke = Some(BrushStroke::new(
                            &session.document.active().source,
                            style,
                            session.generation ^ session.id.rotate_left(19),
                        ));
                    }
                    self.stroke = Some((point, point));
                    self.gesture_session = Some(self.current().id);
                    if tool.paints() {
                        self.paint_sample(point, false, ctx.input(|i| i.time));
                    }
                }
                _ => {}
            }
        }
        if let Some((start, last)) = self.stroke
            && self.gesture_session == Some(self.current().id)
        {
            let mut point = pointer
                .map(|p| image_point(p, canvas, zoom))
                .unwrap_or(last);
            if let Some(kind) = self.tool.shape_kind()
                && ctx.input(|i| i.modifiers.shift)
            {
                point = palette_core::shapes::constrain(kind, start, point);
            }
            let spray = self.tool == Tool::Brush && self.brush_style == Style::Airbrush;
            if down && (point != last || spray) {
                if self.tool.paints() {
                    self.paint_sample(point, false, ctx.input(|i| i.time));
                    if spray {
                        ctx.request_repaint_after(std::time::Duration::from_millis(16));
                    }
                }
                self.stroke = Some((start, point));
            }
            if let Some(kind) = self.tool.shape_kind() {
                let path = palette_core::shapes::path(kind, start, point);
                let style = self.shape_style_values();
                overlays::shape(&painter, &path, style, origin, zoom);
            }
            if released || !down {
                if self.tool.paints() {
                    self.paint_sample(point, true, ctx.input(|i| i.time));
                }
                if let Some(kind) = self.tool.shape_kind() {
                    self.edit(
                        crate::app::editing::Edit::Shape(
                            kind,
                            start,
                            point,
                            self.shape_style_values(),
                        ),
                        &ctx,
                    );
                }
                self.stroke = None;
                let session = self.current_mut();
                session.paint_stroke = None;
                session.history.finish(&session.document);
            }
        }
        self.construction_interaction(
            ui,
            &painter,
            canvas,
            pointer,
            [
                hovered
                    && pressed
                    && !space
                    && !middle
                    && drawing_allowed
                    && canvas.contains(pointer.unwrap_or(Pos2::ZERO)),
                down,
                released,
                response.double_clicked(),
            ],
        );
        self.polygon_selection_interaction(
            &painter,
            canvas,
            pointer.filter(|_| !over_toolbar && !space && !middle),
            hovered && !selection_handled && drawing_allowed && !space && !middle,
            ctx.input(|i| {
                i.pointer
                    .button_double_clicked(egui::PointerButton::Primary)
                    || i.pointer
                        .button_triple_clicked(egui::PointerButton::Primary)
            }),
        );
        self.layer_move_interaction(&ctx, &painter, canvas, pointer, down, released);
        self.brush_selection_interaction(
            &painter,
            canvas,
            pointer.filter(|_| !space && !middle),
            [down, released],
        );
        self.selection_interaction(ui, &painter, canvas, pointer, [down, released]);
        self.text_interaction(&painter, canvas, pointer, down, released);
        if self.close_pending_none()
            && self.cursor.is_some()
            && (self.tool.paints() || self.tool == Tool::SelectBrush)
            && !space
        {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            if let Some(point) = pointer {
                let radius = (if self.tool == Tool::Pencil {
                    1.0
                } else {
                    if self.tool == Tool::SelectBrush {
                        self.current().selection_brush.diameter
                    } else {
                        self.brush.diameter
                    }
                }) * zoom
                    * 0.5;
                painter.circle_stroke(point, radius.max(1.0), Stroke::new(2.0, Color32::BLACK));
                painter.circle_stroke(point, radius.max(1.0), Stroke::new(1.0, Color32::WHITE));
                if self.tool == Tool::SelectBrush {
                    let subtract =
                        self.selection_mode(&ctx) == palette_core::selection::Combine::Subtract;
                    painter.line_segment(
                        [point - vec2(3., 0.), point + vec2(3., 0.)],
                        Stroke::new(1., Color32::BLACK),
                    );
                    if !subtract {
                        painter.line_segment(
                            [point - vec2(0., 3.), point + vec2(0., 3.)],
                            Stroke::new(1., Color32::BLACK),
                        );
                    }
                }
            }
        }
        // The bottom-right handle performs a reversible canvas-bounds resize.
        let handle = Rect::from_center_size(canvas.right_bottom() + vec2(3.0, 3.0), vec2(9.0, 9.0));
        if view.contains(handle.center())
            && self.close_pending_none()
            && self.current().feather_preview.is_none()
            && self.current().selection_brush_draft.is_none()
            && !selection_handled
            && self.current().polygon_selection.is_none()
            && self.current().selection_transform.is_none()
            && !(matches!(
                self.tool,
                Tool::Select | Tool::Lasso | Tool::PolygonSelect | Tool::Move
            ) && self.current().document.selection.is_some())
        {
            let resize = ui
                .interact(handle, ui.scope_id().with("resize-canvas"), Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
                .on_hover_text("Resize canvas bounds");
            painter.rect_filled(handle.shrink(2.0), 0, Color32::WHITE);
            if resize.drag_started() {
                self.resize_start = Some((
                    [
                        self.current().document.width,
                        self.current().document.height,
                    ],
                    pointer.unwrap_or(handle.center()),
                ));
            }
            if resize.dragged()
                && let Some((dimensions, start)) = self.resize_start
            {
                let change = (pointer.unwrap_or(start) - start) / zoom;
                self.resize_preview = Some([
                    (dimensions[0] as f32 + change.x).round().clamp(1.0, 8192.0) as u32,
                    (dimensions[1] as f32 + change.y).round().clamp(1.0, 8192.0) as u32,
                ]);
            }
            if let Some(dimensions) = self.resize_preview {
                let preview = Rect::from_min_size(
                    origin,
                    vec2(dimensions[0] as f32, dimensions[1] as f32) * zoom,
                );
                painter.rect_stroke(preview, 0, Stroke::new(1.5, ACCENT), StrokeKind::Outside);
            }
            if resize.drag_stopped() {
                if let Some(dimensions) = self.resize_preview.take() {
                    self.resize_canvas(dimensions);
                }
                self.resize_start = None;
            }
        }
        if contextual {
            let sliders = Rect::from_min_max(
                pos2(bounds.left() + 12.0, bounds.top() + 24.0),
                pos2(bounds.left() + 48.0, bounds.bottom() - 24.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(sliders), |ui| {
                self.brush_sliders(ui)
            });
            self.paint_brush_size_preview(ui);
        }
        if tray_visibility > 0.0 {
            let panel = Rect::from_min_max(
                pos2(
                    bounds.right() - 12.0 - tray_width * tray_visibility,
                    bounds.top() + 12.0,
                ),
                pos2(
                    bounds.right() - 12.0 + tray_width * (1.0 - tray_visibility),
                    bounds.bottom() - 12.0,
                ),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(panel), |ui| {
                ui.set_clip_rect(panel.intersect(bounds));
                ui.multiply_opacity(tray_visibility);
                ui.add_enabled_ui(tray_visibility > 0.9, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(40, 40, 44))
                        .corner_radius(7)
                        .inner_margin(8)
                        .show(ui, |ui| {
                            ui.set_width(tray_width - 16.0);
                            ui.set_min_height(panel.height() - 16.0);
                            ui.set_max_height(panel.height() - 16.0);
                            self.layers(ui);
                        });
                });
            });
        }
        response.on_hover_text(if self.tool == Tool::Move {
            "Move selected pixels, or the active layer when there is no selection · Shift constrains axis · Arrows nudge · Escape cancels"
        } else if self.tool == Tool::PolygonSelect {
            "Click vertices · Enter/double-click or click the start to finish · Backspace removes a vertex · Escape cancels · Shift/Alt on the first click combines selections"
        } else if matches!(self.tool, Tool::Select | Tool::Lasso) {
            "Drag selected pixels or handles · Shift adds · Alt subtracts · Shift+Alt intersects · Choose Replace for transforms · Escape cancels"
        } else if self.tool==Tool::Wand {
            "Click matching colors · Shift adds · Alt subtracts · Shift+Alt intersects · Options above canvas"
        } else if self.tool==Tool::SelectBrush {
            "Paint selection · Alt subtracts · Shift adds · [ / ] changes size · Escape cancels · Options above canvas"
        } else {
            "Ctrl + wheel to zoom · Middle mouse or Space + drag to pan"
        });
    }

    fn layer_move_interaction(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        canvas: Rect,
        pointer: Option<Pos2>,
        down: bool,
        released: bool,
    ) {
        if self.tool != Tool::Move {
            return;
        }
        ctx.set_cursor_icon(if self.current().document.active().locked {
            egui::CursorIcon::NotAllowed
        } else {
            egui::CursorIcon::Move
        });
        if let Some(drag) = &self.current().layer_drag {
            let original = drag.original;
            let start = drag.start;
            if let Some(point) = pointer {
                let point = image_point(point, canvas, self.current().zoom);
                let mut delta = [
                    (point[0] - start[0]).round() as i32,
                    (point[1] - start[1]).round() as i32,
                ];
                if ctx.input(|i| i.modifiers.shift) {
                    if delta[0].unsigned_abs() > delta[1].unsigned_abs() {
                        delta[1] = 0;
                    } else {
                        delta[0] = 0;
                    }
                }
                let position = [
                    original[0].saturating_add(delta[0]),
                    original[1].saturating_add(delta[1]),
                ];
                if position != self.current().document.active().offset {
                    self.current_mut().document.active_mut().offset = position;
                    self.current_mut().invalidate_preview();
                }
            }
            if released || !down {
                self.current_mut().end_gesture();
            }
        }
        if self.current().document.selection.is_some() {
            return;
        }
        let layer = self.current().document.active();
        let zoom = self.current().zoom;
        let bounds = Rect::from_min_size(
            canvas.min + vec2(layer.offset[0] as f32, layer.offset[1] as f32) * zoom,
            vec2(layer.source.width() as f32, layer.source.height() as f32) * zoom,
        );
        painter.rect_stroke(bounds, 0, Stroke::new(1., ACCENT), StrokeKind::Outside);
    }

    fn shape_style_values(&self) -> palette_core::shapes::Style {
        let (foreground, background) = if self.stroke_background {
            (self.background.to_srgba_unmultiplied(), self.brush.color)
        } else {
            (self.brush.color, self.background.to_srgba_unmultiplied())
        };
        palette_core::shapes::Style {
            outline: self.shape_outline.then_some(foreground),
            fill: self.shape_fill.then_some(background),
            width: self.brush.diameter,
            opacity: self.brush.opacity,
        }
    }

    fn paint_sample(&mut self, to: [f32; 2], finish: bool, time: f64) {
        let mut brush = self.brush;
        if self.stroke_background {
            brush.color = self.background.to_srgba_unmultiplied();
        }
        brush.erase = self.tool == Tool::Eraser;
        if self.tool == Tool::Pencil {
            brush.diameter = 1.0;
            brush.hardness = 1.0;
            brush.opacity = 1.0;
        }
        let offset = self.current().document.active().offset;
        let to = [to[0] - offset[0] as f32, to[1] - offset[1] as f32];
        let sample = Sample {
            position: to,
            pressure: self.pointer_pressure,
            time,
        };
        let selection = self.current().document.selection.clone();
        let session = self.current_mut();
        if let Some(stroke) = &mut session.paint_stroke {
            stroke.sample(
                brush,
                &mut session.document.active_mut().source,
                sample,
                selection.as_ref().map(|mask| (mask, offset)),
            );
            if finish {
                stroke.finish(
                    brush,
                    &mut session.document.active_mut().source,
                    selection.as_ref().map(|mask| (mask, offset)),
                );
            }
            if let Some(dirty) = stroke.take_dirty_region() {
                let left = i64::from(dirty.x) + i64::from(offset[0]);
                let top = i64::from(dirty.y) + i64::from(offset[1]);
                let right = (left + i64::from(dirty.width)).min(i64::from(session.document.width));
                let bottom =
                    (top + i64::from(dirty.height)).min(i64::from(session.document.height));
                let left = left.max(0);
                let top = top.max(0);
                let region = (right > left && bottom > top).then_some(PixelRect {
                    x: left as u32,
                    y: top as u32,
                    width: (right - left).max(0) as u32,
                    height: (bottom - top).max(0) as u32,
                });
                session.invalidate_paint(region);
            }
        }
    }

    fn brush_sliders(&mut self, ui: &mut Ui) {
        let height = ((ui.available_height() - 24.0) / 3.0).max(60.0);
        for (index, (icon, label)) in [
            (Icon::Brush, "Brush size"),
            (
                Icon::Opacity,
                if self.tool == Tool::SelectBrush {
                    "Selection strength"
                } else {
                    "Brush opacity"
                },
            ),
            (Icon::Hardness, "Brush hardness"),
        ]
        .into_iter()
        .enumerate()
        {
            egui::Frame::new()
                .fill(Color32::from_rgb(46, 46, 45))
                .corner_radius(7)
                .inner_margin(5)
                .show(ui, |ui| {
                    ui.set_width(26.0);
                    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
                    icons::draw(ui, rect.shrink(5.0), icon, Color32::WHITE);
                    response.on_hover_text(label);
                    ui.add_enabled_ui(
                        self.tool.paints()
                            || self.tool == Tool::SelectBrush
                            || (self.tool.shape() && index != 2)
                            || (self.tool == Tool::Fill && index == 1),
                        |ui| {
                            ui.spacing_mut().slider_width = (height - 62.0).max(14.0);
                            ui.spacing_mut().slider_rail_height = 3.0;
                            ui.spacing_mut().interact_size.y = 16.0;
                            ui.visuals_mut().selection.bg_fill = ACCENT;
                            ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(135);
                            ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_gray(135);
                            ui.visuals_mut().widgets.active.bg_fill = Color32::from_gray(135);
                            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(3.0, ACCENT);
                            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(3.0, ACCENT);
                            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(3.0, ACCENT);
                            let value = if self.tool == Tool::SelectBrush {
                                match index {
                                    0 => &mut self.sessions[self.active].selection_brush.diameter,
                                    1 => &mut self.sessions[self.active].selection_brush.strength,
                                    _ => &mut self.sessions[self.active].selection_brush.hardness,
                                }
                            } else {
                                match index {
                                    0 => &mut self.brush.diameter,
                                    1 => &mut self.brush.opacity,
                                    _ => &mut self.brush.hardness,
                                }
                            };
                            let range = if index == 0 { 1.0..=256.0 } else { 0.0..=1.0 };
                            let slider = ui
                                .add(
                                    egui::Slider::new(value, range)
                                        .vertical()
                                        .handle_shape(egui::style::HandleShape::Circle)
                                        .trailing_fill(true)
                                        .show_value(false),
                                )
                                .on_hover_text(format!(
                                    "{label}: {:.0}{}",
                                    if index == 0 { *value } else { *value * 100.0 },
                                    if index == 0 { " px" } else { "%" }
                                ));
                            if index == 0 {
                                self.brush_size_slider_rect = slider.rect;
                                if slider.dragged() || slider.changed() {
                                    self.brush_size_preview_until =
                                        Some(ui.input(|i| i.time) + 0.65);
                                }
                            }
                            ui.visuals_mut().widgets.inactive.fg_stroke =
                                Stroke::new(1.0, Color32::from_gray(225));
                            ui.visuals_mut().widgets.hovered.fg_stroke =
                                Stroke::new(1.0, Color32::WHITE);
                            ui.visuals_mut().widgets.active.fg_stroke =
                                Stroke::new(1.0, Color32::WHITE);
                            ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(48);
                            let mut display = if index == 0 { *value } else { *value * 100.0 };
                            if ui
                                .add(
                                    egui::DragValue::new(&mut display)
                                        .range(if index == 0 { 1.0..=256.0 } else { 0.0..=100.0 })
                                        .speed(1.0)
                                        .max_decimals(0),
                                )
                                .on_hover_text(format!("{label}: type an exact value"))
                                .changed()
                            {
                                *value = if index == 0 { display } else { display / 100.0 };
                                if index == 0 {
                                    self.brush_size_preview_until =
                                        Some(ui.input(|i| i.time) + 0.65);
                                }
                            }
                        },
                    );
                });
            ui.add_space(6.0);
        }
    }

    fn resize_canvas(&mut self, dimensions: [u32; 2]) {
        self.current_mut().command("Resize canvas", |doc| {
            palette_core::transform::canvas_resize(doc, dimensions[0], dimensions[1])
                .expect("bounded canvas dimensions");
        });
    }
}

impl PaletteApp {
    fn text_interaction(
        &mut self,
        painter: &egui::Painter,
        canvas: Rect,
        pointer: Option<Pos2>,
        down: bool,
        released: bool,
    ) {
        if self.tool != Tool::Text {
            self.current_mut().text_drag = None;
            return;
        }
        let Some(start) = self.current().text_drag else {
            return;
        };
        let zoom = self.current().zoom;
        let point = pointer
            .map(|p| image_point(p, canvas, zoom))
            .unwrap_or(start);
        let doc = &self.current().document;
        let rect = PixelRect::from_drag(start, point, doc.width, doc.height);
        if released || !down {
            let rect = rect.filter(|r| r.width >= 4 && r.height >= 4).or_else(|| {
                PixelRect::from_drag(
                    start,
                    [start[0] + 320.0, start[1] + 160.0],
                    doc.width,
                    doc.height,
                )
            });
            self.current_mut().text_drag = None;
            if let Some(rect) = rect {
                self.start_text(rect);
            }
        } else if let Some(rect) = rect {
            painter.rect_stroke(
                Rect::from_min_size(
                    canvas.min + vec2(rect.x as f32, rect.y as f32) * zoom,
                    vec2(rect.width as f32, rect.height as f32) * zoom,
                ),
                0,
                Stroke::new(1.0, ACCENT),
                StrokeKind::Outside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_at_fractional_zoom_and_negative_pan() {
        let canvas = Rect::from_min_size(pos2(-30.25, 100.5), vec2(614.25, 445.5));
        assert_eq!(image_point(pos2(7.25, 175.5), canvas, 0.75), [50.0, 100.0]);
    }
}
