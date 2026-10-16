//! Selection-local source transforms and arbitrary-angle document rotation.
use super::*;
use crate::editing::{self, PixelRect};

/// A selection gesture is always evaluated against its original document, never
/// against the preceding preview. This prevents cumulative resampling damage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionTransform {
    /// Integer translation in canvas pixels.
    Move([i32; 2]),
    /// Resize into canvas-space bounds, retaining the canvas dimensions.
    Resize { origin: [i32; 2], size: [u32; 2] },
    /// Clockwise degrees around the original selection center.
    Rotate(f64),
}

impl SelectionTransform {
    /// Transform active-layer source pixels and selection coverage together.
    /// Local masks and live processing stay on the layer; other layers are intact.
    /// Validation/allocation failure leaves the input document unchanged.
    pub fn apply(self, doc: &mut Document) -> Result<bool, String> {
        if doc.active().locked {
            return Ok(false);
        }
        let Some(rect) = doc.selection.as_ref().and_then(bounds) else {
            return Ok(false);
        };
        if let Self::Rotate(degrees) = self {
            return rotate(doc, degrees);
        }
        let (anchor, size) = match self {
            Self::Move([0, 0]) => return Ok(false),
            Self::Move(delta) => (
                [
                    i64::from(rect.x) + i64::from(delta[0]),
                    i64::from(rect.y) + i64::from(delta[1]),
                ],
                [rect.width, rect.height],
            ),
            Self::Resize { origin, size } => (origin.map(i64::from), size),
            Self::Rotate(_) => unreachable!(),
        };
        if anchor == [i64::from(rect.x), i64::from(rect.y)] && size == [rect.width, rect.height] {
            return Ok(false);
        }
        let mut bytes = 0;
        reserve_plane(&mut bytes, rect.width, rect.height, 8)?;
        reserve_plane(&mut bytes, size[0], size[1], 8)?;
        let (pixels, mask) = extracted(doc, rect)?;
        let (pixels, mask) = if size == [rect.width, rect.height] {
            (pixels, mask)
        } else {
            (
                resample(&pixels, size[0], size[1], Resample::Bilinear)?,
                resample(&mask, size[0], size[1], Resample::Bilinear)?,
            )
        };
        replace_selection(doc, pixels, mask, anchor)
    }
}

fn transparent(source: &Raster, x: i64, y: i64) -> [u8; 4] {
    if x < 0 || y < 0 || x >= i64::from(source.width()) || y >= i64::from(source.height()) {
        [0; 4]
    } else {
        source.get(x as u32, y as u32)
    }
}
/// Bilinear interpolation of premultiplied samples, with transparent outside
/// pixels. Coordinates refer to pixel centers; hidden RGB never bleeds into edges.
fn sample(source: &Raster, x: f64, y: f64) -> [u8; 4] {
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let tx = (x - x.floor()) as f32;
    let ty = (y - y.floor()) as f32;
    let samples = [
        (transparent(source, x0, y0), (1. - tx) * (1. - ty)),
        (transparent(source, x0 + 1, y0), tx * (1. - ty)),
        (transparent(source, x0, y0 + 1), (1. - tx) * ty),
        (transparent(source, x0 + 1, y0 + 1), tx * ty),
    ];
    let alpha: f32 = samples.iter().map(|(p, w)| f32::from(p[3]) * w).sum();
    let mut out = [0; 4];
    if alpha > 0. {
        for c in 0..3 {
            out[c] = (samples
                .iter()
                .map(|(p, w)| f32::from(p[c]) * f32::from(p[3]) * w)
                .sum::<f32>()
                / alpha)
                .round()
                .clamp(0., 255.) as u8;
        }
    }
    out[3] = alpha.round().clamp(0., 255.) as u8;
    out
}
fn extracted(doc: &Document, rect: PixelRect) -> Result<(Raster, Raster), String> {
    let mut pixels = Raster::new(rect.width, rect.height, [0; 4])?;
    // Coverage is a separate plane, including selected transparent pixels.
    let mut mask = Raster::new(rect.width, rect.height, [0; 4])?;
    let layer = doc.active();
    for y in 0..rect.height {
        for x in 0..rect.width {
            let cx = x + rect.x;
            let cy = y + rect.y;
            let coverage = editing::coverage(
                doc.selection.as_ref(),
                i64::from(cx),
                i64::from(cy),
                doc.width,
                doc.height,
            );
            let mut p = transparent(
                &layer.source,
                i64::from(cx) - i64::from(layer.offset[0]),
                i64::from(cy) - i64::from(layer.offset[1]),
            );
            p[3] = (f32::from(p[3]) * coverage).round() as u8;
            pixels.set(x, y, p);
            mask.set(x, y, [255, 255, 255, (coverage * 255.).round() as u8]);
        }
    }
    Ok((pixels, mask))
}
fn permuted(source: &Raster, op: Orthogonal) -> Result<Raster, String> {
    let size = dimensions(op, source.width(), source.height());
    let mut result = Raster::new(size[0], size[1], [0; 4])?;
    for y in 0..source.height() {
        for x in 0..source.width() {
            let [dx, dy] = mapped(op, x, y, source.width(), source.height());
            result.set(dx, dy, source.get(x, y));
        }
    }
    Ok(result)
}
// Split coverage into old-only, new-only, overlap and neither regions. The
// selected contribution replaces old coverage in the overlap, and uses source-
// over only over previously unselected coverage. Simply clearing then source-
// over-stamping would reduce alpha twice at feathered edges, even for a flip.
fn rejoin(before: [u8; 4], piece: [u8; 4], old: f32, new: f32) -> [u8; 4] {
    let source_alpha = if new > 0. {
        (f32::from(piece[3]) / 255. / new).clamp(0., 1.)
    } else {
        0.
    };
    let base = (1. - old - (new - old).max(0.) * source_alpha).clamp(0., 1.);
    let a = f32::from(before[3]) * base;
    let b = f32::from(piece[3]);
    let alpha = a + b;
    let mut out = before;
    out[3] = alpha.round().clamp(0., 255.) as u8;
    if alpha > 0. {
        for c in 0..3 {
            out[c] = ((f32::from(before[c]) * a + f32::from(piece[c]) * b) / alpha)
                .round()
                .clamp(0., 255.) as u8;
        }
    }
    out
}
fn replace_selection(
    doc: &mut Document,
    pixels: Raster,
    mask: Raster,
    anchor: [i64; 2],
) -> Result<bool, String> {
    let mut out = doc.clone();
    editing::ensure_canvas_extent(&mut out)?;
    editing::clear(&mut out);
    let (width, height) = (out.width, out.height);
    let mut selected = editing::rectangle(
        width,
        height,
        PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
    );
    let bytes = Arc::make_mut(&mut selected.values);
    let layer = out.active_mut();
    for y in 0..pixels.height() {
        for x in 0..pixels.width() {
            let dx = anchor[0] + i64::from(x);
            let dy = anchor[1] + i64::from(y);
            if dx < 0 || dy < 0 || dx >= i64::from(width) || dy >= i64::from(height) {
                continue;
            }
            bytes[(dy as u32 * width + dx as u32) as usize] = mask.get(x, y)[3];
            let tx = (dx - i64::from(layer.offset[0])) as u32;
            let ty = (dy - i64::from(layer.offset[1])) as u32;
            let pixel = pixels.get(x, y);
            let new = f32::from(mask.get(x, y)[3]) / 255.;
            if new > 0. {
                let before = transparent(
                    &doc.active().source,
                    dx - i64::from(doc.active().offset[0]),
                    dy - i64::from(doc.active().offset[1]),
                );
                let old = editing::coverage(doc.selection.as_ref(), dx, dy, width, height);
                layer.source.set(tx, ty, rejoin(before, pixel, old, new));
            }
        }
    }
    out.selection = Some(selected);
    let changed = out != *doc;
    *doc = out;
    Ok(changed)
}
/// Flip/rotate only selected source pixels on the active layer. The canvas,
/// other layers and layer processing properties remain unchanged. Rotated bounds
/// retain a stable integer-grid center, avoiding half-pixel drift across turns.
pub fn selected_orthogonal(doc: &mut Document, op: Orthogonal) -> Result<bool, String> {
    if doc.active().locked {
        return Ok(false);
    }
    let Some(rect) = doc.selection.as_ref().and_then(bounds) else {
        return Ok(false);
    };
    let mut staging = 0;
    reserve_plane(&mut staging, rect.width, rect.height, 16)?;
    let (pixels, mask) = extracted(doc, rect)?;
    let pixels = permuted(&pixels, op)?;
    let mask = permuted(&mask, op)?;
    let anchor = [
        i64::from(rect.x) + i64::from(rect.width / 2) - i64::from(pixels.width() / 2),
        i64::from(rect.y) + i64::from(rect.height / 2) - i64::from(pixels.height() / 2),
    ];
    replace_selection(doc, pixels, mask, anchor)
}
fn rotation(degrees: f64) -> Result<(f64, f64), String> {
    if !degrees.is_finite() || degrees.abs() > 360000. {
        return Err("Enter a finite rotation angle".into());
    }
    let a = degrees.rem_euclid(360.).to_radians();
    Ok((a.cos(), a.sin()))
}
fn rotated_bounds(
    size: [u32; 2],
    offset: [i32; 2],
    center: [f64; 2],
    output_center: [f64; 2],
    c: f64,
    s: f64,
) -> Result<([i32; 2], [u32; 2]), String> {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for x in [
        f64::from(offset[0]),
        f64::from(offset[0]) + f64::from(size[0]),
    ] {
        for y in [
            f64::from(offset[1]),
            f64::from(offset[1]) + f64::from(size[1]),
        ] {
            let p = [
                output_center[0] + c * (x - center[0]) - s * (y - center[1]),
                output_center[1] + s * (x - center[0]) + c * (y - center[1]),
            ];
            for a in 0..2 {
                min[a] = min[a].min(p[a]);
                max[a] = max[a].max(p[a]);
            }
        }
    }
    let left = min[0].floor();
    let top = min[1].floor();
    let width = max[0].ceil() - left;
    let height = max[1].ceil() - top;
    if left < f64::from(i32::MIN)
        || top < f64::from(i32::MIN)
        || left > f64::from(i32::MAX)
        || top > f64::from(i32::MAX)
        || width > f64::from(u32::MAX)
        || height > f64::from(u32::MAX)
    {
        return Err("Rotated bounds overflow".into());
    }
    Ok((
        [left as i32, top as i32],
        [width.max(1.) as u32, height.max(1.) as u32],
    ))
}
fn rotated_plane(
    source: &Raster,
    source_offset: [i32; 2],
    dest_offset: [i32; 2],
    size: [u32; 2],
    center: [f64; 2],
    output_center: [f64; 2],
    trig: (f64, f64),
) -> Result<Raster, String> {
    let (c, s) = trig;
    let mut out = Raster::new(size[0], size[1], [0; 4])?;
    for y in 0..size[1] {
        for x in 0..size[0] {
            let dx = f64::from(dest_offset[0]) + f64::from(x) + 0.5 - output_center[0];
            let dy = f64::from(dest_offset[1]) + f64::from(y) + 0.5 - output_center[1];
            out.set(
                x,
                y,
                sample(
                    source,
                    center[0] + c * dx + s * dy - f64::from(source_offset[0]) - 0.5,
                    center[1] - s * dx + c * dy - f64::from(source_offset[1]) - 0.5,
                ),
            );
        }
    }
    Ok(out)
}
fn mask_plane(mask: &Coverage) -> Result<Raster, String> {
    let mut plane = Raster::new(mask.width, mask.height, [0; 4])?;
    for y in 0..mask.height {
        for x in 0..mask.width {
            plane.set(
                x,
                y,
                [255, 255, 255, mask.values[(y * mask.width + x) as usize]],
            );
        }
    }
    Ok(plane)
}
fn coverage_from(plane: Raster, enabled: bool) -> Coverage {
    Coverage {
        width: plane.width(),
        height: plane.height(),
        enabled,
        values: Arc::new(
            plane
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[3])
                .collect(),
        ),
    }
}
/// Rotate clockwise by an arbitrary angle. A selection scopes the operation to
/// active-layer source pixels, with clipping at the existing canvas. Without a
/// selection, rotate all layers/masks and expand canvas bounds without flattening.
/// Quarter turns retain exact pixel permutations. General angles use alpha-safe
/// bilinear sampling; all allocations and offsets are validated before commit.
pub fn rotate(doc: &mut Document, degrees: f64) -> Result<bool, String> {
    let trig = rotation(degrees)?;
    let normalized = degrees.rem_euclid(360.);
    if normalized.abs() < 1e-9 {
        return Ok(false);
    }
    let quarter = (normalized / 90.).round();
    if (normalized - quarter * 90.).abs() < 1e-9 {
        let op = match quarter as i32 % 4 {
            1 => Orthogonal::Clockwise,
            2 => Orthogonal::HalfTurn,
            3 => Orthogonal::CounterClockwise,
            _ => return Ok(false),
        };
        if doc.selection.is_some() {
            return selected_orthogonal(doc, op);
        }
        orthogonal(doc, op)?;
        return Ok(true);
    }
    if doc.selection.is_some() {
        if doc.active().locked {
            return Ok(false);
        }
        let Some(rect) = doc.selection.as_ref().and_then(bounds) else {
            return Ok(false);
        };
        let center = [
            f64::from(rect.x) + f64::from(rect.width) * 0.5,
            f64::from(rect.y) + f64::from(rect.height) * 0.5,
        ];
        let (anchor, size) = rotated_bounds(
            [rect.width, rect.height],
            [rect.x as i32, rect.y as i32],
            center,
            center,
            trig.0,
            trig.1,
        )?;
        let mut bytes = 0;
        reserve_plane(&mut bytes, rect.width, rect.height, 8)?;
        reserve_plane(&mut bytes, size[0], size[1], 8)?;
        let (pixels, mask) = extracted(doc, rect)?;
        let pixels = rotated_plane(
            &pixels,
            [rect.x as i32, rect.y as i32],
            anchor,
            size,
            center,
            center,
            trig,
        )?;
        let mask = rotated_plane(
            &mask,
            [rect.x as i32, rect.y as i32],
            anchor,
            size,
            center,
            center,
            trig,
        )?;
        return replace_selection(doc, pixels, mask, anchor.map(i64::from));
    }
    let size = [
        (f64::from(doc.width) * trig.0.abs() + f64::from(doc.height) * trig.1.abs()).ceil() as u32,
        (f64::from(doc.width) * trig.1.abs() + f64::from(doc.height) * trig.0.abs()).ceil() as u32,
    ];
    Raster::new(size[0], size[1], [0; 4])?;
    let center = [f64::from(doc.width) * 0.5, f64::from(doc.height) * 0.5];
    let output_center = [f64::from(size[0]) * 0.5, f64::from(size[1]) * 0.5];
    let mut plans = Vec::new();
    let mut bytes = 0;
    for layer in &doc.layers {
        let plan = rotated_bounds(
            [layer.source.width(), layer.source.height()],
            layer.offset,
            center,
            output_center,
            trig.0,
            trig.1,
        )?;
        reserve_plane(&mut bytes, plan.1[0], plan.1[1], 4)?;
        for mask in layer
            .mask
            .iter()
            .chain(layer.filters.iter().filter_map(|f| f.mask.as_ref()))
        {
            reserve_plane(&mut bytes, plan.1[0], plan.1[1], 4)?;
            mask.validate()?;
        }
        plans.push(plan);
    }
    let mut out = doc.clone();
    for (layer, (offset, size)) in out.layers.iter_mut().zip(plans) {
        let old = layer.offset;
        layer.source = rotated_plane(
            &layer.source,
            old,
            offset,
            size,
            center,
            output_center,
            trig,
        )?;
        let map_mask = |m: &Coverage| -> Result<Coverage, String> {
            Ok(coverage_from(
                rotated_plane(
                    &mask_plane(m)?,
                    old,
                    offset,
                    size,
                    center,
                    output_center,
                    trig,
                )?,
                m.enabled,
            ))
        };
        layer.mask = layer.mask.as_ref().map(map_mask).transpose()?;
        for filter in &mut layer.filters {
            filter.mask = filter.mask.as_ref().map(map_mask).transpose()?;
        }
        layer.offset = offset;
    }
    out.width = size[0];
    out.height = size[1];
    out.validate()?;
    *doc = out;
    Ok(true)
}
