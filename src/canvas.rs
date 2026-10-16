use crate::{
    app::{ACCENT, PaletteApp},
    icons::{self, Icon},
    session::Tool,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use palette_core::{brush::Brush, raster::Raster};

/// Convert logical display points to image coordinates. egui already accounts
/// for OS DPI; multiplying by pixels_per_point here would apply scaling twice.
fn image_point(point: Pos2, canvas: Rect, zoom: f32) -> [f32; 2] {
    [
        (point.x - canvas.left()) / zoom,
        (point.y - canvas.top()) / zoom,
    ]
}

fn draw_shape(
    raster: &mut Raster,
    brush: Brush,
    tool: Tool,
    from: [f32; 2],
    to: [f32; 2],
    filled: bool,
) {
    if tool == Tool::Line {
        brush.segment(raster, from, to);
        return;
    }
    let left = from[0].min(to[0]);
    let right = from[0].max(to[0]);
    let top = from[1].min(to[1]);
    let bottom = from[1].max(to[1]);
    if filled {
        let cx = (left + right) * 0.5;
        let cy = (top + bottom) * 0.5;
        let rx = ((right - left) * 0.5).max(0.5);
        let ry = ((bottom - top) * 0.5).max(0.5);
        for y in (top.max(0.0) as u32)..(bottom.ceil().max(0.0) as u32).min(raster.height()) {
            for x in (left.max(0.0) as u32)..(right.ceil().max(0.0) as u32).min(raster.width()) {
                if tool == Tool::Rectangle
                    || ((x as f32 + 0.5 - cx) / rx).powi(2) + ((y as f32 + 0.5 - cy) / ry).powi(2)
                        <= 1.0
                {
                    raster.set(
                        x,
                        y,
                        palette_core::composite::over(
                            raster.get(x, y),
                            brush.color,
                            brush.opacity,
                            palette_core::document::BlendMode::Normal,
                        ),
                    );
                }
            }
        }
    } else if tool == Tool::Rectangle {
        let corners = [
            [left, top],
            [right, top],
            [right, bottom],
            [left, bottom],
            [left, top],
        ];
        for pair in corners.windows(2) {
            brush.segment(raster, pair[0], pair[1]);
        }
    } else {
        let rx = (right - left) * 0.5;
        let ry = (bottom - top) * 0.5;
        let steps = ((rx + ry) * 3.0).ceil().clamp(32.0, 4096.0) as u32;
        let point = |i: u32| {
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            [
                (left + right) * 0.5 + a.cos() * rx,
                (top + bottom) * 0.5 + a.sin() * ry,
            ]
        };
        for i in 0..steps {
            brush.segment(raster, point(i), point(i + 1));
        }
    }
}

impl PaletteApp {
    pub fn canvas(&mut self, ui: &mut Ui) {
        let bounds = ui.max_rect();
        let left_width = 62.0;
        let right_width = if self.show_layers { 180.0 } else { 12.0 };
        let view = Rect::from_min_max(
            pos2(bounds.left() + left_width, bounds.top()),
            pos2(bounds.right() - right_width, bounds.bottom()),
        );
        self.canvas_rect = view;
        let response = ui.interact(
            view,
            ui.scope_id().with("canvas-view"),
            Sense::click_and_drag(),
        );
        if self.current().fit {
            let size = vec2(
                self.current().document.width as f32,
                self.current().document.height as f32,
            );
            self.current_mut().zoom = ((view.width() - 64.0) / size.x)
                .min((view.height() - 64.0) / size.y)
                .clamp(0.05, 32.0);
        }
        let ctx = ui.ctx().clone();
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
        let hovered = pointer.is_some_and(|p| view.contains(p));
        if hovered {
            if ctrl && scroll.y != 0.0 {
                let session = self.current_mut();
                let old_zoom = session.zoom;
                let new_zoom = (old_zoom * (scroll.y * 0.005).exp()).clamp(0.05, 32.0);
                // Keep the image point under the cursor fixed while zoom changes.
                if let Some(point) = pointer {
                    session.pan = (point - view.center())
                        - ((point - view.center()) - session.pan) * (new_zoom / old_zoom);
                }
                session.zoom = new_zoom;
                session.fit = false;
            } else if !ctrl && scroll != Vec2::ZERO {
                self.current_mut().pan += scroll;
            }
            if middle || (space && down) {
                self.current_mut().pan += delta;
                ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        let zoom = self.current().zoom;
        let size = vec2(
            self.current().document.width as f32,
            self.current().document.height as f32,
        ) * zoom;
        let mut origin = view.center() - size * 0.5 + self.current().pan;
        // Snap display origin to physical pixels; coordinates still use logical points.
        let dpi = ctx.pixels_per_point();
        origin.x = (origin.x * dpi).round() / dpi;
        origin.y = (origin.y * dpi).round() / dpi;
        let canvas = Rect::from_min_size(origin, size);
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
        if let Some(texture) = &self.current().texture {
            painter.image(
                texture.id(),
                canvas,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        painter.rect_stroke(
            canvas,
            0,
            Stroke::new(1.0, Color32::from_gray(107)),
            StrokeKind::Outside,
        );

        self.cursor = pointer
            .filter(|p| view.contains(*p) && canvas.contains(*p))
            .map(|p| {
                let point = image_point(p, canvas, zoom);
                [point[0].floor() as u32, point[1].floor() as u32]
            });
        let drawing_allowed = self.close_pending_none()
            && (!self.current().document.active().locked
                || (!self.tool.paints() && !self.tool.shape()));
        if hovered
            && pressed
            && !space
            && !middle
            && canvas.contains(pointer.unwrap_or(Pos2::ZERO))
            && drawing_allowed
        {
            self.stroke_background = ctx.input(|i| i.pointer.secondary_pressed());
            let point = image_point(pointer.expect("hovered pointer"), canvas, zoom);
            match self.tool {
                Tool::Magnifier => {
                    self.current_mut().zoom = (zoom * 2.0).min(32.0);
                    self.current_mut().fit = false;
                }
                Tool::Picker => {
                    if let Some(rendered) = &self.current().rendered {
                        self.brush.color = rendered.get(point[0] as u32, point[1] as u32);
                    }
                }
                tool if tool.paints() || tool.shape() => {
                    if tool.paints() {
                        let session = self.current_mut();
                        session.history.begin(tool.label(), &session.document);
                    }
                    self.stroke = Some((point, point));
                    self.gesture_session = Some(self.current().id);
                    if tool.paints() {
                        self.paint_segment(point, point);
                    }
                }
                _ => {}
            }
        }
        if let Some((start, last)) = self.stroke
            && self.gesture_session == Some(self.current().id)
        {
            let point = pointer
                .map(|p| image_point(p, canvas, zoom))
                .unwrap_or(last);
            if down && point != last {
                if self.tool.paints() {
                    self.paint_segment(last, point);
                }
                self.stroke = Some((start, point));
            }
            if self.tool.shape() {
                let a = origin + vec2(start[0], start[1]) * zoom;
                let b = origin + vec2(point[0], point[1]) * zoom;
                let rgba = if self.stroke_background {
                    self.background.to_srgba_unmultiplied()
                } else {
                    self.brush.color
                };
                let color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
                let stroke = Stroke::new((self.brush.diameter * zoom).max(1.0), color);
                match self.tool {
                    Tool::Line => {
                        painter.line_segment([a, b], stroke);
                    }
                    Tool::Rectangle => {
                        if self.shape_fill {
                            painter.rect_filled(Rect::from_two_pos(a, b), 0, color);
                        } else {
                            painter.rect_stroke(
                                Rect::from_two_pos(a, b),
                                0,
                                stroke,
                                StrokeKind::Middle,
                            );
                        }
                    }
                    Tool::Ellipse => {
                        let rect = Rect::from_two_pos(a, b);
                        painter.add(egui::epaint::EllipseShape {
                            angle: 0.0,
                            center: rect.center(),
                            radius: rect.size() * 0.5,
                            fill: if self.shape_fill {
                                color
                            } else {
                                Color32::TRANSPARENT
                            },
                            stroke: if self.shape_fill {
                                Stroke::NONE
                            } else {
                                stroke
                            },
                        });
                    }
                    _ => {}
                }
            }
            if released || !down {
                if self.tool.shape() {
                    let label = self.tool.label();
                    let session = self.current_mut();
                    session.history.begin(label, &session.document);
                    let mut brush = self.brush;
                    if self.stroke_background {
                        brush.color = self.background.to_srgba_unmultiplied();
                    }
                    brush.erase = false;
                    let tool = self.tool;
                    let filled = self.shape_fill;
                    let offset = self.current().document.active().offset;
                    let a = [start[0] - offset[0] as f32, start[1] - offset[1] as f32];
                    let b = [point[0] - offset[0] as f32, point[1] - offset[1] as f32];
                    draw_shape(
                        &mut self.current_mut().document.active_mut().source,
                        brush,
                        tool,
                        a,
                        b,
                        filled,
                    );
                    self.current_mut().invalidate();
                }
                self.stroke = None;
            }
        }
        if self.cursor.is_some() && self.tool.paints() && !space {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            if let Some(point) = pointer {
                let radius = (if self.tool == Tool::Pencil {
                    1.0
                } else {
                    self.brush.diameter
                }) * zoom
                    * 0.5;
                painter.circle_stroke(point, radius.max(1.0), Stroke::new(2.0, Color32::BLACK));
                painter.circle_stroke(point, radius.max(1.0), Stroke::new(1.0, Color32::WHITE));
            }
        }
        // The bottom-right handle performs a reversible canvas-bounds resize.
        let handle = Rect::from_center_size(canvas.right_bottom() + vec2(3.0, 3.0), vec2(9.0, 9.0));
        if view.contains(handle.center()) {
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
        let sliders = Rect::from_min_max(
            pos2(bounds.left() + 12.0, bounds.top() + 24.0),
            pos2(bounds.left() + 48.0, bounds.bottom() - 24.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(sliders), |ui| {
            self.brush_sliders(ui)
        });
        if self.show_layers {
            let panel = Rect::from_min_max(
                pos2(bounds.right() - 168.0, bounds.top() + 24.0),
                pos2(bounds.right() - 12.0, bounds.bottom() - 24.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(panel), |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(40, 40, 47))
                    .corner_radius(7)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.set_width(140.0);
                        ui.set_max_height(panel.height() - 16.0);
                        egui::ScrollArea::vertical()
                            .id_salt("layers-scroll")
                            .show(ui, |ui| self.layers(ui));
                    });
            });
        }
        response.on_hover_text("Ctrl + wheel to zoom · Middle mouse or Space + drag to pan");
    }

    fn paint_segment(&mut self, from: [f32; 2], to: [f32; 2]) {
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
        let from = [from[0] - offset[0] as f32, from[1] - offset[1] as f32];
        let to = [to[0] - offset[0] as f32, to[1] - offset[1] as f32];
        brush.segment(
            &mut self.current_mut().document.active_mut().source,
            from,
            to,
        );
        self.current_mut().invalidate();
    }

    fn brush_sliders(&mut self, ui: &mut Ui) {
        let height = ((ui.available_height() - 24.0) / 3.0).max(60.0);
        let enabled = self.tool.paints() || self.tool.shape();
        for (index, (icon, label)) in [
            (Icon::Brush, "Brush size"),
            (Icon::Opacity, "Brush opacity"),
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
                    ui.add_enabled_ui(enabled, |ui| {
                        ui.spacing_mut().slider_width = (height - 62.0).max(14.0);
                        ui.spacing_mut().slider_rail_height = 3.0;
                        ui.visuals_mut().selection.bg_fill = ACCENT;
                        ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(135);
                        ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_gray(135);
                        ui.visuals_mut().widgets.active.bg_fill = Color32::from_gray(135);
                        ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(3.0, ACCENT);
                        ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(3.0, ACCENT);
                        ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(3.0, ACCENT);
                        let value = match index {
                            0 => &mut self.brush.diameter,
                            1 => &mut self.brush.opacity,
                            _ => &mut self.brush.hardness,
                        };
                        let range = if index == 0 { 1.0..=256.0 } else { 0.0..=1.0 };
                        ui.add(
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
                        }
                    });
                });
            ui.add_space(6.0);
        }
    }

    fn resize_canvas(&mut self, dimensions: [u32; 2]) {
        self.current_mut().command("Resize canvas", |doc| {
            for layer in &mut doc.layers {
                if layer.offset == [0, 0]
                    && layer.source.width() == doc.width
                    && layer.source.height() == doc.height
                {
                    let mut expanded = Raster::new(dimensions[0], dimensions[1], [0; 4])
                        .expect("bounded dimensions");
                    for y in 0..dimensions[1].min(doc.height) {
                        for x in 0..dimensions[0].min(doc.width) {
                            expanded.set(x, y, layer.source.get(x, y));
                        }
                    }
                    layer.source = expanded;
                }
            }
            doc.width = dimensions[0];
            doc.height = dimensions[1];
        });
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
