//! Original geometric icons, drawn at the current display scale.
use eframe::egui::{
    self, Color32, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2,
};

#[derive(Clone, Copy)]
pub enum Icon {
    New,
    Move,
    Open,
    Lock,
    Width,
    Curve,
    Shape(palette_core::shapes::Kind),
    Polygon,
    Brush,
    Pencil,
    Eraser,
    Picker,
    Zoom,
    Select,
    Crop,
    Resize,
    Rotate,
    Flip,
    Text,
    Fill,
    Line,
    Rectangle,
    Ellipse,
    Layers,
    Effects,
    Save,
    Undo,
    Redo,
    Fit,
    Hardness,
    Opacity,
}

pub fn button(
    ui: &mut Ui,
    icon: Icon,
    tooltip: &str,
    selected: bool,
    enabled: bool,
    size: Vec2,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let color = if !enabled {
            Color32::from_gray(91)
        } else {
            Color32::from_gray(230)
        };
        let hover = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            crate::interface::HOVER_SECONDS,
        );
        let pressed = ui.ctx().animate_bool_with_time(
            response.id.with("pressed"),
            response.is_pointer_button_down_on(),
            0.07,
        );
        if selected || hover > 0.0 {
            let base = if selected { 52.0 } else { 39.0 };
            ui.painter().rect_filled(
                rect,
                4,
                Color32::from_gray((base + hover * 10.0 - pressed * 6.0) as u8),
            );
            if selected {
                ui.painter().rect_stroke(
                    rect.shrink(0.5),
                    4,
                    Stroke::new(1.0, Color32::from_rgb(83, 221, 225)),
                    StrokeKind::Inside,
                );
            }
        }
        let extent = (size.x.min(size.y) - 14.0).clamp(12.0, 24.0);
        draw(
            ui,
            Rect::from_center_size(rect.center(), Vec2::splat(extent - pressed * 1.5)),
            icon,
            color,
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::Role::Button, enabled, selected, tooltip)
        });
        response
            .on_hover_text(tooltip)
            .on_disabled_hover_text(tooltip)
    })
    .inner
}

pub fn draw(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    let p = ui.painter();
    let s = Stroke::new(1.3, color);
    let at = |x: f32, y: f32| {
        pos2(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        )
    };
    let line = |a: [f32; 2], b: [f32; 2]| {
        p.line_segment([at(a[0], a[1]), at(b[0], b[1])], s);
    };
    let box_at = |a: Pos2, b: Pos2| {
        p.rect_stroke(Rect::from_two_pos(a, b), 0, s, StrokeKind::Inside);
    };
    match icon {
        Icon::Move => {
            line([0.5, 0.05], [0.5, 0.95]);
            line([0.05, 0.5], [0.95, 0.5]);
            for (a, b) in [
                ([0.5, 0.05], [0.32, 0.23]),
                ([0.5, 0.05], [0.68, 0.23]),
                ([0.5, 0.95], [0.32, 0.77]),
                ([0.5, 0.95], [0.68, 0.77]),
                ([0.05, 0.5], [0.23, 0.32]),
                ([0.05, 0.5], [0.23, 0.68]),
                ([0.95, 0.5], [0.77, 0.32]),
                ([0.95, 0.5], [0.77, 0.68]),
            ] {
                line(a, b);
            }
        }
        Icon::New => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.2, 0.1),
                    at(0.62, 0.1),
                    at(0.82, 0.3),
                    at(0.82, 0.9),
                    at(0.2, 0.9),
                ],
                s,
            ));
            p.line_segment([at(0.62, 0.1), at(0.62, 0.3)], s);
            p.line_segment([at(0.62, 0.3), at(0.82, 0.3)], s);
        }
        Icon::Open => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.08, 0.85),
                    at(0.08, 0.25),
                    at(0.4, 0.25),
                    at(0.5, 0.4),
                    at(0.9, 0.4),
                    at(0.82, 0.85),
                ],
                s,
            ));
            p.line_segment([at(0.08, 0.55), at(0.9, 0.55)], s);
        }
        Icon::Lock => {
            p.rect_stroke(
                Rect::from_min_max(at(0.2, 0.42), at(0.8, 0.9)),
                1,
                s,
                StrokeKind::Inside,
            );
            p.add(egui::Shape::line(
                vec![
                    at(0.32, 0.45),
                    at(0.32, 0.25),
                    at(0.5, 0.12),
                    at(0.68, 0.25),
                    at(0.68, 0.45),
                ],
                s,
            ));
        }

        Icon::Width => {
            for (index, width) in [1.0, 2.0, 3.0].iter().enumerate() {
                let y = 0.25 + index as f32 * 0.25;
                p.line_segment([at(0.1, y), at(0.9, y)], Stroke::new(*width, color));
            }
        }
        Icon::Shape(kind) => {
            let path = palette_core::shapes::path(kind, [0.02, 0.02], [0.98, 0.98]);
            let mut points: Vec<_> = path.points.iter().map(|[x, y]| at(*x, *y)).collect();
            if path.closed && !points.is_empty() {
                points.push(points[0]);
            }
            p.add(egui::Shape::line(points, s));
        }
        Icon::Polygon => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.2, 0.05),
                    at(0.6, 0.05),
                    at(0.4, 0.5),
                    at(0.9, 0.5),
                    at(0.7, 0.95),
                    at(0.05, 0.95),
                ],
                s,
            ));
        }
        Icon::Curve => {
            let points = (0..24)
                .map(|i| {
                    let x = i as f32 / 23.0;
                    at(x, 0.5 + (x * std::f32::consts::TAU).sin() * 0.4)
                })
                .collect();
            p.add(egui::Shape::line(points, s));
        }
        Icon::Pencil | Icon::Picker => {
            line([0.15, 0.85], [0.8, 0.2]);
            line([0.15, 0.85], [0.4, 0.78]);
            line([0.4, 0.78], [0.93, 0.25]);
            line([0.8, 0.2], [0.93, 0.25]);
            line([0.15, 0.85], [0.22, 0.6]);
        }
        Icon::Brush => {
            box_at(at(0.22, 0.08), at(0.78, 0.42));
            line([0.22, 0.27], [0.78, 0.27]);
            line([0.3, 0.42], [0.42, 0.92]);
            line([0.7, 0.42], [0.58, 0.92]);
            line([0.42, 0.92], [0.58, 0.92]);
        }
        Icon::Eraser => {
            p.add(egui::Shape::convex_polygon(
                vec![at(0.12, 0.6), at(0.58, 0.1), at(0.96, 0.5), at(0.55, 0.94)],
                Color32::from_rgb(230, 156, 166),
                s,
            ));
            line([0.12, 0.6], [0.55, 0.94]);
            line([0.55, 0.94], [0.95, 0.94]);
        }
        Icon::Zoom | Icon::Fit => {
            p.circle_stroke(at(0.42, 0.42), rect.width() * 0.3, s);
            line([0.65, 0.65], [0.94, 0.94]);
            line([0.25, 0.42], [0.59, 0.42]);
            line([0.42, 0.25], [0.42, 0.59]);
        }
        Icon::Select => {
            for i in 0..4 {
                let a = i as f32 / 4.0;
                let b = a + 0.13;
                line([a, 0.0], [b, 0.0]);
                line([a, 1.0], [b, 1.0]);
                line([0.0, a], [0.0, b]);
                line([1.0, a], [1.0, b]);
            }
        }
        Icon::Crop => {
            line([0.25, 0.0], [0.25, 0.75]);
            line([0.25, 0.75], [1.0, 0.75]);
            line([0.0, 0.25], [0.75, 0.25]);
            line([0.75, 0.25], [0.75, 1.0]);
        }
        Icon::Resize => {
            box_at(at(0.1, 0.1), at(0.9, 0.9));
            line([0.4, 0.6], [0.9, 0.1]);
            line([0.55, 0.1], [0.9, 0.1]);
            line([0.9, 0.1], [0.9, 0.45]);
        }
        Icon::Rotate | Icon::Undo | Icon::Redo => {
            let flip = matches!(icon, Icon::Redo);
            let points: Vec<_> = (0..22)
                .map(|i| {
                    let angle = -2.7 + i as f32 * 0.16;
                    let x = 0.5 + angle.cos() * 0.37;
                    at(if flip { 1.0 - x } else { x }, 0.52 + angle.sin() * 0.37)
                })
                .collect();
            p.add(egui::Shape::line(points, s));
            line([0.12, 0.14], [0.12, 0.45]);
            line([0.12, 0.45], [0.4, 0.4]);
        }
        Icon::Flip => {
            line([0.5, 0.0], [0.5, 1.0]);
            p.add(egui::Shape::closed_line(
                vec![at(0.1, 0.8), at(0.4, 0.2), at(0.4, 0.8)],
                s,
            ));
            p.add(egui::Shape::closed_line(
                vec![at(0.6, 0.8), at(0.6, 0.2), at(0.9, 0.8)],
                s,
            ));
        }
        Icon::Text => {
            line([0.1, 0.9], [0.5, 0.05]);
            line([0.5, 0.05], [0.9, 0.9]);
            line([0.27, 0.6], [0.73, 0.6]);
        }
        Icon::Fill => {
            p.add(egui::Shape::closed_line(
                vec![at(0.1, 0.5), at(0.5, 0.1), at(0.85, 0.5), at(0.5, 0.9)],
                s,
            ));
            line([0.1, 0.5], [0.85, 0.5]);
        }
        Icon::Line => line([0.0, 0.0], [1.0, 1.0]),
        Icon::Rectangle => box_at(at(0.05, 0.1), at(0.95, 0.9)),
        Icon::Ellipse => {
            p.add(egui::epaint::EllipseShape::stroke(
                rect.center(),
                rect.size() * 0.45,
                s,
            ));
        }
        Icon::Layers => {
            for y in [0.25, 0.5, 0.75] {
                p.add(egui::Shape::line(
                    vec![
                        at(0.05, y),
                        at(0.5, y + 0.23),
                        at(0.95, y),
                        at(0.5, y - 0.23),
                        at(0.05, y),
                    ],
                    s,
                ));
            }
        }
        Icon::Effects | Icon::Hardness => {
            p.circle_stroke(rect.center(), rect.width() * 0.42, s);
            p.circle_filled(rect.center(), rect.width() * 0.18, color);
        }
        Icon::Opacity => {
            p.circle_stroke(rect.center(), rect.width() * 0.4, s);
            p.rect_filled(Rect::from_two_pos(at(0.2, 0.5), at(0.8, 0.65)), 0, color);
        }
        Icon::Save => {
            box_at(at(0.05, 0.05), at(0.95, 0.95));
            box_at(at(0.22, 0.05), at(0.7, 0.4));
            box_at(at(0.22, 0.58), at(0.78, 0.95));
        }
    }
}
