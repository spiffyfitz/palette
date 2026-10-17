//! Bounded-scratch neighborhood filters. Both process premultiplied 16-bit
//! channels so hidden transparent RGB cannot contaminate antialiased edges.
use crate::raster::Raster;

fn premultiplied(p: [u8; 4]) -> [u16; 4] {
    let a = u16::from(p[3]);
    [
        u16::from(p[0]) * a,
        u16::from(p[1]) * a,
        u16::from(p[2]) * a,
        a * 255,
    ]
}
fn straight(p: [u16; 4]) -> [u8; 4] {
    let a = u32::from(p[3]);
    if a == 0 {
        return [0; 4];
    }
    [
        ((u32::from(p[0]) * 255 + a / 2) / a).min(255) as u8,
        ((u32::from(p[1]) * 255 + a / 2) / a).min(255) as u8,
        ((u32::from(p[2]) * 255 + a / 2) / a).min(255) as u8,
        ((a + 127) / 255) as u8,
    ]
}
fn sample(source: &Raster, x: i64, y: i64) -> [u16; 4] {
    premultiplied(source.get(
        x.clamp(0, i64::from(source.width()) - 1) as u32,
        y.clamp(0, i64::from(source.height()) - 1) as u32,
    ))
}
/// Separable box average with a complete halo around each 256-pixel tile.
/// The radius is a whole number of pixels, 0–32; image boundaries clamp.
pub fn box_blur_cancel(
    source: &Raster,
    radius: u8,
    cancel: &dyn Fn() -> bool,
) -> Result<Raster, String> {
    source.validate()?;
    if radius > 32 {
        return Err("Box radius must be 0–32 pixels".into());
    }
    if radius == 0 {
        return Ok(source.clone());
    }
    let r = u32::from(radius);
    let mut output = Raster::new(source.width(), source.height(), [0; 4])?;
    for top in (0..source.height()).step_by(256) {
        for left in (0..source.width()).step_by(256) {
            let tw = 256.min(source.width() - left);
            let th = 256.min(source.height() - top);
            let w = (tw + 2 * r) as usize;
            let h = (th + 2 * r) as usize;
            let mut a = vec![[0; 4]; w * h];
            let mut b = vec![[0; 4]; w * h];
            for y in 0..h {
                if cancel() {
                    return Err("Processing cancelled".into());
                }
                for x in 0..w {
                    a[y * w + x] = sample(
                        source,
                        i64::from(left) + x as i64 - i64::from(r),
                        i64::from(top) + y as i64 - i64::from(r),
                    );
                }
            }
            super::gaussian::pass(&a, &mut b, [w, h], r, None, false, cancel)?;
            super::gaussian::pass(&b, &mut a, [w, h], r, None, true, cancel)?;
            for y in 0..th {
                if cancel() {
                    return Err("Processing cancelled".into());
                }
                for x in 0..tw {
                    output.set(
                        left + x,
                        top + y,
                        straight(a[((y + r) as usize) * w + (x + r) as usize]),
                    );
                }
            }
        }
    }
    Ok(output)
}
/// Component-wise median of premultiplied RGBA, radius 0–5. Rank selection
/// preserves 16-bit intermediate precision and uses only one window's scratch.
pub fn median_cancel(
    source: &Raster,
    radius: u8,
    cancel: &dyn Fn() -> bool,
) -> Result<Raster, String> {
    source.validate()?;
    if radius > 5 {
        return Err("Median radius must be 0–5 pixels".into());
    }
    if radius == 0 {
        return Ok(source.clone());
    }
    let r = i64::from(radius);
    let count = ((2 * r + 1).pow(2)) as usize;
    let mut channels: [Vec<u16>; 4] = std::array::from_fn(|_| Vec::with_capacity(count));
    let mut output = Raster::new(source.width(), source.height(), [0; 4])?;
    for y in 0..source.height() {
        if cancel() {
            return Err("Processing cancelled".into());
        }
        for x in 0..source.width() {
            for channel in &mut channels {
                channel.clear();
            }
            for dy in -r..=r {
                for dx in -r..=r {
                    let p = sample(source, i64::from(x) + dx, i64::from(y) + dy);
                    for c in 0..4 {
                        channels[c].push(p[c]);
                    }
                }
            }
            let p = std::array::from_fn(|c| *channels[c].select_nth_unstable(count / 2).1);
            output.set(x, y, straight(p));
        }
    }
    Ok(output)
}
