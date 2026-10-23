//! Original geometric icons, drawn at the current display scale.
use eframe::egui::{
    self, Color32, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2,
};

#[derive(Clone, Copy)]
pub enum Icon {
    Tools,
    Colors,
    Image,
    Shapes,
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
    SelectBrush,
    Wand,
    Lasso,
    Combine(palette_core::selection::Combine),
    InvertSelection,
    Crop,
    Resize,
    Rotate,
    Flip,
    Text,
    TextBold,
    TextItalic,
    TextUnderline,
    TextStrike,
    TextAlign(palette_core::text::Alignment),
    Fill,
    Line,
    Rectangle,
    Ellipse,
    Layers,
    Effects,
    Filters,
    Blend,
    Save,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Delete,
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
        Icon::Copy => {
            box_at(at(0.28, 0.08), at(0.87, 0.72));
            line([0.17, 0.28], [0.08, 0.28]);
            line([0.08, 0.28], [0.08, 0.92]);
            line([0.08, 0.92], [0.67, 0.92]);
            line([0.67, 0.92], [0.67, 0.82]);
        }
        Icon::Cut => {
            p.circle_stroke(at(0.24, 0.78), rect.width() * 0.16, s);
            p.circle_stroke(at(0.76, 0.78), rect.width() * 0.16, s);
            line([0.33, 0.64], [0.84, 0.04]);
            line([0.67, 0.64], [0.16, 0.04]);
        }
        Icon::Paste => {
            box_at(at(0.15, 0.18), at(0.85, 0.95));
            p.rect_filled(Rect::from_min_max(at(0.33, 0.05), at(0.67, 0.3)), 1, color);
        }
        Icon::Delete => {
            line([0.06, 0.22], [0.94, 0.22]);
            box_at(at(0.22, 0.25), at(0.78, 0.94));
            box_at(at(0.35, 0.06), at(0.65, 0.22));
            line([0.41, 0.4], [0.41, 0.78]);
            line([0.59, 0.4], [0.59, 0.78]);
        }
        Icon::Image => {
            box_at(at(0.08, 0.14), at(0.9, 0.86));
            p.circle_filled(at(0.3, 0.35), rect.width() * 0.07, color);
            p.add(egui::Shape::line(
                vec![
                    at(0.12, 0.76),
                    at(0.43, 0.48),
                    at(0.61, 0.66),
                    at(0.75, 0.52),
                    at(0.88, 0.65),
                ],
                s,
            ));
        }
        Icon::Shapes => {
            p.circle_stroke(at(0.38, 0.36), rect.width() * 0.27, s);
            p.rect_filled(
                Rect::from_min_max(at(0.45, 0.42), at(0.92, 0.9)),
                1,
                Color32::from_rgb(39, 39, 47),
            );
            box_at(at(0.45, 0.42), at(0.92, 0.9));
        }
        Icon::Tools => {
            draw(ui, rect.shrink(2.), Icon::Pencil, color);
            line([0.15, 0.1], [0.9, 0.85]);
            p.circle_stroke(at(0.15, 0.1), rect.width() * 0.12, s);
        }
        Icon::Colors => {
            p.circle_stroke(at(0.5, 0.5), rect.width() * 0.43, s);
            for (x, y, c) in [
                (0.3, 0.3, Color32::from_rgb(240, 85, 85)),
                (0.6, 0.25, Color32::from_rgb(245, 214, 73)),
                (0.75, 0.5, Color32::from_rgb(85, 195, 135)),
                (0.3, 0.65, Color32::from_rgb(90, 160, 235)),
            ] {
                p.circle_filled(at(x, y), rect.width() * 0.075, c);
            }
        }
        Icon::Filters => {
            for (y, x) in [(0.2, 0.35), (0.5, 0.7), (0.8, 0.45)] {
                line([0.05, y], [0.95, y]);
                p.rect_filled(
                    Rect::from_center_size(at(x, y), rect.size() * egui::vec2(0.16, 0.24)),
                    1,
                    color,
                );
            }
        }
        Icon::Blend => {
            p.circle_stroke(at(0.35, 0.5), rect.width() * 0.3, s);
            p.circle_filled(
                at(0.65, 0.5),
                rect.width() * 0.3,
                color.gamma_multiply(0.45),
            );
            p.circle_stroke(at(0.65, 0.5), rect.width() * 0.3, s);
        }
        Icon::SelectBrush => {
            draw(ui, rect, Icon::Select, color);
            draw(ui, rect.shrink(rect.width() * 0.18), Icon::Pencil, color);
        }
        Icon::Lasso => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.1, 0.35),
                    at(0.2, 0.1),
                    at(0.65, 0.05),
                    at(0.95, 0.3),
                    at(0.8, 0.65),
                    at(0.35, 0.75),
                ],
                s,
            ));
            line([0.35, 0.75], [0.25, 1.]);
        }
        Icon::Combine(mode) => {
            use palette_core::selection::Combine;
            let a = Rect::from_min_max(at(0.05, 0.1), at(0.65, 0.7));
            let b = Rect::from_min_max(at(0.35, 0.4), at(0.95, 1.));
            if matches!(mode, Combine::Replace) {
                p.rect_stroke(a, 0, s, StrokeKind::Inside);
            } else {
                p.rect_stroke(a, 0, s, StrokeKind::Inside);
                p.rect_stroke(b, 0, s, StrokeKind::Inside);
                match mode {
                    Combine::Add => {
                        line([0.75, 0.05], [0.75, 0.3]);
                        line([0.625, 0.175], [0.875, 0.175]);
                    }
                    Combine::Subtract => {
                        line([0.625, 0.175], [0.875, 0.175]);
                    }
                    Combine::Intersect => {
                        p.rect_filled(a.intersect(b), 0, color);
                    }
                    _ => {}
                }
            }
        }
        Icon::InvertSelection => {
            p.rect_stroke(
                Rect::from_min_max(at(0.05, 0.1), at(0.95, 0.9)),
                0,
                s,
                StrokeKind::Inside,
            );
            p.rect_filled(Rect::from_min_max(at(0.5, 0.1), at(0.95, 0.9)), 0, color);
            p.rect_filled(
                Rect::from_min_max(at(0.5, 0.35), at(0.7, 0.65)),
                0,
                Color32::from_gray(40),
            );
        }
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
        Icon::Wand => {
            let p = ui.painter();
            p.line_segment(
                [rect.left_bottom(), rect.center() + egui::vec2(2., -2.)],
                Stroke::new(1.5, color),
            );
            let star = rect.right_top() + egui::vec2(-3., 3.);
            for d in [egui::vec2(3., 0.), egui::vec2(0., 3.)] {
                p.line_segment([star - d, star + d], Stroke::new(1., color));
            }
            p.circle_filled(rect.left_top() + egui::vec2(1., 2.), 1., color);
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
        Icon::TextBold | Icon::TextStrike => {
            let bold = matches!(icon, Icon::TextBold);
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if bold { "B" } else { "S" },
                egui::FontId::new(
                    rect.height() * 1.2,
                    if bold {
                        egui::FontFamily::Name("Dialog Semibold".into())
                    } else {
                        egui::FontFamily::Proportional
                    },
                ),
                color,
            );
            if !bold {
                line([0., 0.55], [1., 0.55]);
            }
        }
        Icon::TextItalic => {
            line([0.3, 0.1], [0.9, 0.1]);
            line([0.6, 0.1], [0.4, 0.9]);
            line([0.1, 0.9], [0.7, 0.9]);
        }
        Icon::TextUnderline => {
            p.add(egui::Shape::line(
                vec![
                    at(0.2, 0.05),
                    at(0.2, 0.55),
                    at(0.27, 0.72),
                    at(0.5, 0.8),
                    at(0.73, 0.72),
                    at(0.8, 0.55),
                    at(0.8, 0.05),
                ],
                s,
            ));
            line([0.05, 1.], [0.95, 1.]);
        }
        Icon::TextAlign(alignment) => {
            use palette_core::text::Alignment;
            for (y, width) in [(0.18, 0.95), (0.5, 0.62), (0.82, 0.95)] {
                let x = match alignment {
                    Alignment::Left => 0.025,
                    Alignment::Center => (1. - width) * 0.5,
                    Alignment::Right => 0.975 - width,
                };
                line([x, y], [x + width, y]);
            }
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
        Icon::Effects => {
            p.add(egui::Shape::closed_line(
                vec![
                    at(0.38, 0.04),
                    at(0.49, 0.36),
                    at(0.81, 0.47),
                    at(0.49, 0.58),
                    at(0.38, 0.94),
                    at(0.27, 0.58),
                    at(0.01, 0.47),
                    at(0.27, 0.36),
                ],
                s,
            ));
            line([0.83, 0.01], [0.83, 0.25]);
            line([0.71, 0.13], [0.95, 0.13]);
        }
        Icon::Hardness => {
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
