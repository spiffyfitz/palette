//! Grayscale selection operations, perceptual color sampling and layer masks.
//! Masks use canvas coordinates; layer masks are reanchored to source coordinates.
use crate::{composite, document::Document, raster::Coverage};
use std::{collections::HashMap, sync::Arc};

pub mod brush;

/// Closed freeform/lasso coverage. Four vertical subpixel samples and analytic
/// horizontal spans match the shape painter, including concave/self-crossing
/// contours (even/odd fill). The source document is never rasterized or changed.
pub fn freeform(width: u32, height: u32, points: &[[f32; 2]]) -> Result<Coverage, String> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > crate::raster::MAX_PIXELS
    {
        return Err("Selection dimensions must be positive and at most 64 megapixels".into());
    }
    if points.len() < 3
        || points.len() > 4096
        || points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
    {
        return Err("Freeform selection needs 3–4096 finite, bounded points".into());
    }
    let mut values = vec![0; (width * height) as usize];
    let path = crate::shapes::Path {
        points: points.to_vec(),
        closed: true,
    };
    crate::shapes::fill_coverage(width, height, &path, |x, y, amount| {
        values[(y * width + x) as usize] = (amount.clamp(0., 1.) * 255.).round() as u8;
    });
    Ok(Coverage {
        width,
        height,
        values: Arc::new(values),
        enabled: true,
    })
}

/// Alpha-coverage set operations. An absent selection means unconstrained canvas
/// for subtract/intersect; Add starts a new selection when there is none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Combine {
    #[default]
    Replace,
    Add,
    Subtract,
    Intersect,
}
impl Combine {
    /// Compact selection-options labels.
    pub fn label(self) -> &'static str {
        match self {
            Self::Replace => "Replace",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Intersect => "Intersect",
        }
    }
}

/// Combine masks without modifying either source. Product/intersection and
/// source-over union retain fractional coverage rather than thresholding it.
pub fn combine(
    current: Option<&Coverage>,
    incoming: &Coverage,
    mode: Combine,
) -> Result<Coverage, String> {
    incoming.validate()?;
    if let Some(c) = current {
        c.validate()?;
        if [c.width, c.height] != [incoming.width, incoming.height] {
            return Err("Selection sizes do not match".into());
        }
    }
    if incoming.enabled
        && (mode == Combine::Replace
            || (current.is_none() && matches!(mode, Combine::Add | Combine::Intersect)))
    {
        return Ok(incoming.clone());
    }
    let values = incoming
        .values
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let b = if incoming.enabled { u32::from(*b) } else { 255 };
            let a = current.map_or(if mode == Combine::Add { 0 } else { 255 }, |c| {
                if c.enabled {
                    u32::from(c.values[i])
                } else {
                    255
                }
            });
            let product = (a * b + 127) / 255;
            (match mode {
                Combine::Replace => b,
                Combine::Add => a + b - product,
                Combine::Subtract => (a * (255 - b) + 127) / 255,
                Combine::Intersect => product,
            }) as u8
        })
        .collect();
    Ok(Coverage {
        width: incoming.width,
        height: incoming.height,
        values: Arc::new(values),
        enabled: true,
    })
}

/// Commit a new region while preserving its unfeathered contour independently
/// from the coverage that painting/copy/masks consume. Combination uses the same
/// set algebra on both planes; a plain replacement drops old feather metadata.
pub fn apply_region(
    doc: &mut Document,
    incoming: &Coverage,
    mode: Combine,
    radius: f32,
) -> Result<bool, String> {
    if [incoming.width, incoming.height] != [doc.width, doc.height] {
        return Err("Selection dimensions must match the canvas".into());
    }
    let base = combine(
        doc.selection_base.as_ref().or(doc.selection.as_ref()),
        incoming,
        mode,
    )?;
    let softened = feather_region(incoming, radius)?;
    let coverage = combine(doc.selection.as_ref(), &softened, mode)?;
    let base = (coverage != base).then_some(base);
    let changed = doc.selection.as_ref() != Some(&coverage) || doc.selection_base != base;
    doc.selection = Some(coverage);
    doc.selection_base = base;
    Ok(changed)
}

/// Set feathering relative to the retained original, so radius zero restores it
/// and repeated adjustments do not compound the blur. Layer masks use `feather`.
pub fn feather_selection(doc: &mut Document, radius: f32) -> Result<bool, String> {
    let base = doc
        .selection_base
        .as_ref()
        .or(doc.selection.as_ref())
        .ok_or("No selection to feather")?
        .clone();
    let coverage = feather_region(&base, radius)?;
    let base = (coverage != base).then_some(base);
    let changed = doc.selection.as_ref() != Some(&coverage) || doc.selection_base != base;
    doc.selection = Some(coverage);
    doc.selection_base = base;
    Ok(changed)
}

/// Invert both contour and editing coverage without losing feather provenance.
pub fn invert(doc: &mut Document) {
    for plane in [&mut doc.selection, &mut doc.selection_base]
        .into_iter()
        .flatten()
    {
        for value in Arc::make_mut(&mut plane.values) {
            *value = 255 - *value;
        }
        plane.enabled = true;
    }
}

/// Magic Wand parameters. Tolerance is perceptual distance 0–100, not an RGB
/// channel cutoff. Connectivity uses the hard threshold before antialiasing.
#[derive(Clone, Copy, Debug)]
pub struct Wand {
    pub tolerance: f32,
    pub contiguous: bool,
    pub sample_merged: bool,
    pub antialias: bool,
}
impl Default for Wand {
    fn default() -> Self {
        Self {
            tolerance: 12.,
            contiguous: true,
            sample_merged: false,
            antialias: true,
        }
    }
}

// Oklab's published D65/sRGB matrices (Björn Ottosson, public-domain equations):
// https://bottosson.github.io/posts/oklab/ . Alpha scales perceptual coordinates;
// fully transparent hidden RGB never affects distance. Alpha is a fourth axis.
fn perceptual(p: [u8; 4]) -> [f64; 4] {
    let linear = |v: u8| {
        let s = f64::from(v) / 255.;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = [linear(p[0]), linear(p[1]), linear(p[2])];
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    let a = f64::from(p[3]) / 255.;
    [
        a * (0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s),
        a * (1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s),
        a * (0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s),
        a,
    ]
}

/// Select a connected color region or all matching canvas pixels. The original
/// image, masks, filters and selection are never changed by sampling.
pub fn wand(doc: &Document, point: [u32; 2], options: Wand) -> Result<Coverage, String> {
    doc.validate()?;
    if !options.tolerance.is_finite()
        || !(0. ..=100.).contains(&options.tolerance)
        || point[0] >= doc.width
        || point[1] >= doc.height
    {
        return Err("Invalid Magic Wand point or tolerance".into());
    }
    let rendered = options.sample_merged.then(|| composite::render(doc));
    let layer = doc.active();
    let prepared = (!options.sample_merged)
        .then(|| composite::Renderer::default().layer(doc, layer.id, &|| false))
        .transpose()?;
    let sample = |x: u32, y: u32| {
        if let Some(r) = &rendered {
            return r.get(x, y);
        }
        let (lx, ly) = (
            i64::from(x) - i64::from(layer.offset[0]),
            i64::from(y) - i64::from(layer.offset[1]),
        );
        let mut p = prepared.as_ref().expect("local sampler").pixel(lx, ly);
        p[3] = (f32::from(p[3]) * layer.opacity).round() as u8;
        p
    };
    let target = perceptual(sample(point[0], point[1]));
    let limit = f64::from(options.tolerance) / 100.;
    // Bounded exact-color memoization makes flat Paint drawings cheap without
    // allocating a perceptual-color plane for photographs or large documents.
    let mut colors = HashMap::new();
    let mut matches = |x, y| {
        if options.tolerance == 100. {
            return true;
        }
        let pixel = sample(x, y);
        if let Some(result) = colors.get(&pixel) {
            return *result;
        }
        let p = perceptual(pixel);
        let result = p
            .iter()
            .zip(target)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            <= limit * limit;
        if colors.len() < 4096 {
            colors.insert(pixel, result);
        }
        result
    };
    let (width, height) = (doc.width, doc.height);
    let mut values = vec![0; (width * height) as usize];
    if !options.contiguous {
        for y in 0..height {
            for x in 0..width {
                if matches(x, y) {
                    values[(y * width + x) as usize] = 255;
                }
            }
        }
    } else {
        // Scanline spans bound the frontier to runs instead of pushing a node
        // for every matching pixel. Coverage doubles as the visited bitmap.
        let mut stack = vec![(point[0], point[1])];
        while let Some((sx, y)) = stack.pop() {
            if values[(y * width + sx) as usize] != 0 || !matches(sx, y) {
                continue;
            }
            let mut x = sx;
            while x > 0 && values[(y * width + x - 1) as usize] == 0 && matches(x - 1, y) {
                x -= 1;
            }
            let (mut above, mut below) = (false, false);
            while x < width && values[(y * width + x) as usize] == 0 && matches(x, y) {
                values[(y * width + x) as usize] = 255;
                for (ny, run) in [
                    (y.checked_sub(1), &mut above),
                    ((y + 1 < height).then_some(y + 1), &mut below),
                ] {
                    let eligible = ny
                        .is_some_and(|ny| values[(ny * width + x) as usize] == 0 && matches(x, ny));
                    if eligible && !*run {
                        if stack.len() >= 1_048_576 {
                            return Err(
                                "Selection region exceeds the one-million pending-span budget"
                                    .into(),
                            );
                        }
                        stack.push((x, ny.expect("eligible span")));
                    }
                    *run = eligible;
                }
                x += 1;
            }
        }
    }
    if options.antialias {
        // One-pixel separable tent smooths the boundary, including exterior
        // partial coverage. Connectivity was decided before this smoothing.
        let mut temp = vec![0; values.len()];
        for vertical in [false, true] {
            for y in 0..height {
                for x in 0..width {
                    let mut sum = 0u32;
                    for (d, weight) in [(-1, 1), (0, 2), (1, 1)] {
                        let xx = (i64::from(x) + if vertical { 0 } else { d })
                            .clamp(0, i64::from(width) - 1) as u32;
                        let yy = (i64::from(y) + if vertical { d } else { 0 })
                            .clamp(0, i64::from(height) - 1)
                            as u32;
                        sum += u32::from(values[(yy * width + xx) as usize]) * weight;
                    }
                    temp[(y * width + x) as usize] = ((sum + 2) / 4) as u8;
                }
            }
            std::mem::swap(&mut values, &mut temp);
        }
    }
    Ok(Coverage {
        width,
        height,
        values: Arc::new(values),
        enabled: true,
    })
}

/// Gaussian feathering: pixel-integrated weights at small/subpixel radii, with
/// three separable running-sum box passes for larger radii.
/// Radius is sigma in image pixels (0–128). O(pixels), two byte planes, clamped
/// canvas edges; a full-canvas selection stays full. Stored coverage is edited
/// even when a mask is disabled; its enabled state and input remain intact.
pub fn feather(mask: &Coverage, radius: f32) -> Result<Coverage, String> {
    mask.validate()?;
    if !radius.is_finite() || !(0. ..=128.).contains(&radius) {
        return Err("Feather radius must be 0–128 pixels".into());
    }
    if radius == 0. {
        return Ok(mask.clone());
    }
    if radius < 5. {
        return Ok(subpixel_feather(mask, f64::from(radius)));
    }
    let mut values = mask.values.as_ref().clone();
    let mut temp = vec![0; values.len()];
    let sigma = f64::from(radius);
    let mut low = (4. * sigma * sigma + 1.).sqrt().floor() as u32;
    if low.is_multiple_of(2) {
        low = low.saturating_sub(1);
    }
    let low = low.max(1);
    let l = f64::from(low);
    let small = ((12. * sigma * sigma - 3. * l * l - 12. * l - 9.) / (-4. * l - 4.))
        .round()
        .clamp(0., 3.) as u32;
    for pass in 0..3 {
        let radius = (low + if pass < small { 0 } else { 2 }) / 2;
        for vertical in [false, true] {
            box_pass(
                &values,
                &mut temp,
                mask.width,
                mask.height,
                radius,
                vertical,
            );
            std::mem::swap(&mut values, &mut temp);
        }
    }
    Ok(Coverage {
        width: mask.width,
        height: mask.height,
        values: Arc::new(values),
        enabled: mask.enabled,
    })
}

/// Feather sparse incoming regions without convolving an otherwise empty
/// canvas. The conservative four-sigma-plus-four padding exceeds both finite
/// kernel supports. Zero padding at an interior crop border is therefore exact;
/// crop borders on canvas edges retain the reference implementation's clamping.
/// Returns full canvas coverage, byte-identical to `feather`.
pub fn feather_region(mask: &Coverage, radius: f32) -> Result<Coverage, String> {
    mask.validate()?;
    if !radius.is_finite() || !(0. ..=128.).contains(&radius) {
        return Err("Feather radius must be 0–128 pixels".into());
    }
    if radius == 0. {
        return Ok(mask.clone());
    }
    let Some(bounds) = crate::editing::bounds(mask) else {
        return Ok(mask.clone());
    };
    let pad = radius.ceil() as u32 * 4 + 4;
    let x = bounds.x.saturating_sub(pad);
    let y = bounds.y.saturating_sub(pad);
    let right = (bounds.x + bounds.width + pad).min(mask.width);
    let bottom = (bounds.y + bounds.height + pad).min(mask.height);
    let (width, height) = (right - x, bottom - y);
    if u64::from(width) * u64::from(height) * 2 >= u64::from(mask.width) * u64::from(mask.height) {
        return feather(mask, radius);
    }
    let mut crop = Vec::with_capacity((width * height) as usize);
    for row in y..bottom {
        let start = (row * mask.width + x) as usize;
        crop.extend_from_slice(&mask.values[start..start + width as usize]);
    }
    let cropped = Coverage {
        width,
        height,
        values: Arc::new(crop),
        enabled: mask.enabled,
    };
    let blurred = feather(&cropped, radius)?;
    let mut values = vec![0; mask.values.len()];
    for row in 0..height {
        let start = ((y + row) * mask.width + x) as usize;
        let source = (row * width) as usize;
        values[start..start + width as usize]
            .copy_from_slice(&blurred.values[source..source + width as usize]);
    }
    Ok(Coverage {
        width: mask.width,
        height: mask.height,
        values: Arc::new(values),
        enabled: mask.enabled,
    })
}

/// Integrating over both source and destination pixel areas gives a triangle
/// convolved with the Gaussian, rather than point samples that round every
/// 0.1–0.4 px radius to the identity. Its weights are second differences of
/// F(x) = x*Phi(x/sigma) + sigma*phi(x/sigma). Only the two byte-plane passes
/// round; the original coverage and its enabled state remain untouched.
fn subpixel_feather(mask: &Coverage, sigma: f64) -> Coverage {
    fn cdf(x: f64) -> f64 {
        let z = x.abs() / std::f64::consts::SQRT_2;
        if z >= 8. {
            return if x < 0. { 0. } else { 1. };
        }
        // Positive convergent series for erf(z), obtained by integrating its
        // derivative and factoring out exp(-z²). No cancellation near zero.
        let (mut term, mut sum) = (z, z);
        for n in 1..256 {
            term *= 2. * z * z / f64::from(2 * n + 1);
            sum += term;
            if term <= sum * 1e-15 {
                break;
            }
        }
        let erf = (2. / std::f64::consts::PI.sqrt()) * (-z * z).exp() * sum;
        (0.5 + 0.5 * erf.copysign(x)).clamp(0., 1.)
    }
    let integrated = |x: f64| {
        let z = x / sigma;
        x * cdf(z) + sigma * (-0.5 * z * z).exp() / (2. * std::f64::consts::PI).sqrt()
    };
    let reach = (4. * sigma + 1.).ceil() as i32;
    let mut weights: Vec<f32> = (-reach..=reach)
        .map(|k| {
            let k = f64::from(k);
            (integrated(k + 1.) - 2. * integrated(k) + integrated(k - 1.)).max(0.) as f32
        })
        .collect();
    let total: f32 = weights.iter().sum();
    for weight in &mut weights {
        *weight /= total;
    }
    let mut temp = vec![0; mask.values.len()];
    let mut values = vec![0; mask.values.len()];
    gaussian_pass(
        &mask.values,
        &mut temp,
        mask.width,
        mask.height,
        &weights,
        reach,
        false,
    );
    gaussian_pass(
        &temp,
        &mut values,
        mask.width,
        mask.height,
        &weights,
        reach,
        true,
    );
    Coverage {
        width: mask.width,
        height: mask.height,
        values: Arc::new(values),
        enabled: mask.enabled,
    }
}
fn gaussian_pass(
    src: &[u8],
    dst: &mut [u8],
    width: u32,
    height: u32,
    weights: &[f32],
    reach: i32,
    vertical: bool,
) {
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.;
            for (i, weight) in weights.iter().enumerate() {
                let offset = i as i64 - i64::from(reach);
                let xx = (i64::from(x) + if vertical { 0 } else { offset })
                    .clamp(0, i64::from(width) - 1) as u32;
                let yy = (i64::from(y) + if vertical { offset } else { 0 })
                    .clamp(0, i64::from(height) - 1) as u32;
                sum += f32::from(src[(yy * width + xx) as usize]) * weight;
            }
            dst[(y * width + x) as usize] = sum.round().clamp(0., 255.) as u8;
        }
    }
}
fn box_pass(src: &[u8], dst: &mut [u8], width: u32, height: u32, r: u32, vertical: bool) {
    let (lines, len) = if vertical {
        (width, height)
    } else {
        (height, width)
    };
    let count = 2 * r + 1;
    for line in 0..lines {
        let index = |p: i64| {
            let p = p.clamp(0, i64::from(len) - 1) as u32;
            if vertical {
                (p * width + line) as usize
            } else {
                (line * width + p) as usize
            }
        };
        let mut sum: u32 = (-(i64::from(r))..=i64::from(r))
            .map(|p| u32::from(src[index(p)]))
            .sum();
        for p in 0..len {
            dst[index(i64::from(p))] = ((sum + count / 2) / count) as u8;
            sum -= u32::from(src[index(i64::from(p) - i64::from(r))]);
            sum += u32::from(src[index(i64::from(p) + i64::from(r) + 1)]);
        }
    }
}

/// Create a local mask from canvas-space selection coverage, respecting signed
/// layer offsets. None creates a white reveal-all mask. Source pixels stay intact.
pub fn mask_from_selection(doc: &mut Document) -> Result<bool, String> {
    doc.validate()?;
    if doc.active().locked {
        return Err("Unlock the layer before editing its mask".into());
    }
    let layer = doc.active();
    let (w, h) = (layer.source.width(), layer.source.height());
    if layer.offset == [0, 0]
        && [w, h] == [doc.width, doc.height]
        && let Some(mask) = doc.selection.as_ref().filter(|m| m.enabled)
    {
        // Common full-canvas masks share immutable coverage with the selection.
        // Later mask edits use Arc::make_mut, preserving selection/history and
        // avoiding a second dense 64 MiB plane for a large untouched mask.
        let mask = mask.clone();
        let changed = doc.active().mask.as_ref() != Some(&mask);
        doc.active_mut().mask = Some(mask);
        return Ok(changed);
    }
    let mut values = vec![255; (w * h) as usize];
    if let Some(selection) = &doc.selection {
        for y in 0..h {
            for x in 0..w {
                values[(y * w + x) as usize] = (255.
                    * crate::editing::coverage(
                        Some(selection),
                        i64::from(x) + i64::from(layer.offset[0]),
                        i64::from(y) + i64::from(layer.offset[1]),
                        doc.width,
                        doc.height,
                    ))
                .round() as u8;
            }
        }
    }
    let mask = Coverage {
        width: w,
        height: h,
        values: Arc::new(values),
        enabled: true,
    };
    let changed = doc.active().mask.as_ref() != Some(&mask);
    doc.active_mut().mask = Some(mask);
    Ok(changed)
}
