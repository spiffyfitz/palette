//! Presentation-only selection animation and bounded stroke previews.
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
const DASH: f32 = 4.;
const PERIOD: f32 = 2. * DASH;
/// Shared contour preview used by sized shapes and multi-step construction tools.
pub(super) fn shape(
    painter: &egui::Painter,
    path: &palette_core::shapes::Path,
    style: palette_core::shapes::Style,
    origin: Pos2,
    zoom: f32,
) {
    let points: Vec<_> = path
        .points
        .iter()
        .map(|p| origin + egui::vec2(p[0], p[1]) * zoom)
        .collect();
    let color = |rgba: [u8; 4]| {
        Color32::from_rgba_unmultiplied(
            rgba[0],
            rgba[1],
            rgba[2],
            (f32::from(rgba[3]) * style.opacity).round() as u8,
        )
    };
    if let Some(fill) = style.fill.filter(|_| path.closed) {
        let mut mesh = egui::Mesh::default();
        for p in &points {
            mesh.colored_vertex(*p, color(fill));
        }
        for triangle in palette_core::shapes::triangles(path) {
            mesh.indices.extend(triangle.map(|i| i as u32));
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    if let Some(outline) = style.outline {
        painter.add(bounded_outline(
            points,
            path.closed,
            Stroke::new(style.width * zoom, color(outline)),
        ));
    }
}
/// Build an open, bevel-limited loop instead of epaint's unlimited closed-path
/// miters. Nearly opposite edges of a flat ellipse/triangle otherwise create
/// normals whose length approaches infinity. Degenerate points are removed first.
pub(super) fn bounded_outline(points: Vec<Pos2>, closed: bool, stroke: Stroke) -> egui::Shape {
    let mut clean: Vec<Pos2> = Vec::with_capacity(points.len() + 1);
    for point in points {
        if point.is_finite() && clean.last().is_none_or(|p| p.distance_sq(point) > 0.0001) {
            clean.push(point);
        }
    }
    if closed && clean.len() > 1 && clean[0].distance_sq(*clean.last().unwrap()) <= 0.0001 {
        clean.pop();
    }
    match clean.len() {
        0 => egui::Shape::Noop,
        1 => egui::Shape::circle_filled(clean[0], stroke.width * 0.5, stroke.color),
        2 => egui::Shape::line_segment([clean[0], clean[1]], stroke),
        _ => {
            if closed {
                clean.push(clean[0]);
            }
            egui::Shape::line(clean, stroke)
        }
    }
}
/// Clip a line before splitting dashes, so high zoom and offscreen selections do
/// not allocate thousands of invisible dash primitives. Distances stay anchored
/// to the original contour, preserving phase through clipping and around corners.
fn visible_span(a: Pos2, b: Pos2, clip: Rect) -> Option<(egui::Vec2, f32, f32, f32)> {
    let delta = b - a;
    let length = delta.length();
    if length <= 0.0001 {
        return None;
    }
    let dir = delta / length;
    let mut low = 0.0_f32;
    let mut high = length;
    for (position, velocity, min, max) in [
        (a.x, dir.x, clip.left(), clip.right()),
        (a.y, dir.y, clip.top(), clip.bottom()),
    ] {
        if velocity.abs() < 0.00001 {
            if position < min || position > max {
                return None;
            }
        } else {
            let t0 = (min - position) / velocity;
            let t1 = (max - position) / velocity;
            low = low.max(t0.min(t1));
            high = high.min(t0.max(t1));
        }
    }
    (high > low).then_some((dir, length, low, high))
}
pub(super) fn phase(seconds: f64) -> f32 {
    (seconds * 12.).rem_euclid(f64::from(PERIOD)) as f32
}
/// Return only black dash segments; white under-strokes provide the alternating
/// contrast. Phase is in logical display points and independent of image zoom.
pub(super) fn dashes(points: &[Pos2], phase: f32, clip: Rect) -> Vec<[Pos2; 2]> {
    let mut distance = 0.;
    let mut segments = Vec::new();
    for pair in points.windows(2) {
        if let Some((dir, _, mut from, end)) = visible_span(pair[0], pair[1], clip) {
            while from < end {
                let cycle = (distance + from - phase).rem_euclid(PERIOD);
                let black = cycle < DASH;
                let until = from + (if black { DASH - cycle } else { PERIOD - cycle }).max(0.001);
                let to = until.min(end);
                if black {
                    segments.push([pair[0] + dir * from, pair[0] + dir * to]);
                }
                from = to;
            }
        }
        distance += pair[0].distance(pair[1]);
    }
    segments
}
pub(super) fn ants(painter: &egui::Painter, points: &[Pos2], phase: f32) -> bool {
    ants_alpha(painter, points, phase, 255)
}
pub(super) fn ants_alpha(painter: &egui::Painter, points: &[Pos2], phase: f32, alpha: u8) -> bool {
    let clip = painter.clip_rect();
    let mut visible = false;
    for pair in points.windows(2) {
        if let Some((dir, _, from, to)) = visible_span(pair[0], pair[1], clip) {
            painter.line_segment(
                [pair[0] + dir * from, pair[0] + dir * to],
                Stroke::new(1., Color32::from_white_alpha(alpha)),
            );
            visible = true;
        }
    }
    for pair in dashes(points, phase, clip) {
        painter.line_segment(pair, Stroke::new(1., Color32::from_black_alpha(alpha)));
    }
    visible
}

/// Close the visible selected region along viewport/canvas edges. Without this
/// continuation a zoomed inverted selection loses its outer contour offscreen.
/// Sampling only the four borders is O(visible width + height), not a mask copy.
pub(super) fn viewport_ants(
    painter: &egui::Painter,
    mask: &palette_core::raster::Coverage,
    canvas: Rect,
    zoom: f32,
    phase: f32,
) -> bool {
    let visible = canvas.intersect(painter.clip_rect());
    if !visible.is_positive() {
        return false;
    }
    let inset = 0.75 / painter.ctx().pixels_per_point();
    let border = visible.shrink(inset);
    let mut drawn = false;
    let mut perimeter = 0.;
    for (a, b) in [
        (border.left_top(), border.right_top()),
        (border.right_top(), border.right_bottom()),
        (border.right_bottom(), border.left_bottom()),
        (border.left_bottom(), border.left_top()),
    ] {
        let length = a.distance(b);
        // Sample at physical display pixels, rather than shifting image-space
        // bins: partial pixels at a clipped/high-zoom edge stay within one pixel.
        let count = (length * painter.ctx().pixels_per_point()).ceil().max(1.) as usize;
        let mut first = None;
        for index in 0..=count {
            let selected = index < count && {
                let p = a.lerp(b, (index as f32 + 0.5) / count as f32);
                let x = ((p.x - canvas.left()) / zoom)
                    .floor()
                    .clamp(0., mask.width as f32 - 1.) as u32;
                let y = ((p.y - canvas.top()) / zoom)
                    .floor()
                    .clamp(0., mask.height as f32 - 1.) as u32;
                mask.at(x, y) >= 0.5
            };
            if selected && first.is_none() {
                first = Some(index);
            }
            if !selected && let Some(start) = first.take() {
                drawn |= ants(
                    painter,
                    &[
                        a.lerp(b, start as f32 / count as f32),
                        a.lerp(b, index as f32 / count as f32),
                    ],
                    phase - perimeter - length * start as f32 / count as f32,
                );
            }
        }
        perimeter += length;
    }
    drawn
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thin_preview_tessellation_remains_finite_and_within_stroke_bounds() {
        use palette_core::shapes::{self, Kind};
        for kind in [
            Kind::Ellipse,
            Kind::RoundedRectangle,
            Kind::Triangle,
            Kind::Star,
            Kind::RightArrow,
            Kind::Heart,
            Kind::CloudCallout,
        ] {
            for size in [
                [0., 200.],
                [0.00001, 200.],
                [0.1, 200.],
                [200., 0.00001],
                [200., 0.],
                [0., 0.],
            ] {
                let start = egui::pos2(300., 300.);
                let end = start + egui::vec2(size[0], size[1]);
                let path = shapes::path(kind, [start.x, start.y], [end.x, end.y]);
                let shape = bounded_outline(
                    path.points.iter().map(|p| egui::pos2(p[0], p[1])).collect(),
                    path.closed,
                    Stroke::new(20., Color32::BLACK),
                );
                let ctx = egui::Context::default();
                let mut warmup = ctx.run_ui(egui::RawInput::default(), |_| {});
                warmup.textures_delta.clear();
                let primitives = ctx.tessellate(
                    vec![egui::epaint::ClippedShape::new(Rect::EVERYTHING, shape)],
                    1.,
                );
                let bounds = Rect::from_two_pos(start, end).expand(24.);
                for primitive in primitives {
                    if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                        for vertex in mesh.vertices {
                            assert!(vertex.pos.is_finite(), "{kind:?} {size:?}");
                            assert!(
                                bounds.contains(vertex.pos),
                                "{kind:?} {size:?}: {:?}",
                                vertex.pos
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn ants_move_keep_corner_phase_and_clip_large_offscreen_edges() {
        let points = [
            egui::pos2(0., 0.),
            egui::pos2(10., 0.),
            egui::pos2(10., 10.),
        ];
        let clip = Rect::from_min_max(egui::pos2(-1., -1.), egui::pos2(20., 20.));
        let a = dashes(&points, phase(0.), clip);
        let b = dashes(&points, phase(0.2), clip);
        assert_ne!(a, b);
        assert!(a.contains(&[egui::pos2(10., 0.), egui::pos2(10., 2.)]));
        let clipped = dashes(
            &[egui::pos2(-100000., 5.), egui::pos2(100000., 5.)],
            0.,
            clip,
        );
        assert!(clipped.len() < 5);
        assert!(clipped.iter().flatten().all(|p| clip.contains(*p)));
    }
}
