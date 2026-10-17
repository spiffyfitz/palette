//! Independent blend equations: W3C Compositing Level 1, sections 10.1/10.2.
//! https://www.w3.org/TR/compositing-1/#blending
//! RGB functions are evaluated before alpha coverage; HSL-like modes use the
//! specified luminosity/saturation helpers, not HSV or channel-wise blending.
use crate::document::BlendMode;

fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}
fn sat(c: [f32; 3]) -> f32 {
    c.into_iter().fold(0., f32::max) - c.into_iter().fold(1., f32::min)
}
fn set_lum(c: [f32; 3], target: f32) -> [f32; 3] {
    let mut c = c.map(|v| v + target - lum(c));
    let low = c.into_iter().fold(f32::INFINITY, f32::min);
    let high = c.into_iter().fold(f32::NEG_INFINITY, f32::max);
    if low < 0. {
        c = c.map(|v| target + (v - target) * target / (target - low));
    }
    if high > 1. {
        c = c.map(|v| target + (v - target) * (1. - target) / (high - target));
    }
    c.map(|v| v.clamp(0., 1.))
}
fn set_sat(c: [f32; 3], target: f32) -> [f32; 3] {
    let mut order = [0, 1, 2];
    order.sort_by(|a, b| c[*a].total_cmp(&c[*b]));
    let [lo, mid, hi] = order;
    let mut out = [0.; 3];
    if c[hi] > c[lo] {
        out[mid] = (c[mid] - c[lo]) * target / (c[hi] - c[lo]);
        out[hi] = target;
    }
    out
}
fn hard(b: f32, f: f32) -> f32 {
    if f <= 0.5 {
        2. * b * f
    } else {
        1. - 2. * (1. - b) * (1. - f)
    }
}

pub(super) fn rgb(mode: BlendMode, b: [f32; 3], f: [f32; 3]) -> [f32; 3] {
    use BlendMode::*;
    match mode {
        Hue => return set_lum(set_sat(f, sat(b)), lum(b)),
        Saturation => return set_lum(set_sat(b, sat(f)), lum(b)),
        Color => return set_lum(f, lum(b)),
        Luminosity => return set_lum(b, lum(f)),
        _ => {}
    }
    std::array::from_fn(|i| {
        let (b, f) = (b[i], f[i]);
        match mode {
            Normal => f,
            Multiply => b * f,
            Screen => b + f - b * f,
            Darken => b.min(f),
            Lighten => b.max(f),
            Overlay => hard(f, b),
            HardLight => hard(b, f),
            ColorDodge => {
                if b == 0. {
                    0.
                } else if f == 1. {
                    1.
                } else {
                    (b / (1. - f)).min(1.)
                }
            }
            ColorBurn => {
                if b == 1. {
                    1.
                } else if f == 0. {
                    0.
                } else {
                    1. - ((1. - b) / f).min(1.)
                }
            }
            LinearDodge => (b + f).min(1.),
            LinearBurn => (b + f - 1.).max(0.),
            Difference => (b - f).abs(),
            Exclusion => b + f - 2. * b * f,
            Divide => {
                if f == 0. {
                    1.
                } else {
                    (b / f).min(1.)
                }
            }
            SoftLight => {
                if f <= 0.5 {
                    b - (1. - 2. * f) * b * (1. - b)
                } else {
                    let d = if b <= 0.25 {
                        ((16. * b - 12.) * b + 4.) * b
                    } else {
                        b.sqrt()
                    };
                    b + (2. * f - 1.) * (d - b)
                }
            }
            Hue | Saturation | Color | Luminosity => {
                unreachable!("nonseparable modes handled above")
            }
        }
    })
}
