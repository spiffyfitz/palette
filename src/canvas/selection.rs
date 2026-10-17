//! Selection handles use logical display points for hit targets, image pixels
//! for geometry, and immutable source snapshots for asynchronous pixel previews.
use super::*;
use crate::session::{SelectionGesture, SelectionHandle};
use palette_core::transform::SelectionTransform;

fn screen_bounds(rect: PixelRect, canvas: Rect, zoom: f32) -> Rect {
    Rect::from_min_size(
        canvas.min + vec2(rect.x as f32, rect.y as f32) * zoom,
        vec2(rect.width as f32, rect.height as f32) * zoom,
    )
}

fn handles(rect: Rect) -> [(SelectionHandle, Pos2); 9] {
    use SelectionHandle::*;
    [
        (Rotate, rect.center_top() - vec2(0., 24.)),
        (Resize([-1, -1]), rect.left_top()),
        (Resize([1, -1]), rect.right_top()),
        (Resize([1, 1]), rect.right_bottom()),
        (Resize([-1, 1]), rect.left_bottom()),
        (Resize([0, -1]), rect.center_top()),
        (Resize([1, 0]), rect.right_center()),
        (Resize([0, 1]), rect.center_bottom()),
        (Resize([-1, 0]), rect.left_center()),
    ]
}

fn cursor(handle: SelectionHandle) -> egui::CursorIcon {
    match handle {
        SelectionHandle::Move => egui::CursorIcon::Move,
        SelectionHandle::Rotate => egui::CursorIcon::Grab,
        SelectionHandle::Resize([0, _]) => egui::CursorIcon::ResizeVertical,
        SelectionHandle::Resize([_, 0]) => egui::CursorIcon::ResizeHorizontal,
        SelectionHandle::Resize([x, y]) if x == y => egui::CursorIcon::ResizeNwSe,
        SelectionHandle::Resize(_) => egui::CursorIcon::ResizeNeSw,
    }
}

/// Positive extents and fixed opposite edges keep a crossing handle finite.
/// Shift preserves the original aspect ratio; rotation snaps to 15 degrees.
fn target(gesture: &SelectionGesture, point: [f32; 2], shift: bool) -> SelectionTransform {
    let rect = gesture.bounds;
    let delta = [point[0] - gesture.start[0], point[1] - gesture.start[1]];
    match gesture.handle {
        SelectionHandle::Move => {
            let mut delta = delta.map(|v| v.round() as i32);
            if shift {
                if delta[0].unsigned_abs() >= delta[1].unsigned_abs() {
                    delta[1] = 0;
                } else {
                    delta[0] = 0;
                }
            }
            SelectionTransform::Move(delta)
        }
        SelectionHandle::Rotate => {
            let center = [
                rect.x as f32 + rect.width as f32 * 0.5,
                rect.y as f32 + rect.height as f32 * 0.5,
            ];
            let start = (gesture.start[1] - center[1]).atan2(gesture.start[0] - center[0]);
            let angle = (point[1] - center[1]).atan2(point[0] - center[0]);
            let mut degrees = (angle - start).to_degrees();
            if shift {
                degrees = (degrees / 15.).round() * 15.;
            }
            SelectionTransform::Rotate(f64::from(degrees))
        }
        SelectionHandle::Resize(direction) => {
            let original = [rect.width as f32, rect.height as f32];
            let mut size = original;
            for axis in 0..2 {
                if direction[axis] != 0 {
                    size[axis] = (original[axis] + delta[axis] * f32::from(direction[axis]))
                        .clamp(1., 16384.);
                }
            }
            if shift {
                let axis = if direction[0] == 0 {
                    1
                } else if direction[1] == 0
                    || (size[0] / original[0] - 1.).abs() >= (size[1] / original[1] - 1.).abs()
                {
                    0
                } else {
                    1
                };
                let scale = (size[axis] / original[axis]).clamp(
                    (1. / original[0]).max(1. / original[1]),
                    (16384. / original[0]).min(16384. / original[1]),
                );
                size = original.map(|v| v * scale);
            }
            let size = size.map(|v| v.round().clamp(1., 16384.) as u32);
            let mut origin = [rect.x as i32, rect.y as i32];
            for axis in 0..2 {
                let change = size[axis] as i32 - original[axis] as i32;
                if direction[axis] < 0 {
                    origin[axis] -= change;
                } else if direction[axis] == 0 {
                    origin[axis] -= change / 2;
                }
            }
            SelectionTransform::Resize { origin, size }
        }
    }
}

impl PaletteApp {
    /// Give handles priority over canvas rectangle creation, including the
    /// rotation handle above the canvas edge. Modifiers combine new selections.
    pub(super) fn selection_handle_press(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        canvas: Rect,
        pointer: Option<Pos2>,
        pressed: bool,
    ) -> bool {
        if !matches!(self.tool, Tool::Select | Tool::Lasso)
            || !self.close_pending_none()
            || self.current().document.active().locked
            || self.current().selection_drag.is_some()
            || self.current().lasso_drag.is_some()
            || self.current().feather_preview.is_some()
            || self.current().selection_transform.is_some()
            || ctx.input(|i| i.modifiers.alt)
            || self.current().selection_mode != palette_core::selection::Combine::Replace
        {
            return false;
        }
        let Some(rect) = self.current_mut().selection_bounds() else {
            return false;
        };
        let Some(pointer) = pointer.filter(|p| painter.clip_rect().contains(*p)) else {
            return false;
        };
        let bounds = screen_bounds(rect, canvas, self.current().zoom);
        // Do not let enlarged outside hit targets consume the interior of small
        // selections. Inside, only the visible handle square claims the press.
        let radius = if bounds.shrink(3.).contains(pointer) {
            3.
        } else {
            6.
        };
        let handle = handles(bounds)
            .into_iter()
            .filter(|(_, p)| painter.clip_rect().contains(*p))
            .find(|(_, p)| p.distance(pointer) <= radius)
            .map(|(h, _)| h)
            .or_else(|| {
                let [x, y] = image_point(pointer, canvas, self.current().zoom);
                (!ctx.input(|i| i.modifiers.shift)
                    && canvas.contains(pointer)
                    && self
                        .current()
                        .document
                        .selection
                        .as_ref()
                        .is_some_and(|m| m.at(x as u32, y as u32) > 0.))
                .then_some(SelectionHandle::Move)
            });
        if let Some(handle) = handle {
            ctx.set_cursor_icon(cursor(handle));
            if pressed && ctx.input(|i| i.pointer.primary_pressed()) {
                let point = image_point(pointer, canvas, self.current().zoom);
                self.current_mut().begin_selection_transform(handle, point);
                return true;
            }
        }
        false
    }

    pub(super) fn selection_interaction(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        canvas: Rect,
        pointer: Option<Pos2>,
        buttons: [bool; 2],
    ) {
        let zoom = self.current().zoom;
        if self.tool == Tool::Lasso
            && let Some(mut drag) = self.current_mut().lasso_drag.take()
        {
            if let Some(pointer) = pointer {
                drag.sample(image_point(pointer, canvas, zoom), zoom);
            }
            if !self.is_capturing() && (buttons[1] || !buttons[0]) {
                if drag.points.len() >= 3 {
                    self.edit(
                        crate::app::editing::Edit::Freeform(drag.points, drag.mode),
                        ui.ctx(),
                    );
                }
            } else {
                let mut points: Vec<_> = drag
                    .points
                    .iter()
                    .map(|p| canvas.min + vec2(p[0], p[1]) * zoom)
                    .collect();
                if let Some(first) = points.first().copied() {
                    points.push(first);
                }
                let phase = if self.is_capturing() {
                    0.
                } else {
                    overlays::phase(ui.input(|i| i.time))
                };
                if overlays::ants(painter, &points, phase) && !self.is_capturing() {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(50));
                }
                self.current_mut().lasso_drag = Some(drag);
            }
        }
        if matches!(self.tool, Tool::Select | Tool::Lasso)
            && !self.is_capturing()
            && let Some(gesture) = &self.current().selection_transform
            && !gesture.released
        {
            let operation = pointer.map(|p| {
                target(
                    gesture,
                    image_point(p, canvas, zoom),
                    ui.input(|i| i.modifiers.shift),
                )
            });
            if let Some(operation) = operation
                && operation != gesture.desired
            {
                self.current_mut()
                    .selection_transform
                    .as_mut()
                    .expect("active gesture")
                    .desired = operation;
                self.current_mut().invalidate_preview();
            }
            if buttons[1] || !buttons[0] {
                let gesture = self
                    .current_mut()
                    .selection_transform
                    .as_mut()
                    .expect("active gesture");
                gesture.released = true;
                if gesture.applied == Some(gesture.desired)
                    || gesture.desired == SelectionTransform::Move([0, 0])
                {
                    self.current_mut().finish_selection_transform();
                }
            }
        }
        if self.tool == Tool::Select
            && let Some(mut drag) = self.current_mut().selection_drag.take()
        {
            if let Some(pointer) = pointer {
                drag.current = image_point(pointer, canvas, zoom);
            }
            if buttons[1] || !buttons[0] {
                let doc = &self.current().document;
                if let Some(rect) =
                    PixelRect::from_drag(drag.start, drag.current, doc.width, doc.height)
                {
                    if self.current().selection_feather > 0. {
                        self.edit(
                            crate::app::editing::Edit::Rectangle(rect, drag.mode),
                            ui.ctx(),
                        );
                    } else {
                        self.current_mut().command("Rectangular selection", |doc| {
                            let incoming = editing::rectangle(doc.width, doc.height, rect);
                            palette_core::selection::apply_region(doc, &incoming, drag.mode, 0.)
                                .expect("matching canvas masks");
                        });
                    }
                } else if drag.mode == palette_core::selection::Combine::Replace {
                    self.deselect();
                }
            } else {
                self.current_mut().selection_drag = Some(drag);
            }
        }
        let preview = self.current().selection_drag.as_ref().and_then(|d| {
            PixelRect::from_drag(
                d.start,
                d.current,
                self.current().document.width,
                self.current().document.height,
            )
        });
        let phase = if self.is_capturing() {
            0.
        } else {
            overlays::phase(ui.input(|i| i.time))
        };
        let mut visible = false;
        if let Some(rect) = preview {
            let bounds = screen_bounds(rect, canvas, zoom);
            visible |= overlays::ants(
                painter,
                &[
                    bounds.left_top(),
                    bounds.right_top(),
                    bounds.right_bottom(),
                    bounds.left_bottom(),
                    bounds.left_top(),
                ],
                phase,
            );
        } else {
            for contour in self.current_mut().feather_contours() {
                let points: Vec<_> = contour
                    .iter()
                    .map(|[x, y]| canvas.min + vec2(*x, *y) * zoom)
                    .collect();
                visible |= overlays::ants_alpha(painter, &points, phase, 102);
            }
            for contour in self.current_mut().selection_contours() {
                let points: Vec<_> = contour
                    .iter()
                    .map(|[x, y]| canvas.min + vec2(*x, *y) * zoom)
                    .collect();
                visible |= overlays::ants(painter, &points, phase);
            }
            if let Some(mask) = self
                .current()
                .selection_brush_draft
                .as_ref()
                .and_then(|d| d.frame.as_ref())
                .map(|f| f.base.as_ref().unwrap_or(&f.coverage))
                .or(self.current().document.selection_base.as_ref())
                .or(self.current().document.selection.as_ref())
            {
                visible |= overlays::viewport_ants(painter, mask, canvas, zoom, phase);
            }
        }
        if visible && !self.is_capturing() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if matches!(self.tool, Tool::Select | Tool::Lasso)
            && self.close_pending_none()
            && self.current().selection_mode == palette_core::selection::Combine::Replace
            && self.current().selection_drag.is_none()
            && self.current().lasso_drag.is_none()
            && self.current().feather_preview.is_none()
            && !self.current().document.active().locked
            && let Some(rect) = self.current_mut().selection_bounds()
        {
            let bounds = screen_bounds(rect, canvas, zoom);
            let stroke = Stroke::new(1., Color32::from_gray(75));
            painter.line_segment(
                [
                    bounds.center_top() - vec2(0., 4.),
                    bounds.center_top() - vec2(0., 20.),
                ],
                stroke,
            );
            for (handle, position) in handles(bounds) {
                if painter.clip_rect().contains(position) {
                    if handle == SelectionHandle::Rotate {
                        painter.circle(position, 3.5, Color32::WHITE, stroke);
                    } else {
                        let square = Rect::from_center_size(position, vec2(6., 6.));
                        painter.rect(square, 0, Color32::WHITE, stroke, StrokeKind::Outside);
                    }
                }
            }
        }
        if let Some(gesture) = &self.current().selection_transform {
            ui.ctx()
                .set_cursor_icon(if gesture.handle == SelectionHandle::Rotate {
                    egui::CursorIcon::Grabbing
                } else {
                    cursor(gesture.handle)
                });
            // Show the exact requested target immediately while the bounded CPU
            // worker catches up. The ants remain on the actual processed preview.
            if let SelectionTransform::Resize { origin, size } = gesture.desired {
                let bounds = Rect::from_min_size(
                    canvas.min + vec2(origin[0] as f32, origin[1] as f32) * zoom,
                    vec2(size[0] as f32, size[1] as f32) * zoom,
                );
                painter.rect_stroke(bounds, 0, Stroke::new(1., ACCENT), StrokeKind::Outside);
            }
        } else if matches!(self.tool, Tool::Select | Tool::Lasso) {
            if self.cursor.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            self.selection_handle_press(
                ui.ctx(),
                painter,
                canvas,
                pointer.filter(|p| !self.selection_toolbar_rect.contains(*p)),
                false,
            );
        }
    }
}
