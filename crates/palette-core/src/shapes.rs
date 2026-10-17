//! Shared image-space shape geometry and antialiased rasterization.
use crate::{
    brush::{Brush, PaintStroke},
    composite,
    document::{BlendMode, Document},
    editing,
    raster::Raster,
};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Implemented drag-to-size shapes. These are raster tool choices, not stored objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Line,
    Rectangle,
    Ellipse,
    RoundedRectangle,
    Triangle,
    Diamond,
    Pentagon,
    Hexagon,
    Star,
    RightArrow,
    DownArrow,
    LeftArrow,
    UpArrow,
    RightTriangle,
    FourPointStar,
    SixPointStar,
    RectangularCallout,
    OvalCallout,
    CloudCallout,
    Heart,
    Lightning,
}
impl Kind {
    /// Human-readable tool label shared with the interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::RoundedRectangle => "Rounded rectangle",
            Self::Triangle => "Triangle",
            Self::Diamond => "Diamond",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Star => "Five-point star",
            Self::RightArrow => "Right arrow",
            Self::DownArrow => "Down arrow",
            Self::LeftArrow => "Left arrow",
            Self::UpArrow => "Up arrow",
            Self::RightTriangle => "Right triangle",
            Self::FourPointStar => "Four-point star",
            Self::SixPointStar => "Six-point star",
            Self::RectangularCallout => "Rectangular callout",
            Self::OvalCallout => "Oval callout",
            Self::CloudCallout => "Cloud callout",
            Self::Heart => "Heart",
            Self::Lightning => "Lightning",
        }
    }
}
/// Independent foreground outline and background fill, with straight RGBA colors.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub outline: Option<[u8; 4]>,
    pub fill: Option<[u8; 4]>,
    pub width: f32,
    pub opacity: f32,
}
/// Shared contour used by the UI preview and CPU rasterizer.
#[derive(Clone, Debug)]
pub struct Path {
    pub points: Vec<[f32; 2]>,
    pub closed: bool,
}

/// Shift constrains closed shapes to a square and lines to 45-degree increments.
pub fn constrain(kind: Kind, from: [f32; 2], to: [f32; 2]) -> [f32; 2] {
    let d = [to[0] - from[0], to[1] - from[1]];
    if kind == Kind::Line {
        let a = (d[1].atan2(d[0]) / (TAU / 8.0)).round() * (TAU / 8.0);
        let r = d[0].hypot(d[1]);
        [from[0] + r * a.cos(), from[1] + r * a.sin()]
    } else {
        let side = d[0].abs().max(d[1].abs());
        [
            from[0] + side * d[0].signum(),
            from[1] + side * d[1].signum(),
        ]
    }
}
/// Construct a bounded contour. Reversed drag directions produce the same bounds.
pub fn path(kind: Kind, from: [f32; 2], to: [f32; 2]) -> Path {
    if kind == Kind::Line {
        return Path {
            points: vec![from, to],
            closed: false,
        };
    }
    let left = from[0].min(to[0]);
    let top = from[1].min(to[1]);
    let w = (from[0] - to[0]).abs();
    let h = (from[1] - to[1]).abs();
    if w <= f32::EPSILON || h <= f32::EPSILON {
        return Path {
            points: vec![[left, top], [left + w, top + h]],
            closed: false,
        };
    }
    let unit: Vec<[f32; 2]> = match kind {
        Kind::Rectangle => vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        Kind::RightTriangle => vec![[0., 0.], [1., 1.], [0., 1.]],
        Kind::Triangle => vec![[0.5, 0.], [1., 1.], [0., 1.]],
        Kind::Diamond => vec![[0.5, 0.], [1., 0.5], [0.5, 1.], [0., 0.5]],
        Kind::RightArrow | Kind::DownArrow | Kind::LeftArrow | Kind::UpArrow => {
            let p = vec![
                [0., 0.3],
                [0.6, 0.3],
                [0.6, 0.],
                [1., 0.5],
                [0.6, 1.],
                [0.6, 0.7],
                [0., 0.7],
            ];
            p.into_iter()
                .map(|[x, y]| match kind {
                    Kind::DownArrow => [y, x],
                    Kind::UpArrow => [y, 1. - x],
                    Kind::LeftArrow => [1. - x, y],
                    _ => [x, y],
                })
                .collect()
        }
        Kind::Lightning => vec![
            [0.55, 0.],
            [0.05, 0.55],
            [0.4, 0.55],
            [0.25, 1.],
            [0.95, 0.35],
            [0.6, 0.35],
            [0.8, 0.],
        ],
        Kind::RectangularCallout => vec![
            [0., 0.],
            [1., 0.],
            [1., 0.75],
            [0.55, 0.75],
            [0.2, 1.],
            [0.3, 0.75],
            [0., 0.75],
        ],
        Kind::OvalCallout | Kind::CloudCallout => {
            let mut p = Vec::new();
            // Leave a gap in the lower-left silhouette for a single connected tail.
            for i in 0..=80 {
                let a = 2.35 + i as f32 / 80. * (TAU - 0.55);
                let wave = if kind == Kind::CloudCallout {
                    1. + 0.10 * (a * 9.).cos()
                } else {
                    1.
                };
                p.push([0.5 + 0.45 * wave * a.cos(), 0.4 + 0.36 * wave * a.sin()]);
            }
            p.push([0.12, 1.]);
            p
        }
        Kind::Heart => {
            let mut p = Vec::new();
            for (a, b, c, d) in [
                ([0.5, 0.2], [0.2, -0.15], [-0.15, 0.15], [0.1, 0.5]),
                ([0.1, 0.5], [0.2, 0.65], [0.35, 0.85], [0.5, 1.]),
                ([0.5, 1.], [0.65, 0.85], [0.8, 0.65], [0.9, 0.5]),
                ([0.9, 0.5], [1.15, 0.15], [0.8, -0.15], [0.5, 0.2]),
            ] {
                for i in 0..20 {
                    let t = i as f32 / 20.;
                    let u = 1. - t;
                    p.push([
                        u * u * u * a[0]
                            + 3. * u * u * t * b[0]
                            + 3. * u * t * t * c[0]
                            + t * t * t * d[0],
                        u * u * u * a[1]
                            + 3. * u * u * t * b[1]
                            + 3. * u * t * t * c[1]
                            + t * t * t * d[1],
                    ]);
                }
            }
            p
        }
        Kind::RoundedRectangle => {
            // Constant circular corner radius in image space, limited by both axes.
            let r = w.min(h) * 0.2;
            let mut p = Vec::new();
            for (cx, cy, start) in [
                (w - r, r, -FRAC_PI_2),
                (w - r, h - r, 0.),
                (r, h - r, FRAC_PI_2),
                (r, r, 2. * FRAC_PI_2),
            ] {
                for i in 0..=12 {
                    let a = start + i as f32 / 12. * FRAC_PI_2;
                    p.push([
                        (cx + r * a.cos()) / w.max(0.001),
                        (cy + r * a.sin()) / h.max(0.001),
                    ]);
                }
            }
            p
        }
        _ => {
            let count = match kind {
                Kind::Pentagon => 5,
                Kind::Hexagon => 6,
                Kind::Star => 10,
                Kind::FourPointStar => 8,
                Kind::SixPointStar => 12,
                _ => ((w + h) * 0.5).ceil().clamp(32., 256.) as usize,
            };
            (0..count)
                .map(|i| {
                    let a = i as f32 / count as f32 * TAU - FRAC_PI_2;
                    let r = if matches!(kind, Kind::Star | Kind::FourPointStar | Kind::SixPointStar)
                        && i % 2 == 1
                    {
                        if kind == Kind::FourPointStar {
                            0.14
                        } else {
                            0.24
                        }
                    } else {
                        0.5
                    };
                    [0.5 + r * a.cos(), 0.5 + r * a.sin()]
                })
                .collect()
        }
    };
    Path {
        points: unit
            .into_iter()
            .map(|[x, y]| [left + x * w, top + y * h])
            .collect(),
        closed: true,
    }
}
fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
/// Ear clipping supports concave stars/arrows in the GUI preview without a GUI dependency.
pub fn triangles(path: &Path) -> Vec<[usize; 3]> {
    if !path.closed || path.points.len() < 3 {
        return vec![];
    }
    let p = &path.points;
    let mut ids: Vec<_> = (0..p.len()).collect();
    let area: f32 = (0..p.len())
        .map(|i| {
            let a = p[i];
            let b = p[(i + 1) % p.len()];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    let sign = area.signum();
    let mut result = Vec::new();
    while ids.len() > 2 {
        let mut found = false;
        for i in 0..ids.len() {
            let a = ids[(i + ids.len() - 1) % ids.len()];
            let b = ids[i];
            let c = ids[(i + 1) % ids.len()];
            if cross(p[a], p[b], p[c]) * sign <= 0.00001 {
                continue;
            }
            let contains = ids
                .iter()
                .copied()
                .filter(|j| *j != a && *j != b && *j != c)
                .any(|j| {
                    cross(p[a], p[b], p[j]) * sign >= 0.
                        && cross(p[b], p[c], p[j]) * sign >= 0.
                        && cross(p[c], p[a], p[j]) * sign >= 0.
                });
            if !contains {
                result.push([a, b, c]);
                ids.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    result
}
/// Draw into the active source, respecting lock, layer translation and selection.
/// Masks and live processing remain unchanged. Call on a worker for large shapes.
pub fn paint(
    doc: &mut Document,
    kind: Kind,
    from: [f32; 2],
    to: [f32; 2],
    style: Style,
) -> Result<bool, String> {
    paint_path(doc, &path(kind, from, to), style)
}
/// Paint an explicit polygon or flattened cubic path using the same alpha and
/// selection rules as drag-to-size shapes. Inputs are bounded before allocation.
pub fn paint_path(doc: &mut Document, path: &Path, style: Style) -> Result<bool, String> {
    if path.points.len() < 2
        || path.points.len() > 4096
        || path
            .points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
        || !style.width.is_finite()
        || !(1.0..=1024.).contains(&style.width)
        || !style.opacity.is_finite()
        || !(0.0..=1.).contains(&style.opacity)
    {
        return Err("Invalid shape parameters".into());
    }
    if doc.active().locked
        || style.opacity == 0.
        || (style.outline.is_none() && (style.fill.is_none() || !path.closed))
    {
        return Ok(false);
    }
    let mut contour = path.clone();
    editing::ensure_canvas_extent(doc)?;
    let offset = doc.active().offset;
    for p in &mut contour.points {
        p[0] -= offset[0] as f32;
        p[1] -= offset[1] as f32;
    }
    let selection = doc.selection.clone();
    let selected = |x: u32, y: u32| {
        editing::coverage(
            selection.as_ref(),
            i64::from(x) + i64::from(offset[0]),
            i64::from(y) + i64::from(offset[1]),
            doc.width,
            doc.height,
        )
    };
    let mut raster = doc.active().source.clone();
    let before = raster.clone();
    if let Some(color) = style.fill.filter(|_| contour.closed) {
        fill(
            &mut raster,
            &contour,
            |x, y| selected(x, y) * style.opacity,
            color,
        );
    }
    if let Some(color) = style.outline {
        let brush = Brush {
            diameter: style.width,
            opacity: style.opacity,
            hardness: 1.,
            color,
            erase: false,
        };
        let mut stroke = PaintStroke::new(&raster);
        let count = if contour.closed {
            contour.points.len()
        } else {
            contour.points.len() - 1
        };
        for i in 0..count {
            stroke.segment(
                brush,
                &mut raster,
                contour.points[i],
                contour.points[(i + 1) % contour.points.len()],
                selection.as_ref().map(|m| (m, offset)),
            );
        }
    }
    let changed = raster != before;
    doc.active_mut().source = raster;
    Ok(changed)
}
/// Four vertical samples plus analytic horizontal span coverage. Even/odd scanline
/// intersections handle concave paths, with no repeated alpha buildup at vertices.
fn fill(raster: &mut Raster, path: &Path, coverage: impl Fn(u32, u32) -> f32, color: [u8; 4]) {
    fill_coverage(raster.width(), raster.height(), path, |x, y, edge| {
        let alpha = edge * coverage(x, y);
        if alpha > 0. {
            raster.set(
                x,
                y,
                composite::over(raster.get(x, y), color, alpha, BlendMode::Normal),
            );
        }
    });
}

/// Shared antialiased even/odd spans for raster shapes and freeform selection.
/// Callers validate finite coordinates, bounded dimensions and point count.
pub(crate) fn fill_coverage(
    width: u32,
    height: u32,
    path: &Path,
    mut visit: impl FnMut(u32, u32, f32),
) {
    let min_y = path
        .points
        .iter()
        .map(|p| p[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.) as u32;
    let max_y = (path
        .points
        .iter()
        .map(|p| p[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .max(0.) as u32)
        .min(height);
    let mut row = vec![0.0_f32; width as usize];
    let mut intersections = Vec::with_capacity(path.points.len());
    for y in min_y..max_y {
        row.fill(0.);
        for sample in 0..4 {
            intersections.clear();
            let scan = y as f32 + (sample as f32 + 0.5) / 4.;
            for i in 0..path.points.len() {
                let a = path.points[i];
                let b = path.points[(i + 1) % path.points.len()];
                if (a[1] <= scan && b[1] > scan) || (b[1] <= scan && a[1] > scan) {
                    intersections.push(a[0] + (scan - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
                }
            }
            intersections.sort_by(f32::total_cmp);
            for pair in intersections.as_chunks::<2>().0 {
                let left = pair[0].max(0.);
                let right = pair[1].min(width as f32);
                for x in left.floor() as u32..right.ceil().max(0.) as u32 {
                    row[x as usize] +=
                        ((x as f32 + 1.).min(right) - (x as f32).max(left)).max(0.) * 0.25;
                }
            }
        }
        for (x, edge) in row
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, edge)| *edge > 0.)
        {
            let x = x as u32;
            visit(x, y, edge);
        }
    }
}

/// Commit a raster shape and select its clipped stroke/fill bounds. An existing
/// coverage selection still constrains the drawing and the resulting selection.
pub fn paint_selected(doc: &mut Document, path: &Path, style: Style) -> Result<bool, String> {
    let old = doc.selection.clone();
    if !paint_path(doc, path, style)? {
        return Ok(false);
    }
    let padding = if style.outline.is_some() {
        style.width * 0.5 + 1.
    } else {
        0.
    };
    let left = path
        .points
        .iter()
        .map(|p| p[0])
        .fold(f32::INFINITY, f32::min)
        - padding;
    let top = path
        .points
        .iter()
        .map(|p| p[1])
        .fold(f32::INFINITY, f32::min)
        - padding;
    let right = path
        .points
        .iter()
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max)
        + padding;
    let bottom = path
        .points
        .iter()
        .map(|p| p[1])
        .fold(f32::NEG_INFINITY, f32::max)
        + padding;
    let rect = editing::PixelRect::from_drag([left, top], [right, bottom], doc.width, doc.height);
    let mut mask = editing::rectangle(
        doc.width,
        doc.height,
        rect.unwrap_or(editing::PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }),
    );
    if let Some(old) = old {
        for (i, v) in std::sync::Arc::make_mut(&mut mask.values)
            .iter_mut()
            .enumerate()
        {
            if *v != 0 {
                *v = (old.at(i as u32 % doc.width, i as u32 / doc.width) * 255.).round() as u8;
            }
        }
    }
    doc.selection = Some(mask);
    doc.selection_base = None;
    Ok(true)
}
/// Flatten a cubic into bounded image-space segments for both preview and commit.
pub fn cubic(start: [f32; 2], end: [f32; 2], controls: [[f32; 2]; 2]) -> Path {
    let length = (end[0] - start[0]).hypot(end[1] - start[1])
        + (controls[0][0] - start[0]).hypot(controls[0][1] - start[1])
        + (controls[1][0] - end[0]).hypot(controls[1][1] - end[1]);
    let steps = (length * 0.5).ceil().clamp(24., 512.) as usize;
    let points = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let u = 1. - t;
            std::array::from_fn(|a| {
                u * u * u * start[a]
                    + 3. * u * u * t * controls[0][a]
                    + 3. * u * t * t * controls[1][a]
                    + t * t * t * end[a]
            })
        })
        .collect();
    Path {
        points,
        closed: false,
    }
}
