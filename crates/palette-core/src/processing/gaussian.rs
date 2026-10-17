//! Premultiplied fixed-point filtering with bounded tile/halo scratch storage.
//! Small sigmas use a sampled Gaussian; larger ones use three separable boxes
//! (the conventional Gaussian approximation described by SVG Filter Effects).
use crate::raster::Raster;
const TILE: u32 = 256;
const SCALE: u64 = 1 << 20;
type Pixel = [u16; 4];

fn boxes(sigma: f32) -> [u32; 3] {
    let s = f64::from(sigma);
    let mut low = (4. * s * s + 1.).sqrt().floor() as u32;
    if low.is_multiple_of(2) {
        low = low.saturating_sub(1);
    }
    let low = low.max(1);
    let l = f64::from(low);
    let small = ((12. * s * s - 3. * l * l - 12. * l - 9.) / (-4. * l - 4.))
        .round()
        .clamp(0., 3.) as usize;
    std::array::from_fn(|i| (low + if i < small { 0 } else { 2 }) / 2)
}
pub(super) fn support(sigma: f32) -> u32 {
    if sigma == 0. {
        0
    } else if sigma < 2. {
        (3. * sigma).ceil() as u32
    } else {
        boxes(sigma).iter().sum()
    }
}
fn kernel(sigma: f32) -> Vec<u64> {
    let reach = support(sigma) as i32;
    let values: Vec<_> = (-reach..=reach)
        .map(|x| (-0.5 * (f64::from(x) / f64::from(sigma)).powi(2)).exp())
        .collect();
    let total: f64 = values.iter().sum();
    let mut fixed: Vec<_> = values
        .iter()
        .map(|v| (v / total * SCALE as f64).round() as u64)
        .collect();
    let sum: u64 = fixed.iter().sum();
    fixed[reach as usize] = (fixed[reach as usize] as i64 + SCALE as i64 - sum as i64) as u64;
    fixed
}
pub(super) fn pass(
    src: &[Pixel],
    dst: &mut [Pixel],
    dimensions: [usize; 2],
    r: u32,
    weights: Option<&[u64]>,
    vertical: bool,
    cancel: &dyn Fn() -> bool,
) -> Result<(), String> {
    let [w, h] = dimensions;
    let (lines, len) = if vertical { (w, h) } else { (h, w) };
    let r = r as i64;
    for line in 0..lines {
        if cancel() {
            return Err("Processing cancelled".into());
        }
        let index = |p: i64| {
            let p = p.clamp(0, len as i64 - 1) as usize;
            if vertical { p * w + line } else { line * w + p }
        };
        if let Some(weights) = weights {
            for p in 0..len {
                let mut sum = [0u64; 4];
                for (i, weight) in weights.iter().enumerate() {
                    let pixel = src[index(p as i64 + i as i64 - r)];
                    for c in 0..4 {
                        sum[c] += u64::from(pixel[c]) * weight;
                    }
                }
                dst[index(p as i64)] = sum.map(|v| ((v + SCALE / 2) / SCALE) as u16);
            }
        } else {
            let count = (2 * r + 1) as u64;
            let mut sum = [0u64; 4];
            for p in -r..=r {
                let pixel = src[index(p)];
                for c in 0..4 {
                    sum[c] += u64::from(pixel[c]);
                }
            }
            for p in 0..len {
                dst[index(p as i64)] = sum.map(|v| ((v + count / 2) / count) as u16);
                let old = src[index(p as i64 - r)];
                let new = src[index(p as i64 + r + 1)];
                for c in 0..4 {
                    sum[c] = sum[c] - u64::from(old[c]) + u64::from(new[c]);
                }
            }
        }
    }
    Ok(())
}
/// Filter RGBA together in premultiplied space, then return straight RGBA8.
/// Transparent hidden colors cannot bleed into soft edges. Boundaries clamp.
pub fn gaussian_blur(source: &Raster, radius: f32) -> Result<Raster, String> {
    gaussian_blur_cancel(source, radius, &|| false)
}
/// Cancellation is checked between tile scanlines/passes; partial output never
/// escapes. Integer running sums make output independent of tile boundaries.
pub fn gaussian_blur_cancel(
    source: &Raster,
    radius: f32,
    cancel: &dyn Fn() -> bool,
) -> Result<Raster, String> {
    source.validate()?;
    if !radius.is_finite() || !(0. ..=64.).contains(&radius) {
        return Err("Gaussian radius must be 0–64 pixels".into());
    }
    if radius == 0. {
        return Ok(source.clone());
    }
    let pad = support(radius);
    let weights = (radius < 2.).then(|| kernel(radius));
    let radii = if weights.is_some() {
        vec![pad]
    } else {
        boxes(radius).to_vec()
    };
    let mut result = Raster::new(source.width(), source.height(), [0; 4])?;
    for top in (0..source.height()).step_by(TILE as usize) {
        for left in (0..source.width()).step_by(TILE as usize) {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            let tw = TILE.min(source.width() - left);
            let th = TILE.min(source.height() - top);
            let w = (tw + 2 * pad) as usize;
            let h = (th + 2 * pad) as usize;
            let mut a = vec![[0; 4]; w * h];
            let mut b = vec![[0; 4]; w * h];
            for y in 0..h {
                for x in 0..w {
                    let sx = (i64::from(left) + x as i64 - i64::from(pad))
                        .clamp(0, i64::from(source.width()) - 1)
                        as u32;
                    let sy = (i64::from(top) + y as i64 - i64::from(pad))
                        .clamp(0, i64::from(source.height()) - 1)
                        as u32;
                    let p = source.get(sx, sy);
                    let alpha = u16::from(p[3]);
                    a[y * w + x] = [
                        u16::from(p[0]) * alpha,
                        u16::from(p[1]) * alpha,
                        u16::from(p[2]) * alpha,
                        alpha * 255,
                    ];
                }
            }
            for r in &radii {
                for vertical in [false, true] {
                    pass(&a, &mut b, [w, h], *r, weights.as_deref(), vertical, cancel)?;
                    std::mem::swap(&mut a, &mut b);
                }
            }
            for y in 0..th {
                for x in 0..tw {
                    let p = a[((y + pad) as usize) * w + (x + pad) as usize];
                    let alpha = u64::from(p[3]);
                    let channel = |v: u16| {
                        (u64::from(v) * 255 + alpha / 2)
                            .checked_div(alpha)
                            .unwrap_or(0)
                            .min(255) as u8
                    };
                    let rgba = [
                        channel(p[0]),
                        channel(p[1]),
                        channel(p[2]),
                        ((alpha + 127) / 255).min(255) as u8,
                    ];
                    result.set(left + x, top + y, rgba);
                }
            }
        }
    }
    Ok(result)
}
