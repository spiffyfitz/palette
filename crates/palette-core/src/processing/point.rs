//! Shared point-filter mathematics for both fast point compositing and spatial
//! pipelines. Point adjustments preserve alpha; procedural generators use the
//! spatial path because they can introduce new coverage.
use crate::document::{Filter, NoiseDistribution};

fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn encoded(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn hsl(rgb: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = rgb;
    let hi = r.max(g).max(b);
    let lo = r.min(g).min(b);
    let delta = hi - lo;
    let l = (hi + lo) * 0.5;
    if delta == 0. {
        return [0., 0., l];
    }
    let hue = if hi == r {
        ((g - b) / delta).rem_euclid(6.)
    } else if hi == g {
        (b - r) / delta + 2.
    } else {
        (r - g) / delta + 4.
    };
    [hue / 6., delta / (1. - (2. * l - 1.).abs()), l]
}
fn rgb([h, s, l]: [f32; 3]) -> [f32; 3] {
    let c = (1. - (2. * l - 1.).abs()) * s;
    let h = h.rem_euclid(1.) * 6.;
    let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
    let base = match h as u8 {
        0 => [c, x, 0.],
        1 => [x, c, 0.],
        2 => [0., c, x],
        3 => [0., x, c],
        4 => [x, 0., c],
        _ => [c, 0., x],
    };
    base.map(|v| v + l - c * 0.5)
}

pub(crate) fn point_filter(filter: &Filter, pixel: [u8; 4], x: u32, y: u32) -> [u8; 4] {
    if let Filter::ColorLookup {
        lut: Some(lut),
        amount,
    } = filter
    {
        return super::editable::lookup(pixel, lut, *amount);
    }
    if matches!(filter, Filter::NtscColors { .. } | Filter::HsbHsl { .. }) {
        return super::video::point(pixel, filter);
    }
    let input = std::array::from_fn(|c| f32::from(pixel[c]) / 255.);
    let luminance = || input[0] * 0.2126 + input[1] * 0.7152 + input[2] * 0.0722;
    let result = match *filter {
        Filter::SkinSmoothing { .. }
        | Filter::Colorize { .. }
        | Filter::JpegArtifactsRemoval { .. }
        | Filter::StyleTransfer { .. }
        | Filter::FieldBlur { .. }
        | Filter::IrisBlur { .. }
        | Filter::TiltShift { .. }
        | Filter::PathBlur { .. }
        | Filter::SpinBlur { .. }
        | Filter::Flame { .. }
        | Filter::PictureFrame { .. }
        | Filter::Tree { .. }
        | Filter::Deinterlace { .. }
        | Filter::NtscColors { .. }
        | Filter::HsbHsl { .. }
        | Filter::Chrome { .. }
        | Filter::ConteCrayon { .. }
        | Filter::Plaster { .. }
        | Filter::Reticulation { .. }
        | Filter::TornEdges { .. }
        | Filter::Fresco { .. }
        | Filter::NeonGlow { .. }
        | Filter::PaletteKnife { .. }
        | Filter::PlasticWrap { .. }
        | Filter::RoughPastels { .. }
        | Filter::Sponge { .. }
        | Filter::Underpainting { .. }
        | Filter::DiffuseGlow { .. }
        | Filter::Glass { .. }
        | Filter::OceanRipple { .. }
        | Filter::Texturizer { .. }
        | Filter::ShapeBlur { .. }
        | Filter::SmartBlur { .. }
        | Filter::SharpenEdges
        | Filter::SmartSharpen { .. }
        | Filter::Extrude { .. }
        | Filter::OilPaint { .. }
        | Filter::Tiles { .. }
        | Filter::TraceContour { .. }
        | Filter::Wind { .. }
        | Filter::LensBlur { .. }
        | Filter::LensFlare { .. }
        | Filter::LightingEffects { .. }
        | Filter::Displace { .. }
        | Filter::Shear { .. }
        | Filter::Custom { .. }
        | Filter::ColorLookup { .. }
        | Filter::Watercolor { .. }
        | Filter::WaterPaper { .. } => return pixel,
        Filter::SprayedStrokes { .. }
        | Filter::SumiE { .. }
        | Filter::BasRelief { .. }
        | Filter::ChalkCharcoal { .. }
        | Filter::Charcoal { .. }
        | Filter::GraphicPen { .. }
        | Filter::NotePaper { .. }
        | Filter::Stamp { .. } => return pixel,
        Filter::ColoredPencil { .. }
        | Filter::DryBrush { .. }
        | Filter::PaintDaubs { .. }
        | Filter::SmudgeStick { .. }
        | Filter::AngledStrokes { .. }
        | Filter::Crosshatch { .. }
        | Filter::DarkStrokes { .. }
        | Filter::Spatter { .. } => return pixel,
        Filter::ColorBalance { .. }
        | Filter::PhotoFilter { .. }
        | Filter::ChannelMixer { .. }
        | Filter::GradientMap { .. }
        | Filter::SelectiveColor { .. } => return super::adjustments::point(filter, pixel),
        Filter::ShadowsHighlights { .. } | Filter::Blur | Filter::BlurMore => return pixel,
        Filter::Mezzotint { .. } => return super::pixelate::mezzotint(filter, pixel, x, y),
        Filter::ColorHalftone { .. }
        | Filter::Crystallize { .. }
        | Filter::Facet
        | Filter::Pointillize { .. }
        | Filter::StainedGlass { .. }
        | Filter::MosaicTiles { .. }
        | Filter::Diffuse { .. } => return pixel,
        Filter::FilmGrain { .. } | Filter::Craquelure { .. } => {
            return super::artistic::point(filter, pixel, x, y);
        }
        Filter::Cutout { .. }
        | Filter::AccentedEdges { .. }
        | Filter::InkOutlines { .. }
        | Filter::Photocopy { .. }
        | Filter::HalftonePattern { .. }
        | Filter::Patchwork { .. } => return pixel,
        Filter::PolarCoordinates { .. }
        | Filter::ZigZag { .. }
        | Filter::Fibers { .. }
        | Filter::GlowingEdges { .. } => return pixel,
        Filter::Grain { .. } => return super::creative::grain(pixel, x, y, filter),
        Filter::Levels {
            black,
            white,
            gamma,
            output_black,
            output_white,
        } => input.map(|v| {
            let t = ((v * 255. - f32::from(black)) / f32::from(white - black))
                .clamp(0., 1.)
                .powf(1. / gamma);
            (f32::from(output_black) + (f32::from(output_white) - f32::from(output_black)) * t)
                / 255.
        }),
        Filter::Curves { points } => input.map(|v| curve(points, v * 255.) / 255.),
        Filter::AddNoise {
            amount,
            seed,
            monochrome,
            distribution,
        } => {
            if amount == 0. {
                return pixel;
            }
            std::array::from_fn(|c| {
                input[c]
                    + amount
                        * noise(
                            x,
                            y,
                            seed,
                            if monochrome { 0 } else { c as u32 },
                            distribution,
                        )
            })
        }

        Filter::Solarize => {
            return [
                if pixel[0] > 127 {
                    255 - pixel[0]
                } else {
                    pixel[0]
                },
                if pixel[1] > 127 {
                    255 - pixel[1]
                } else {
                    pixel[1]
                },
                if pixel[2] > 127 {
                    255 - pixel[2]
                } else {
                    pixel[2]
                },
                pixel[3],
            ];
        }
        Filter::Invert => return [255 - pixel[0], 255 - pixel[1], 255 - pixel[2], pixel[3]],
        Filter::Brightness { amount } => {
            return [
                (f32::from(pixel[0]) + amount * 255.)
                    .round()
                    .clamp(0., 255.) as u8,
                (f32::from(pixel[1]) + amount * 255.)
                    .round()
                    .clamp(0., 255.) as u8,
                (f32::from(pixel[2]) + amount * 255.)
                    .round()
                    .clamp(0., 255.) as u8,
                pixel[3],
            ];
        }
        Filter::BrightnessContrast {
            brightness,
            contrast,
        } => {
            let gain = (1. + contrast) / (1. - contrast);
            input.map(|v| ((v - 0.5) * gain + 0.5) + brightness)
        }
        Filter::Exposure { stops } => {
            if stops == 0. {
                return pixel;
            }
            input.map(|v| encoded((linear(v) * stops.exp2()).clamp(0., 1.)))
        }
        Filter::HueSaturation {
            hue,
            saturation,
            lightness,
        } => {
            if hue == 0. && saturation == 0. && lightness == 0. {
                return pixel;
            }
            let [h, s, l] = hsl(input);
            rgb([
                h + hue / 360.,
                (s * (1. + saturation)).clamp(0., 1.),
                (l + lightness).clamp(0., 1.),
            ])
        }
        Filter::Vibrance { amount } => {
            if amount == 0. {
                return pixel;
            }
            let [h, s, l] = hsl(input);
            // Prefer lower-saturation colors. This is an independent HSL
            // operation, without proprietary skin-tone protection heuristics.
            rgb([h, (s * (1. + amount * (1. - s))).clamp(0., 1.), l])
        }
        Filter::BlackWhite { strength } => input.map(|v| v + (luminance() - v) * strength),
        Filter::Threshold { level } => {
            [if luminance() * 255. >= f32::from(level) {
                1.
            } else {
                0.
            }; 3]
        }
        Filter::Posterize { levels } => {
            let steps = f32::from(levels - 1);
            input.map(|v| (v * steps).round() / steps)
        }
        // Zero-radius neighborhood operations can use the point fast path.
        Filter::Despeckle
        | Filter::DustScratches { .. }
        | Filter::ReduceNoise { .. }
        | Filter::Pinch { .. }
        | Filter::Spherize { .. }
        | Filter::Ripple { .. }
        | Filter::Sharpen
        | Filter::SharpenMore
        | Filter::Clouds { .. }
        | Filter::DifferenceClouds { .. }
        | Filter::SurfaceBlur { .. }
        | Filter::RadialBlur { .. }
        | Filter::PosterEdges { .. }
        | Filter::Fragment { .. }
        | Filter::Average
        | Filter::FindEdges
        | Filter::Emboss { .. }
        | Filter::UnsharpMask { .. }
        | Filter::Minimum { .. }
        | Filter::Maximum { .. }
        | Filter::Offset { .. }
        | Filter::Twirl { .. }
        | Filter::Wave { .. }
        | Filter::GaussianBlur { .. }
        | Filter::BoxBlur { .. }
        | Filter::Median { .. }
        | Filter::MotionBlur { .. }
        | Filter::HighPass { .. }
        | Filter::Mosaic { .. } => {
            return pixel;
        }
    };
    [
        (result[0] * 255.).round().clamp(0., 255.) as u8,
        (result[1] * 255.).round().clamp(0., 255.) as u8,
        (result[2] * 255.).round().clamp(0., 255.) as u8,
        pixel[3],
    ]
}

// Cubic Hermite interpolation at five fixed input positions. Endpoint and
// centered slopes preserve the exact identity curve; overshoots are clipped.
fn curve(points: [u8; 5], value: f32) -> f32 {
    let knots = [0., 64., 128., 192., 255.];
    let i = if value < 64. {
        0
    } else if value < 128. {
        1
    } else if value < 192. {
        2
    } else {
        3
    };
    let slope = |j: usize| {
        let left = j.saturating_sub(1);
        let right = (j + 1).min(4);
        (f32::from(points[right]) - f32::from(points[left])) / (knots[right] - knots[left])
    };
    let width = knots[i + 1] - knots[i];
    let t = (value - knots[i]) / width;
    let t2 = t * t;
    let t3 = t2 * t;
    ((2. * t3 - 3. * t2 + 1.) * f32::from(points[i])
        + (t3 - 2. * t2 + t) * width * slope(i)
        + (-2. * t3 + 3. * t2) * f32::from(points[i + 1])
        + (t3 - t2) * width * slope(i + 1))
    .clamp(0., 255.)
}
// Coordinate hashing fixes the random field independently of evaluation order,
// tiles or thread scheduling. Gaussian uses independently hashed Box–Muller inputs.
fn noise(x: u32, y: u32, seed: u16, channel: u32, distribution: NoiseDistribution) -> f32 {
    let mut n = 0xa511e9b3
        ^ x.wrapping_mul(0x9e3779b9)
        ^ y.wrapping_mul(0x85ebca6b)
        ^ u32::from(seed).wrapping_mul(0xc2b2ae35)
        ^ channel.wrapping_mul(0x27d4eb2d);
    let mut uniform = || {
        n ^= n >> 16;
        n = n.wrapping_mul(0x7feb352d);
        n ^= n >> 15;
        n = n.wrapping_mul(0x846ca68b);
        n ^= n >> 16;
        (f64::from(n) + 0.5) / 4294967296.
    };
    match distribution {
        NoiseDistribution::Uniform => (uniform() * 2. - 1.) as f32,
        NoiseDistribution::Gaussian => {
            ((-2. * uniform().ln()).sqrt() * (std::f64::consts::TAU * uniform()).cos()) as f32
        }
    }
}
