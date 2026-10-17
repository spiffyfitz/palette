//! Scrollbars share the canvas pan in logical points. They reserve workspace
//! gutters before image hit testing, so the layers tray cannot cover the rail.
use super::*;
const RAIL: f32 = 14.;
const EDGE_MARGIN: f32 = 16.;

pub(super) struct Viewport {
    pub view: Rect,
    pub rails: [Option<Rect>; 2],
}
pub(super) fn layout(bounds: Rect, size: Vec2, fit: bool) -> Viewport {
    let (mut horizontal, mut vertical) = (false, false);
    if !fit {
        // One rail can make the other axis overflow. Iterate to a stable layout.
        for _ in 0..3 {
            horizontal = size.x > bounds.width() - if vertical { RAIL } else { 0. };
            vertical = size.y > bounds.height() - if horizontal { RAIL } else { 0. };
        }
    }
    let view = Rect::from_min_max(
        bounds.min,
        bounds.max
            - vec2(
                if vertical { RAIL } else { 0. },
                if horizontal { RAIL } else { 0. },
            ),
    );
    Viewport {
        view,
        rails: [
            horizontal.then(|| {
                Rect::from_min_max(
                    pos2(view.left(), view.bottom() + 2.),
                    pos2(view.right(), bounds.bottom() - 2.),
                )
            }),
            vertical.then(|| {
                Rect::from_min_max(
                    pos2(view.right() + 2., view.top()),
                    pos2(bounds.right() - 2., view.bottom()),
                )
            }),
        ],
    }
}
pub(crate) fn id(session: u64, axis: usize) -> egui::Id {
    egui::Id::unique(("canvas-scrollbar", session, axis))
}
pub(crate) fn has_focus(ctx: &egui::Context, session: u64) -> bool {
    ctx.memory(|m| (0..2).any(|axis| m.has_focus(id(session, axis))))
}

impl PaletteApp {
    pub(super) fn canvas_scrollbars(&mut self, ui: &mut Ui, viewport: &Viewport, size: Vec2) {
        self.canvas_scrollbars = viewport.rails;
        for axis in 0..2 {
            let Some(track) = viewport.rails[axis] else {
                self.current_mut().pan[axis] = 0.;
                continue;
            };
            let length = track.size()[axis];
            let visible = viewport.view.size()[axis];
            let limit = ((size[axis] - visible) * 0.5 + EDGE_MARGIN).max(0.);
            let pan = self.current().pan[axis].clamp(-limit, limit);
            self.current_mut().pan[axis] = pan;
            let thumb_length =
                (length * visible / (size[axis] + 2. * EDGE_MARGIN)).clamp(24., length);
            let travel = (length - thumb_length).max(0.);
            let at = if limit > 0. {
                (limit - pan) / (2. * limit) * travel
            } else {
                0.
            };
            let thumb_rect = |at: f32| {
                let mut rect = track;
                rect.min[axis] += at;
                rect.max[axis] = rect.min[axis] + thumb_length;
                rect.shrink2(if axis == 0 {
                    vec2(0., 2.)
                } else {
                    vec2(2., 0.)
                })
            };
            let thumb = thumb_rect(at);
            let widget = id(self.current().id, axis);
            let response = ui.interact(track, widget, Sense::click_and_drag());
            if response.hovered() && !ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                let delta = ui.input(|i| i.smooth_scroll_delta);
                self.current_mut().pan[axis] = (pan
                    + if axis == 0 && delta.x == 0. {
                        delta.y
                    } else {
                        delta[axis]
                    })
                .clamp(-limit, limit);
            }
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if response.is_pointer_button_down_on()
                && let Some(pointer) = pointer
            {
                let anchor = widget.with("grab");
                if ui.input(|i| i.pointer.primary_pressed()) {
                    let grab = if thumb.contains(pointer) {
                        pointer[axis] - thumb.min[axis]
                    } else {
                        thumb_length * 0.5
                    };
                    ui.ctx().data_mut(|d| d.insert_temp(anchor, grab));
                    response.request_focus();
                }
                let grab = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(anchor))
                    .unwrap_or(thumb_length * 0.5);
                let position = (pointer[axis] - track.min[axis] - grab).clamp(0., travel);
                self.current_mut().pan[axis] = if travel > 0. {
                    limit - position / travel * (2. * limit)
                } else {
                    0.
                };
            }
            if response.has_focus() {
                let delta = ui.input(|i| {
                    let keys = if axis == 0 {
                        [egui::Key::ArrowLeft, egui::Key::ArrowRight]
                    } else {
                        [egui::Key::ArrowUp, egui::Key::ArrowDown]
                    };
                    let mut delta = (i.key_pressed(keys[0]) as i32 - i.key_pressed(keys[1]) as i32)
                        as f32
                        * 32.;
                    delta += (i.key_pressed(egui::Key::PageUp) as i32
                        - i.key_pressed(egui::Key::PageDown) as i32)
                        as f32
                        * visible
                        * 0.9;
                    delta
                });
                self.current_mut().pan[axis] =
                    (self.current().pan[axis] + delta).clamp(-limit, limit);
                if ui.input(|i| i.key_pressed(egui::Key::Home)) {
                    self.current_mut().pan[axis] = limit;
                }
                if ui.input(|i| i.key_pressed(egui::Key::End)) {
                    self.current_mut().pan[axis] = -limit;
                }
            }
            let at = if limit > 0. {
                (limit - self.current().pan[axis]) / (2. * limit) * travel
            } else {
                0.
            };
            ui.painter()
                .rect_filled(track, 4, Color32::from_rgb(33, 33, 40));
            ui.painter().rect_filled(
                thumb_rect(at),
                4,
                Color32::from_gray(if response.is_pointer_button_down_on() {
                    140
                } else if response.hovered() {
                    115
                } else {
                    80
                }),
            );
            response.on_hover_text(if axis == 0 {
                "Scroll canvas horizontally"
            } else {
                "Scroll canvas vertically"
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rails_follow_each_overflow_axis_and_account_for_the_other_rail() {
        let bounds = Rect::from_min_size(pos2(62., 175.), vec2(500., 300.));
        for (size, wanted) in [
            (vec2(400., 200.), [false, false]),
            (vec2(600., 200.), [true, false]),
            (vec2(400., 400.), [false, true]),
            (vec2(600., 400.), [true, true]),
            (vec2(495., 400.), [true, true]),
            (vec2(600., 295.), [true, true]),
        ] {
            let result = layout(bounds, size, false);
            assert_eq!(result.rails.map(|rail| rail.is_some()), wanted);
            for rail in result.rails.into_iter().flatten() {
                assert!(!result.view.intersects(rail));
                assert!(bounds.contains_rect(rail));
            }
            assert_eq!(layout(bounds, size, true).rails, [None, None]);
        }
    }
}
