//! Polygon rubber-band and two-bend cubic transactions. No source pixel changes
//! until closure/confirmation; session ownership prevents cross-tab gestures.
use super::*;
use crate::session::Construction;
use palette_core::shapes::{self, Path};
impl PaletteApp {
    pub(super) fn construction_interaction(
        &mut self,
        ui: &Ui,
        painter: &egui::Painter,
        canvas: Rect,
        pointer: Option<Pos2>,
        buttons: [bool; 4],
    ) {
        if !matches!(self.tool, Tool::Polygon | Tool::Curve) {
            return;
        }
        let ctx = ui.ctx();
        if !self.close_pending_none()
            || egui::Popup::is_any_open(ctx)
            || self.interface.popup_was_open
        {
            return;
        }
        let [pressed, down, released, double] = buttons;
        let point = pointer.map(|p| image_point(p, canvas, self.current().zoom));
        let mut draft = self.current_mut().construction.take();
        if pressed
            && draft.is_none()
            && let Some(point) = point
        {
            self.current_mut().end_gesture();
            draft = Some(Construction {
                tool: self.tool,
                points: if self.tool == Tool::Polygon {
                    vec![point]
                } else {
                    vec![point; 4]
                },
                hover: point,
                dragging: true,
                stage: 0,
                style: self.shape_style_values(),
            });
        } else if pressed && let (Some(draft), Some(point)) = (&mut draft, point) {
            draft.dragging = true;
            if draft.tool == Tool::Polygon && draft.points.len() < 4096 {
                let last = *draft.points.last().unwrap();
                let point = if ctx.input(|i| i.modifiers.shift) {
                    shapes::constrain(shapes::Kind::Line, last, point)
                } else {
                    point
                };
                if (point[0] - last[0]).hypot(point[1] - last[1]) > 0.001 {
                    draft.points.push(point);
                }
            }
        }
        let Some(mut draft) = draft else {
            return;
        };
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        if !self.is_capturing()
            && let Some(point) = point
            && pointer.is_some_and(|p| self.canvas_rect.contains(p))
        {
            draft.hover = if ctx.input(|i| i.modifiers.shift) {
                shapes::constrain(shapes::Kind::Line, *draft.points.last().unwrap(), point)
            } else {
                point
            };
        }
        if draft.tool == Tool::Polygon
            && ctx.input(|i| i.key_pressed(egui::Key::Backspace))
            && draft.points.len() > 1
        {
            draft.points.pop();
        }
        let mut finish = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        if draft.tool == Tool::Polygon {
            if released && draft.dragging && draft.stage == 0 {
                let last = draft.points[0];
                if (draft.hover[0] - last[0]).hypot(draft.hover[1] - last[1]) * self.current().zoom
                    > 2.
                {
                    draft.points.push(draft.hover);
                }
                draft.stage = 1;
            }
            if pressed && draft.points.len() > 3 {
                let first = draft.points[0];
                let last = *draft.points.last().unwrap();
                if (last[0] - first[0]).hypot(last[1] - first[1]) * self.current().zoom <= 8. {
                    draft.points.pop();
                    finish = true;
                }
            }
            finish |= double || ctx.input(|i| i.pointer.secondary_pressed());
            let mut points = draft.points.clone();
            if points.last().is_none_or(|p| *p != draft.hover) {
                points.push(draft.hover);
            }
            // Even fill-only polygons need a visible rubber-band for the next edge.
            let mut style = draft.style;
            style.outline = Some(style.outline.or(style.fill).unwrap_or(self.brush.color));
            style.fill = None;
            overlays::shape(
                painter,
                &Path {
                    points,
                    closed: false,
                },
                style,
                canvas.min,
                self.current().zoom,
            );
            if finish && draft.points.len() >= 3 {
                self.edit(
                    crate::app::editing::Edit::Path(
                        Path {
                            points: draft.points,
                            closed: true,
                        },
                        draft.style,
                    ),
                    ctx,
                );
                return;
            }
            self.message="Polygon: click vertices · Enter/double-click to finish · Backspace removes · Escape cancels".into();
        } else {
            if draft.dragging && down {
                match draft.stage {
                    0 => {
                        draft.points[3] = draft.hover;
                        for (i, t) in [(1, 1. / 3.), (2, 2. / 3.)] {
                            draft.points[i] = std::array::from_fn(|a| {
                                draft.points[0][a] + (draft.points[3][a] - draft.points[0][a]) * t
                            });
                        }
                    }
                    1 => draft.points[1] = draft.hover,
                    _ => draft.points[2] = draft.hover,
                }
            }
            let path = shapes::cubic(
                draft.points[0],
                draft.points[3],
                [draft.points[1], draft.points[2]],
            );
            overlays::shape(painter, &path, draft.style, canvas.min, self.current().zoom);
            if released && draft.dragging {
                if draft.stage == 2 {
                    finish = true;
                } else {
                    draft.stage += 1;
                }
            }
            if finish && draft.stage > 0 {
                self.edit(crate::app::editing::Edit::Path(path, draft.style), ctx);
                return;
            }
            self.message = if draft.stage == 0 {
                "Curve: drag its baseline"
            } else {
                "Curve: drag two bends · Enter finishes · Escape cancels"
            }
            .into();
        }
        if released {
            draft.dragging = false;
        }
        self.current_mut().construction = Some(draft);
    }
}
