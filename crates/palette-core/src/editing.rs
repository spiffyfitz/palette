//! Coverage-aware edits in canvas coordinates. No window-system or clipboard API.
use crate::{
    composite::over,
    document::{BlendMode, Document},
    raster::{Coverage, Raster},
};
use std::sync::Arc;

/// A half-open integer rectangle in canvas space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    /// Clamp a pointer-drag rectangle to canvas bounds, independent of drag direction.
    pub fn from_drag(a: [f32; 2], b: [f32; 2], width: u32, height: u32) -> Option<Self> {
        let x = a[0].min(b[0]).floor().clamp(0.0, width as f32) as u32;
        let y = a[1].min(b[1]).floor().clamp(0.0, height as f32) as u32;
        let right = a[0].max(b[0]).ceil().clamp(0.0, width as f32) as u32;
        let bottom = a[1].max(b[1]).ceil().clamp(0.0, height as f32) as u32;
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
    /// Test membership without overflowing rectangle endpoints.
    pub fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x && y >= self.y && x - self.x < self.width && y - self.y < self.height
    }
}

/// Create a grayscale rectangular selection; `None` represents no selection constraint.
pub fn rectangle(width: u32, height: u32, rect: PixelRect) -> Coverage {
    let mut values = vec![0; (width * height) as usize];
    for y in rect.y.min(height)..rect.y.saturating_add(rect.height).min(height) {
        let left = (y * width + rect.x.min(width)) as usize;
        let right = (y * width + rect.x.saturating_add(rect.width).min(width)) as usize;
        values[left..right].fill(255);
    }
    Coverage {
        width,
        height,
        values: Arc::new(values),
        enabled: true,
    }
}

/// Bounding box of nonzero coverage. An empty mask selects nothing.
pub fn bounds(mask: &Coverage) -> Option<PixelRect> {
    if !mask.enabled {
        return Some(PixelRect {
            x: 0,
            y: 0,
            width: mask.width,
            height: mask.height,
        });
    }
    let mut left = mask.width;
    let mut top = mask.height;
    let mut right = 0;
    let mut bottom = 0;
    for (i, value) in mask.values.iter().enumerate() {
        if *value != 0 {
            let x = i as u32 % mask.width;
            let y = i as u32 / mask.width;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    (right > left && bottom > top).then(|| PixelRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

/// Directed visual boundaries at 50% coverage, with selected pixels to the right.
/// Direction changes split runs so diagonal islands remain separate contours.
pub fn outline(mask: &Coverage) -> Vec<[[f32; 2]; 2]> {
    outline_at(mask, 128)
}
fn outline_at(mask: &Coverage, threshold: u8) -> Vec<[[f32; 2]; 2]> {
    let Some(region) = bounds(mask) else {
        return vec![];
    };
    let inside = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < i64::from(mask.width)
            && y < i64::from(mask.height)
            && mask.at(x as u32, y as u32) * 255. >= f32::from(threshold)
    };
    let mut edges = Vec::new();
    for vertical in [false, true] {
        // Empty space cannot contain a boundary. Scanning the nonzero bounds
        // preserves the same directed run order without visiting the full canvas.
        let (outer_start, outer, inner_start, inner) = if vertical {
            (
                region.x,
                region.x + region.width,
                region.y,
                region.y + region.height,
            )
        } else {
            (
                region.y,
                region.y + region.height,
                region.x,
                region.x + region.width,
            )
        };
        for fixed in outer_start..=outer {
            let mut run: Option<(u32, i32)> = None;
            for along in inner_start..=inner {
                let direction = if along == inner {
                    0
                } else {
                    let (x, y) = if vertical {
                        (i64::from(fixed), i64::from(along))
                    } else {
                        (i64::from(along), i64::from(fixed))
                    };
                    if vertical {
                        i32::from(inside(x - 1, y)) - i32::from(inside(x, y))
                    } else {
                        i32::from(inside(x, y)) - i32::from(inside(x, y - 1))
                    }
                };
                if run.is_some_and(|(_, d)| d != direction) {
                    let (first, d) = run.take().expect("boundary run");
                    let at = |a: u32| {
                        if vertical {
                            [fixed as f32, a as f32]
                        } else {
                            [a as f32, fixed as f32]
                        }
                    };
                    edges.push(if d > 0 {
                        [at(first), at(along)]
                    } else {
                        [at(along), at(first)]
                    });
                }
                if direction != 0 && run.is_none() {
                    run = Some((along, direction));
                }
            }
        }
    }
    edges
}
/// Ordered closed loops for animated outlines. Right turns at diagonal contacts
/// keep touching islands separate. Geometry depends only on mask data, not time.
pub fn contours(mask: &Coverage) -> Vec<Vec<[f32; 2]>> {
    contours_at(mask, 128)
}
/// Trace a coverage isocontour. Threshold 1 shows the actual feather support;
/// threshold 128 retains the ordinary half-coverage marching selection boundary.
pub fn contours_at(mask: &Coverage, threshold: u8) -> Vec<Vec<[f32; 2]>> {
    use std::collections::HashMap;
    let edges = outline_at(mask, threshold.max(1));
    let key = |p: [f32; 2]| (p[0] as u32, p[1] as u32);
    let mut starts: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, edge) in edges.iter().enumerate() {
        starts.entry(key(edge[0])).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut loops = Vec::new();
    for first in 0..edges.len() {
        if used[first] {
            continue;
        }
        let mut points = vec![edges[first][0]];
        let mut current = first;
        loop {
            used[current] = true;
            let [a, b] = edges[current];
            points.push(b);
            if b == points[0] {
                break;
            }
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let next = starts
                .get(&key(b))
                .into_iter()
                .flatten()
                .copied()
                .filter(|i| !used[*i])
                .max_by_key(|i| {
                    let c = edges[*i][1];
                    let cross = dx * (c[1] - b[1]) - dy * (c[0] - b[0]);
                    if cross > 0. {
                        3
                    } else if cross == 0. {
                        2
                    } else {
                        1
                    }
                });
            let Some(next) = next else {
                break;
            };
            current = next;
        }
        loops.push(points);
    }
    loops
}

/// Selection coverage at signed canvas coordinates; off-canvas pixels are excluded.
pub fn coverage(mask: Option<&Coverage>, x: i64, y: i64, width: u32, height: u32) -> f32 {
    if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
        return 0.0;
    }
    mask.map_or(1.0, |m| m.at(x as u32, y as u32))
}

/// Interpolate pixel states in premultiplied space. Straight RGB interpolation
/// would introduce halos when clearing or moving partially covered transparent pixels.
pub fn mix(before: [u8; 4], after: [u8; 4], amount: f32) -> [u8; 4] {
    let amount = amount.clamp(0.0, 1.0);
    if amount == 0.0 {
        return before;
    }
    if amount == 1.0 {
        return after;
    }
    let a0 = f32::from(before[3]) * (1.0 - amount);
    let a1 = f32::from(after[3]) * amount;
    let alpha = a0 + a1;
    if alpha == 0.0 {
        return [0; 4];
    }
    let mut pixel = [0; 4];
    for c in 0..3 {
        pixel[c] = ((f32::from(before[c]) * a0 + f32::from(after[c]) * a1) / alpha).round() as u8;
    }
    pixel[3] = alpha.round() as u8;
    pixel
}

/// Delete selected source coverage, preserving layer properties and other layers.
pub fn clear(document: &mut Document) -> bool {
    if document.active().locked {
        return false;
    }
    let selection = document.selection.clone();
    let (width, height) = (document.width, document.height);
    let layer = document.active_mut();
    let mut changed = false;
    for y in 0..layer.source.height() {
        for x in 0..layer.source.width() {
            let amount = coverage(
                selection.as_ref(),
                i64::from(x) + i64::from(layer.offset[0]),
                i64::from(y) + i64::from(layer.offset[1]),
                width,
                height,
            );
            let before = layer.source.get(x, y);
            if amount > 0.0 && before[3] > 0 {
                let mut after = before;
                after[3] = (f32::from(before[3]) * (1.0 - amount)).round() as u8;
                layer.source.set(x, y, after);
                changed = true;
            }
        }
    }
    changed
}

/// Copy the active layer's evaluated appearance, including its selection coverage.
/// Filters/masks stay live on the source; the system clipboard receives flat RGBA.
pub fn copy(document: &Document) -> Result<Raster, String> {
    let rect = match &document.selection {
        Some(mask) => bounds(mask).ok_or("The selection is empty")?,
        None => PixelRect {
            x: 0,
            y: 0,
            width: document.width,
            height: document.height,
        },
    };
    let mut output = Raster::new(rect.width, rect.height, [0; 4])?;
    let layer = document.active();
    let prepared = crate::processing::prepare_layer(layer)?;
    for y in 0..rect.height {
        for x in 0..rect.width {
            let cx = x + rect.x;
            let cy = y + rect.y;
            let lx = i64::from(cx) - i64::from(layer.offset[0]);
            let ly = i64::from(cy) - i64::from(layer.offset[1]);
            let mut pixel = prepared.pixel(lx, ly);
            pixel[3] = (f32::from(pixel[3])
                * layer.opacity
                * coverage(
                    document.selection.as_ref(),
                    i64::from(cx),
                    i64::from(cy),
                    document.width,
                    document.height,
                ))
            .round() as u8;
            output.set(x, y, pixel);
        }
    }
    Ok(output)
}

/// Paste into a new editable layer anchored to the selection or canvas origin.
pub fn paste(document: &mut Document, pixels: Raster) -> Result<(), String> {
    if document.layers.len() >= 256 {
        return Err("The document already has 256 layers".into());
    }
    let anchor = document
        .selection
        .as_ref()
        .and_then(bounds)
        .map_or([0, 0], |r| [r.x, r.y]);
    let width = document.width.max(
        anchor[0]
            .checked_add(pixels.width())
            .ok_or("Pasted image bounds overflow")?,
    );
    let height = document.height.max(
        anchor[1]
            .checked_add(pixels.height())
            .ok_or("Pasted image bounds overflow")?,
    );
    // Paint-style paste expands the canvas rather than silently hiding part of
    // the clipboard image. Validation precedes any document mutation.
    crate::transform::canvas_resize(document, width, height)?;
    let rect = PixelRect {
        x: anchor[0],
        y: anchor[1],
        width: pixels.width(),
        height: pixels.height(),
    };
    document.add_layer();
    let layer = document.active_mut();
    layer.name = "Pasted image".into();
    layer.offset = [anchor[0] as i32, anchor[1] as i32];
    layer.source = pixels;
    document.selection = Some(rectangle(document.width, document.height, rect));
    document.selection_base = None;
    Ok(())
}

/// Move selected source pixels without flattening their layer's filters/mask.
pub fn move_selection(document: &mut Document, delta: [i32; 2]) -> Result<bool, String> {
    crate::transform::SelectionTransform::Move(delta).apply(document)
}

/// Grow the active layer to cover the canvas while retaining all hidden source
/// pixels and reanchoring local masks. Returns an error before oversized allocation.
pub fn ensure_canvas_extent(document: &mut Document) -> Result<(), String> {
    let layer = document.active();
    let left = i64::from(layer.offset[0]).min(0);
    let top = i64::from(layer.offset[1]).min(0);
    let right = (i64::from(layer.offset[0]) + i64::from(layer.source.width()))
        .max(i64::from(document.width));
    let bottom = (i64::from(layer.offset[1]) + i64::from(layer.source.height()))
        .max(i64::from(document.height));
    if left == i64::from(layer.offset[0])
        && top == i64::from(layer.offset[1])
        && right - left == i64::from(layer.source.width())
        && bottom - top == i64::from(layer.source.height())
    {
        return Ok(());
    }
    let width = u32::try_from(right - left).map_err(|_| "Layer bounds overflow")?;
    let height = u32::try_from(bottom - top).map_err(|_| "Layer bounds overflow")?;
    let mut expanded = Raster::new(width, height, [0; 4])?;
    let shift = [
        (i64::from(layer.offset[0]) - left) as u32,
        (i64::from(layer.offset[1]) - top) as u32,
    ];
    for y in 0..layer.source.height() {
        for x in 0..layer.source.width() {
            expanded.set(x + shift[0], y + shift[1], layer.source.get(x, y));
        }
    }
    let mask = |mask: &Coverage| {
        let mut values = vec![0; (width * height) as usize];
        for y in 0..mask.height.min(height - shift[1]) {
            for x in 0..mask.width.min(width - shift[0]) {
                values[((y + shift[1]) * width + x + shift[0]) as usize] =
                    mask.values[(y * mask.width + x) as usize];
            }
        }
        Coverage {
            width,
            height,
            values: Arc::new(values),
            enabled: mask.enabled,
        }
    };
    let layer = document.active_mut();
    layer.mask = layer.mask.as_ref().map(mask);
    for filter in &mut layer.filters {
        filter.mask = filter.mask.as_ref().map(mask);
    }
    layer.offset = [left as i32, top as i32];
    layer.source = expanded;
    Ok(())
}

/// Contiguous scanline flood fill. Distance compares premultiplied RGBA, so RGB
/// hidden behind zero alpha never creates false boundaries. Tolerance is 0..=255.
pub fn flood_fill(
    document: &mut Document,
    point: [u32; 2],
    color: [u8; 4],
    opacity: f32,
    tolerance: u8,
) -> Result<bool, String> {
    if document.active().locked || point[0] >= document.width || point[1] >= document.height {
        return Ok(false);
    }
    if coverage(
        document.selection.as_ref(),
        i64::from(point[0]),
        i64::from(point[1]),
        document.width,
        document.height,
    ) == 0.0
    {
        return Ok(false);
    }
    ensure_canvas_extent(document)?;
    let layer = document.active();
    let offset = layer.offset;
    let source = layer.source.clone();
    let selection = document.selection.clone();
    let (width, height) = (document.width, document.height);
    let sample = |x: u32, y: u32| {
        source.get(
            (i64::from(x) - i64::from(offset[0])) as u32,
            (i64::from(y) - i64::from(offset[1])) as u32,
        )
    };
    let target = sample(point[0], point[1]);
    let premul = |p: [u8; 4]| {
        [
            u16::from(p[0]) * u16::from(p[3]) / 255,
            u16::from(p[1]) * u16::from(p[3]) / 255,
            u16::from(p[2]) * u16::from(p[3]) / 255,
            u16::from(p[3]),
        ]
    };
    let target = premul(target);
    let matches = |x: u32, y: u32| {
        coverage(
            selection.as_ref(),
            i64::from(x),
            i64::from(y),
            width,
            height,
        ) > 0.0
            && premul(sample(x, y))
                .iter()
                .zip(target)
                .all(|(a, b)| a.abs_diff(b) <= u16::from(tolerance))
    };
    let mut seen = vec![false; (width * height) as usize];
    let mut stack = vec![(point[0], point[1])];
    let layer = document.active_mut();
    let mut changed = false;
    while let Some((sx, y)) = stack.pop() {
        if seen[(y * width + sx) as usize] || !matches(sx, y) {
            continue;
        }
        let mut left = sx;
        while left > 0 && !seen[(y * width + left - 1) as usize] && matches(left - 1, y) {
            left -= 1;
        }
        let mut x = left;
        let mut above = false;
        let mut below = false;
        while x < width && !seen[(y * width + x) as usize] && matches(x, y) {
            seen[(y * width + x) as usize] = true;
            let lx = (i64::from(x) - i64::from(offset[0])) as u32;
            let ly = (i64::from(y) - i64::from(offset[1])) as u32;
            let before = sample(x, y);
            let after = over(
                before,
                color,
                opacity
                    * coverage(
                        selection.as_ref(),
                        i64::from(x),
                        i64::from(y),
                        width,
                        height,
                    ),
                BlendMode::Normal,
            );
            if after != before {
                layer.source.set(lx, ly, after);
                changed = true;
            }
            for (neighbor, run) in [
                (y.checked_sub(1), &mut above),
                ((y + 1 < height).then_some(y + 1), &mut below),
            ] {
                let eligible =
                    neighbor.is_some_and(|ny| !seen[(ny * width + x) as usize] && matches(x, ny));
                if eligible && !*run {
                    stack.push((x, neighbor.expect("eligible neighbor")));
                }
                *run = eligible;
            }
            x += 1;
        }
    }
    Ok(changed)
}

/// Translate the complete active layer without resampling or rewriting source,
/// local masks, filters or effects. Canvas selections remain fixed in canvas space.
/// Checked arithmetic makes an overflowing move atomic.
pub fn translate_layer(document: &mut Document, delta: [i32; 2]) -> Result<bool, String> {
    if document.active().locked || delta == [0, 0] {
        return Ok(false);
    }
    let offset = document.active().offset;
    let x = offset[0]
        .checked_add(delta[0])
        .ok_or("Layer position exceeds the supported range")?;
    let y = offset[1]
        .checked_add(delta[1])
        .ok_or("Layer position exceeds the supported range")?;
    document.active_mut().offset = [x, y];
    Ok(true)
}
