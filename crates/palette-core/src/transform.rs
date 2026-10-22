//! Document geometry operations preserve layers and their live processing stacks.
mod selection;
use crate::{
    document::Document,
    editing::bounds,
    raster::{Coverage, Raster},
};
pub use selection::{SelectionTransform, rotate, rotate_raster, selected_orthogonal};
use std::sync::Arc;

// Whole-image operations materialize every output plane. Validate aggregate
// output before starting so many small layers cannot amplify into unbounded RAM.
const MAX_TRANSFORM_BYTES: u64 = 512 * 1024 * 1024;
fn reserve_plane(bytes: &mut u64, width: u32, height: u32, channels: u64) -> Result<(), String> {
    Raster::new(width, height, [0; 4])?;
    *bytes += u64::from(width) * u64::from(height) * channels;
    if *bytes > MAX_TRANSFORM_BYTES {
        return Err("This transform would exceed the 512 MiB output-buffer limit. Reduce the image size or layer count.".into());
    }
    Ok(())
}

/// Exact pixel permutations; no resampling or alpha rounding.
#[derive(Clone, Copy, Debug)]
pub enum Orthogonal {
    Clockwise,
    CounterClockwise,
    HalfTurn,
    FlipHorizontal,
    FlipVertical,
}

fn mapped(operation: Orthogonal, x: u32, y: u32, width: u32, height: u32) -> [u32; 2] {
    match operation {
        Orthogonal::Clockwise => [height - 1 - y, x],
        Orthogonal::CounterClockwise => [y, width - 1 - x],
        Orthogonal::HalfTurn => [width - 1 - x, height - 1 - y],
        Orthogonal::FlipHorizontal => [width - 1 - x, y],
        Orthogonal::FlipVertical => [x, height - 1 - y],
    }
}
fn dimensions(operation: Orthogonal, width: u32, height: u32) -> [u32; 2] {
    if matches!(
        operation,
        Orthogonal::Clockwise | Orthogonal::CounterClockwise
    ) {
        [height, width]
    } else {
        [width, height]
    }
}
fn coverage_map(
    mask: &Coverage,
    width: u32,
    height: u32,
    size: [u32; 2],
    map: impl Fn(u32, u32) -> [u32; 2],
) -> Coverage {
    let mut values = vec![0; (size[0] * size[1]) as usize];
    for y in 0..height {
        for x in 0..width {
            let [dx, dy] = map(x, y);
            values[(dy * size[0] + dx) as usize] = if x < mask.width && y < mask.height {
                mask.values[(y * mask.width + x) as usize]
            } else {
                0
            };
        }
    }
    Coverage {
        width: size[0],
        height: size[1],
        values: Arc::new(values),
        enabled: mask.enabled,
    }
}

/// Crop canvas bounds to selected coverage. Source rasters outside the crop are
/// retained, reanchored by layer offsets, and remain recoverable through undo.
pub fn crop(document: &mut Document) -> Result<bool, String> {
    let rect = document
        .selection
        .as_ref()
        .and_then(bounds)
        .ok_or("Select an area before cropping")?;
    if rect.x == 0 && rect.y == 0 && rect.width == document.width && rect.height == document.height
    {
        return Ok(false);
    }
    let offsets: Result<Vec<_>, _> = document
        .layers
        .iter()
        .map(|l| {
            Ok::<_, String>([
                i32::try_from(i64::from(l.offset[0]) - i64::from(rect.x))
                    .map_err(|_| "Layer offset overflow")?,
                i32::try_from(i64::from(l.offset[1]) - i64::from(rect.y))
                    .map_err(|_| "Layer offset overflow")?,
            ])
        })
        .collect();
    for (layer, offset) in document.layers.iter_mut().zip(offsets?) {
        layer.offset = offset;
    }
    document.width = rect.width;
    document.height = rect.height;
    document.selection = None;
    document.selection_base = None;
    Ok(true)
}

/// Rotate/flip the whole document, including selection, positions and local masks.
/// Locked layers participate in whole-image transforms, as in canvas cropping.
pub fn orthogonal(document: &mut Document, operation: Orthogonal) -> Result<(), String> {
    let [width, height] = [document.width, document.height];
    let mut bytes = 0;
    for layer in &document.layers {
        reserve_plane(&mut bytes, layer.source.width(), layer.source.height(), 4)?;
        for _ in layer
            .mask
            .iter()
            .chain(layer.filters.iter().filter_map(|f| f.mask.as_ref()))
        {
            reserve_plane(&mut bytes, layer.source.width(), layer.source.height(), 1)?;
        }
    }
    if document.selection.is_some() {
        reserve_plane(&mut bytes, width, height, 1)?;
    }
    if document.selection_base.is_some() {
        reserve_plane(&mut bytes, width, height, 1)?;
    }
    // Work on a cheap COW clone: any size/offset error leaves the caller unchanged.
    let mut output = document.clone();
    for layer in &mut output.layers {
        let (w, h) = (layer.source.width(), layer.source.height());
        let size = dimensions(operation, w, h);
        let map = |x, y| mapped(operation, x, y, w, h);
        let mut source = Raster::new(size[0], size[1], [0; 4])?;
        for y in 0..h {
            for x in 0..w {
                let [dx, dy] = map(x, y);
                source.set(dx, dy, layer.source.get(x, y));
            }
        }
        layer.mask = layer
            .mask
            .as_ref()
            .map(|m| coverage_map(m, w, h, size, map));
        for filter in &mut layer.filters {
            filter.mask = filter
                .mask
                .as_ref()
                .map(|m| coverage_map(m, w, h, size, map));
        }
        let x = i64::from(layer.offset[0]);
        let y = i64::from(layer.offset[1]);
        let offset = match operation {
            Orthogonal::Clockwise => [i64::from(height) - y - i64::from(h), x],
            Orthogonal::CounterClockwise => [y, i64::from(width) - x - i64::from(w)],
            Orthogonal::HalfTurn => [
                i64::from(width) - x - i64::from(w),
                i64::from(height) - y - i64::from(h),
            ],
            Orthogonal::FlipHorizontal => [i64::from(width) - x - i64::from(w), y],
            Orthogonal::FlipVertical => [x, i64::from(height) - y - i64::from(h)],
        };
        layer.offset = [
            i32::try_from(offset[0]).map_err(|_| "Layer offset overflow")?,
            i32::try_from(offset[1]).map_err(|_| "Layer offset overflow")?,
        ];
        layer.source = source;
    }
    let size = dimensions(operation, width, height);
    output.selection = document.selection.as_ref().map(|m| {
        coverage_map(m, width, height, size, |x, y| {
            mapped(operation, x, y, width, height)
        })
    });
    output.selection_base = document.selection_base.as_ref().map(|m| {
        coverage_map(m, width, height, size, |x, y| {
            mapped(operation, x, y, width, height)
        })
    });
    output.width = size[0];
    output.height = size[1];
    *document = output;
    Ok(())
}

/// Image resampling algorithms exposed to users.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resample {
    Nearest,
    Bilinear,
}

fn axis(position: u32, source: u32, destination: u32) -> (u32, u32, f32) {
    let p = ((position as f64 + 0.5) * f64::from(source) / f64::from(destination) - 0.5)
        .clamp(0.0, f64::from(source - 1));
    (
        p.floor() as u32,
        (p.floor() as u32 + 1).min(source - 1),
        p.fract() as f32,
    )
}

/// Premultiplied bilinear interpolation prevents colored halos at transparent edges.
pub fn resample(
    source: &Raster,
    width: u32,
    height: u32,
    method: Resample,
) -> Result<Raster, String> {
    let mut output = Raster::new(width, height, [0; 4])?;
    for y in 0..height {
        for x in 0..width {
            let pixel = if method == Resample::Nearest {
                source.get(
                    (u64::from(x) * u64::from(source.width()) / u64::from(width)) as u32,
                    (u64::from(y) * u64::from(source.height()) / u64::from(height)) as u32,
                )
            } else {
                let (x0, x1, tx) = axis(x, source.width(), width);
                let (y0, y1, ty) = axis(y, source.height(), height);
                let samples = [
                    (source.get(x0, y0), (1.0 - tx) * (1.0 - ty)),
                    (source.get(x1, y0), tx * (1.0 - ty)),
                    (source.get(x0, y1), (1.0 - tx) * ty),
                    (source.get(x1, y1), tx * ty),
                ];
                let alpha: f32 = samples.iter().map(|(p, w)| f32::from(p[3]) * w).sum();
                let mut p = [0; 4];
                if alpha > 0.0 {
                    for c in 0..3 {
                        p[c] = (samples
                            .iter()
                            .map(|(p, w)| f32::from(p[c]) * f32::from(p[3]) * w)
                            .sum::<f32>()
                            / alpha)
                            .round()
                            .clamp(0.0, 255.0) as u8;
                    }
                }
                p[3] = alpha.round().clamp(0.0, 255.0) as u8;
                p
            };
            output.set(x, y, pixel);
        }
    }
    Ok(output)
}

fn resize_coverage(mask: &Coverage, width: u32, height: u32, method: Resample) -> Coverage {
    let mut values = vec![0; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let sample = |x, y| f32::from(mask.values[(y * mask.width + x) as usize]);
            let value = if method == Resample::Nearest {
                sample(
                    (u64::from(x) * u64::from(mask.width) / u64::from(width)) as u32,
                    (u64::from(y) * u64::from(mask.height) / u64::from(height)) as u32,
                )
            } else {
                let (x0, x1, tx) = axis(x, mask.width, width);
                let (y0, y1, ty) = axis(y, mask.height, height);
                (sample(x0, y0) * (1.0 - tx) + sample(x1, y0) * tx) * (1.0 - ty)
                    + (sample(x0, y1) * (1.0 - tx) + sample(x1, y1) * tx) * ty
            };
            values[(y * width + x) as usize] = value.round() as u8;
        }
    }
    Coverage {
        width,
        height,
        values: Arc::new(values),
        enabled: mask.enabled,
    }
}

/// Resample every layer while retaining its filters, effects and independent mask.
pub fn resize(
    document: &mut Document,
    width: u32,
    height: u32,
    method: Resample,
) -> Result<(), String> {
    Raster::new(width, height, [0; 4])?;
    if width == document.width && height == document.height {
        return Ok(());
    }
    let sx = f64::from(width) / f64::from(document.width);
    let sy = f64::from(height) / f64::from(document.height);
    let mut bytes = 0;
    for layer in &document.layers {
        reserve_plane(
            &mut bytes,
            (f64::from(layer.source.width()) * sx)
                .round()
                .clamp(1.0, u32::MAX as f64) as u32,
            (f64::from(layer.source.height()) * sy)
                .round()
                .clamp(1.0, u32::MAX as f64) as u32,
            4,
        )?;
    }
    if document.selection.is_some() {
        reserve_plane(&mut bytes, width, height, 1)?;
    }
    if document.selection_base.is_some() {
        reserve_plane(&mut bytes, width, height, 1)?;
    }
    // Masks may have different local dimensions from their source. Validate all
    // scaled planes before allocating any, including malicious oversized ratios.
    for mask in document.layers.iter().flat_map(|l| {
        l.mask
            .iter()
            .chain(l.filters.iter().filter_map(|f| f.mask.as_ref()))
    }) {
        let w = (f64::from(mask.width) * sx)
            .round()
            .clamp(1.0, u32::MAX as f64) as u32;
        let h = (f64::from(mask.height) * sy)
            .round()
            .clamp(1.0, u32::MAX as f64) as u32;
        reserve_plane(&mut bytes, w, h, 1)?;
    }
    let mut output = document.clone();
    for layer in &mut output.layers {
        let w = (f64::from(layer.source.width()) * sx)
            .round()
            .clamp(1.0, u32::MAX as f64) as u32;
        let h = (f64::from(layer.source.height()) * sy)
            .round()
            .clamp(1.0, u32::MAX as f64) as u32;
        layer.source = resample(&layer.source, w, h, method)?;
        let mask = |m: &Coverage| {
            resize_coverage(
                m,
                (f64::from(m.width) * sx)
                    .round()
                    .clamp(1.0, u32::MAX as f64) as u32,
                (f64::from(m.height) * sy)
                    .round()
                    .clamp(1.0, u32::MAX as f64) as u32,
                method,
            )
        };
        layer.mask = layer.mask.as_ref().map(mask);
        for filter in &mut layer.filters {
            filter.mask = filter.mask.as_ref().map(mask);
        }
        let x = (f64::from(layer.offset[0]) * sx).round();
        let y = (f64::from(layer.offset[1]) * sy).round();
        if x < f64::from(i32::MIN)
            || x > f64::from(i32::MAX)
            || y < f64::from(i32::MIN)
            || y > f64::from(i32::MAX)
        {
            return Err("Layer offset overflow".into());
        }
        layer.offset = [x as i32, y as i32];
    }
    output.selection = output
        .selection
        .as_ref()
        .map(|m| resize_coverage(m, width, height, method));
    output.selection_base = output
        .selection_base
        .as_ref()
        .map(|m| resize_coverage(m, width, height, method));
    output.width = width;
    output.height = height;
    *document = output;
    Ok(())
}

/// Resize canvas bounds without resampling. Existing source pixels are retained;
/// newly exposed mask/selection coverage is black, keeping hidden areas hidden.
pub fn canvas_resize(document: &mut Document, width: u32, height: u32) -> Result<(), String> {
    Raster::new(width, height, [0; 4])?;
    document.width = width;
    document.height = height;
    for plane in [&mut document.selection, &mut document.selection_base] {
        if let Some(mask) = plane {
            let mut values = vec![0; (width * height) as usize];
            for y in 0..height.min(mask.height) {
                for x in 0..width.min(mask.width) {
                    values[(y * width + x) as usize] = mask.values[(y * mask.width + x) as usize];
                }
            }
            *plane = Some(Coverage {
                width,
                height,
                values: Arc::new(values),
                enabled: mask.enabled,
            });
        }
    }
    Ok(())
}
