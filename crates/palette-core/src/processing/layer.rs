//! Ordered filters -> color overlays -> mask -> live inner/outer styles.
use super::*;
use crate::{
    composite::over,
    document::{BlendMode, Layer, StrokePosition},
    raster::Raster,
};
use std::collections::VecDeque;

struct Style {
    pixels: Raster,
    origin: [f32; 2],
    color: [u8; 3],
    opacity: f32,
    outer: bool,
    inner: bool,
    inverse: bool,
}
/// Immutable evaluated appearance, including spatial style extents.
pub struct PreparedLayer {
    pub(super) original: Layer,
    pixels: Option<Raster>,
    styles: Vec<Style>,
}
impl PreparedLayer {
    /// Signed source-space lookup includes styles outside the source bounds.
    pub fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        let mut front = if x < 0
            || y < 0
            || x >= i64::from(self.original.source.width())
            || y >= i64::from(self.original.source.height())
        {
            [0; 4]
        } else if let Some(p) = &self.pixels {
            p.get(x as u32, y as u32)
        } else {
            crate::composite::point_pixel(&self.original, x as u32, y as u32)
        };
        let mut under = [0; 4];
        if self.styles.is_empty() {
            return front;
        }
        for effect in &self.styles {
            let px = x as f32 - effect.origin[0];
            let py = y as f32 - effect.origin[1];
            let (ix, iy) = (px.floor() as i64, py.floor() as i64);
            let (fx, fy) = (px - px.floor(), py - py.floor());
            let at = |x: i64, y: i64| {
                if x < 0
                    || y < 0
                    || x >= i64::from(effect.pixels.width())
                    || y >= i64::from(effect.pixels.height())
                {
                    0.
                } else {
                    f32::from(effect.pixels.get(x as u32, y as u32)[3])
                }
            };
            let a = (at(ix, iy) * (1. - fx) + at(ix + 1, iy) * fx) * (1. - fy)
                + (at(ix, iy + 1) * (1. - fx) + at(ix + 1, iy + 1) * fx) * fy;
            let a = if effect.inverse { 255. - a } else { a };
            if effect.inner {
                // Inner styles recolor through existing coverage. They cannot
                // create opacity or reveal pixels hidden by the layer mask.
                let coverage = a / 255. * effect.opacity;
                for (c, color) in effect.color.iter().enumerate() {
                    front[c] = (f32::from(front[c]) * (1. - coverage)
                        + f32::from(*color) * coverage)
                        .round() as u8;
                }
                continue;
            }
            let alpha = a
                * effect.opacity
                * if effect.outer {
                    1. - f32::from(front[3]) / 255.
                } else {
                    1.
                };
            under = over(
                under,
                [
                    effect.color[0],
                    effect.color[1],
                    effect.color[2],
                    alpha.round() as u8,
                ],
                1.,
                BlendMode::Normal,
            );
        }
        over(under, front, 1., BlendMode::Normal)
    }
    /// Source-space half-open bounds: left, top, right, bottom.
    pub fn bounds(&self) -> [i64; 4] {
        let mut bounds = [
            0,
            0,
            i64::from(self.original.source.width()),
            i64::from(self.original.source.height()),
        ];
        for e in &self.styles {
            bounds[0] = bounds[0].min(e.origin[0].floor() as i64);
            bounds[1] = bounds[1].min(e.origin[1].floor() as i64);
            bounds[2] = bounds[2].max((e.origin[0] + e.pixels.width() as f32).ceil() as i64);
            bounds[3] = bounds[3].max((e.origin[1] + e.pixels.height() as f32).ceil() as i64);
        }
        bounds
    }
    /// Conservative cache estimate, including retained source and coverage planes.
    /// Shared masks/tiles are deliberately counted repeatedly.
    pub fn byte_len(&self) -> usize {
        self.original
            .filters
            .iter()
            .map(|f| f.filter.asset_bytes())
            .sum::<usize>()
            + self.original.source.byte_len()
            + self.original.mask.as_ref().map_or(0, |m| m.values.len())
            + self
                .original
                .filters
                .iter()
                .filter_map(|f| f.mask.as_ref())
                .map(|m| m.values.len())
                .sum::<usize>()
            + self.pixels.as_ref().map_or(0, Raster::byte_len)
            + self
                .styles
                .iter()
                .map(|e| e.pixels.byte_len())
                .sum::<usize>()
    }
    /// Structural/source equality invalidates the cache after any layer edit.
    pub fn matches(&self, layer: &Layer) -> bool {
        self.original == *layer
    }
}
/// Whether evaluating this layer requires neighborhood/padded image stages.
pub fn has_neighborhood(layer: &Layer) -> bool {
    layer.filters.iter().any(|f| {
        f.enabled
            && match f.filter {
                Filter::ColorTransfer { strength, .. } => strength > 0.,
                Filter::DepthBlur {
                    radius, show_depth, ..
                } => radius > 0. || show_depth,
                Filter::SuperZoom { .. } => true,
                Filter::SkinSmoothing { strength, .. }
                | Filter::Colorize { strength, .. }
                | Filter::StyleTransfer { strength, .. }
                | Filter::JpegArtifactsRemoval { strength, .. } => strength > 0.,
                Filter::FieldBlur { .. }
                | Filter::IrisBlur { .. }
                | Filter::TiltShift { .. }
                | Filter::PathBlur { .. }
                | Filter::SpinBlur { .. }
                | Filter::Flame { .. }
                | Filter::PictureFrame { .. }
                | Filter::Tree { .. }
                | Filter::Deinterlace { .. }
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
                | Filter::Watercolor { .. }
                | Filter::WaterPaper { .. } => true,
                Filter::SprayedStrokes { .. }
                | Filter::SumiE { .. }
                | Filter::BasRelief { .. }
                | Filter::ChalkCharcoal { .. }
                | Filter::Charcoal { .. }
                | Filter::GraphicPen { .. }
                | Filter::NotePaper { .. }
                | Filter::Stamp { .. } => true,
                Filter::ColoredPencil { .. }
                | Filter::DryBrush { .. }
                | Filter::PaintDaubs { .. }
                | Filter::SmudgeStick { .. }
                | Filter::AngledStrokes { .. }
                | Filter::Crosshatch { .. }
                | Filter::DarkStrokes { .. }
                | Filter::Spatter { .. } => true,
                Filter::ShadowsHighlights {
                    shadows,
                    highlights,
                    ..
                } => shadows > 0. || highlights > 0.,
                Filter::Blur | Filter::BlurMore => true,
                Filter::ColorHalftone { .. }
                | Filter::Crystallize { .. }
                | Filter::Facet
                | Filter::Pointillize { .. }
                | Filter::StainedGlass { .. }
                | Filter::MosaicTiles { .. }
                | Filter::Diffuse { .. } => true,
                Filter::Cutout { .. }
                | Filter::AccentedEdges { .. }
                | Filter::InkOutlines { .. }
                | Filter::Photocopy { .. }
                | Filter::HalftonePattern { .. }
                | Filter::Patchwork { .. } => true,
                Filter::PolarCoordinates { .. }
                | Filter::ZigZag { .. }
                | Filter::Fibers { .. }
                | Filter::GlowingEdges { .. } => true,
                Filter::Despeckle | Filter::Sharpen | Filter::SharpenMore => true,
                Filter::DustScratches { radius, threshold } => radius > 0 && threshold < 255,
                Filter::ReduceNoise {
                    radius,
                    strength,
                    color_noise,
                    sharpen,
                    ..
                } => radius > 0 && (strength > 0. || color_noise > 0. || sharpen > 0.),
                Filter::Pinch { amount, radius, .. } | Filter::Spherize { amount, radius, .. } => {
                    amount != 0. && radius > 0.
                }
                Filter::Ripple { amplitude, .. } => amplitude != 0.,
                Filter::Clouds { .. }
                | Filter::DifferenceClouds { .. }
                | Filter::PosterEdges { .. } => true,
                Filter::Fragment { distance } => distance > 0.,
                Filter::SurfaceBlur { radius, threshold } => radius > 0 && threshold > 0,
                Filter::RadialBlur { amount, .. } => amount > 0.,
                Filter::GaussianBlur { radius } => radius > 0.,
                Filter::BoxBlur { radius } | Filter::Median { radius } => radius > 0,
                Filter::MotionBlur { distance, .. } => distance > 0.,
                Filter::HighPass { .. }
                | Filter::Average
                | Filter::FindEdges
                | Filter::Emboss { .. } => true,
                Filter::Minimum { radius } | Filter::Maximum { radius } => radius > 0,
                Filter::UnsharpMask { radius, amount, .. } => radius > 0. && amount > 0.,
                Filter::Offset { x, y, .. } => x != 0 || y != 0,
                Filter::Twirl { radius, angle, .. } => radius > 0. && angle != 0.,
                Filter::Wave { amplitude, .. } => amplitude != 0.,
                Filter::Mosaic { block_size } => block_size > 1,
                _ => false,
            }
    }) || layer
        .effects
        .iter()
        .any(|e| e.enabled && !matches!(e.effect, Effect::ColorOverlay { .. }))
}
/// Evaluate a validated layer once for bulk sampling without changing its source.
pub fn prepare_layer(layer: &Layer) -> Result<PreparedLayer, String> {
    prepare_layer_cancel(layer, &|| false)
}
/// As `prepare_layer`, with cancellation between processing rows and stages.
pub fn prepare_layer_cancel(
    layer: &Layer,
    cancel: &dyn Fn() -> bool,
) -> Result<PreparedLayer, String> {
    if !has_neighborhood(layer) {
        return Ok(PreparedLayer {
            original: layer.clone(),
            pixels: None,
            styles: vec![],
        });
    }
    let mut pixels = layer.source.clone();
    for filter in layer.filters.iter().filter(|f| f.enabled) {
        if cancel() {
            return Err("Processing cancelled".into());
        }
        let before = pixels.clone();
        match filter.filter {
            Filter::ColorTransfer { .. } => {
                pixels = super::color_transfer::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::DepthBlur { .. } => {
                pixels = super::depth::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::SuperZoom {
                model,
                zoom,
                enhancement,
                center,
            } => {
                pixels = super::zoom::evaluate(&pixels, model, zoom, enhancement, center, cancel)?;
            }
            Filter::SkinSmoothing {
                model,
                strength,
                blur,
                range,
            } => {
                pixels = super::skin::evaluate(&pixels, model, strength, blur, range, cancel)?;
            }
            Filter::Colorize {
                model,
                strength,
                saturation,
            } => {
                pixels = super::colorize::evaluate(&pixels, model, strength, saturation, cancel)?;
            }
            Filter::JpegArtifactsRemoval { model, strength } => {
                pixels = super::restoration::evaluate(&pixels, model, strength, cancel)?;
            }
            Filter::StyleTransfer { style, strength } => {
                pixels = super::neural::style_transfer(&pixels, style, strength, cancel)?;
            }
            Filter::FieldBlur { .. }
            | Filter::IrisBlur { .. }
            | Filter::TiltShift { .. }
            | Filter::PathBlur { .. }
            | Filter::SpinBlur { .. } => {
                pixels = super::gallery::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Flame { .. } | Filter::PictureFrame { .. } | Filter::Tree { .. } => {
                pixels = super::generators::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Deinterlace { .. } => {
                pixels = super::video::deinterlace(&pixels, &filter.filter, cancel)?
            }
            Filter::Displace { .. } | Filter::Shear { .. } | Filter::Custom { .. } => {
                pixels = super::editable::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::LensBlur { .. } | Filter::LensFlare { .. } | Filter::LightingEffects { .. } => {
                pixels = super::cinematic::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Extrude { .. }
            | Filter::OilPaint { .. }
            | Filter::Tiles { .. }
            | Filter::TraceContour { .. }
            | Filter::Wind { .. } => {
                pixels = super::stylize::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::DiffuseGlow { .. }
            | Filter::Glass { .. }
            | Filter::OceanRipple { .. }
            | Filter::Texturizer { .. }
            | Filter::ShapeBlur { .. }
            | Filter::SmartBlur { .. }
            | Filter::SharpenEdges
            | Filter::SmartSharpen { .. } => {
                pixels = super::optical::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Fresco { .. }
            | Filter::NeonGlow { .. }
            | Filter::PaletteKnife { .. }
            | Filter::PlasticWrap { .. }
            | Filter::RoughPastels { .. }
            | Filter::Sponge { .. }
            | Filter::Underpainting { .. }
            | Filter::Watercolor { .. } => {
                pixels = super::media::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Chrome { .. }
            | Filter::ConteCrayon { .. }
            | Filter::Plaster { .. }
            | Filter::Reticulation { .. }
            | Filter::TornEdges { .. }
            | Filter::WaterPaper { .. } => {
                pixels = super::sketch::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::SprayedStrokes { .. }
            | Filter::SumiE { .. }
            | Filter::BasRelief { .. }
            | Filter::ChalkCharcoal { .. }
            | Filter::Charcoal { .. }
            | Filter::GraphicPen { .. }
            | Filter::NotePaper { .. }
            | Filter::Stamp { .. } => {
                pixels = super::drawing::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::ColoredPencil { .. }
            | Filter::DryBrush { .. }
            | Filter::PaintDaubs { .. }
            | Filter::SmudgeStick { .. }
            | Filter::AngledStrokes { .. }
            | Filter::Crosshatch { .. }
            | Filter::DarkStrokes { .. }
            | Filter::Spatter { .. } => {
                pixels = super::painting::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::ShadowsHighlights { .. } => {
                pixels = super::adjustments::shadows_highlights(&pixels, &filter.filter, cancel)?
            }
            Filter::Blur => pixels = box_blur_cancel(&pixels, 1, cancel)?,
            Filter::BlurMore => pixels = box_blur_cancel(&pixels, 2, cancel)?,

            Filter::Crystallize { .. }
            | Filter::Pointillize { .. }
            | Filter::StainedGlass { .. } => {
                pixels = super::cellular::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::ColorHalftone { .. }
            | Filter::Facet
            | Filter::MosaicTiles { .. }
            | Filter::Diffuse { .. } => {
                pixels = super::pixelate::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Cutout { .. } => {
                pixels = super::cutout::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::AccentedEdges { .. }
            | Filter::InkOutlines { .. }
            | Filter::Photocopy { .. }
            | Filter::HalftonePattern { .. }
            | Filter::Patchwork { .. } => {
                pixels = super::artistic::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::PolarCoordinates { .. }
            | Filter::ZigZag { .. }
            | Filter::Fibers { .. }
            | Filter::GlowingEdges { .. } => {
                pixels = super::studio::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Despeckle
            | Filter::DustScratches { .. }
            | Filter::ReduceNoise { .. }
            | Filter::Sharpen
            | Filter::SharpenMore => {
                pixels = super::retouch::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Pinch { .. } | Filter::Spherize { .. } | Filter::Ripple { .. } => {
                pixels = super::distort::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Clouds { .. }
            | Filter::DifferenceClouds { .. }
            | Filter::SurfaceBlur { .. }
            | Filter::RadialBlur { .. }
            | Filter::PosterEdges { .. }
            | Filter::Fragment { .. } => {
                pixels = super::creative::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::Average
            | Filter::FindEdges
            | Filter::Emboss { .. }
            | Filter::UnsharpMask { .. }
            | Filter::Minimum { .. }
            | Filter::Maximum { .. }
            | Filter::Offset { .. }
            | Filter::Twirl { .. }
            | Filter::Wave { .. } => {
                pixels = super::conventional::evaluate(&pixels, &filter.filter, cancel)?
            }
            Filter::GaussianBlur { radius } => {
                pixels = gaussian_blur_cancel(&pixels, radius, cancel)?
            }
            Filter::MotionBlur { distance, angle } => {
                pixels = motion_blur_cancel(&pixels, distance, angle, cancel)?
            }
            Filter::HighPass { radius } => pixels = high_pass_cancel(&pixels, radius, cancel)?,
            Filter::Mosaic { block_size } => pixels = mosaic_cancel(&pixels, block_size, cancel)?,
            Filter::BoxBlur { radius } => pixels = box_blur_cancel(&pixels, radius, cancel)?,
            Filter::Median { radius } => pixels = median_cancel(&pixels, radius, cancel)?,
            _ => {
                for y in 0..pixels.height() {
                    if cancel() {
                        return Err("Processing cancelled".into());
                    }
                    for x in 0..pixels.width() {
                        pixels.set(x, y, point_filter(&filter.filter, pixels.get(x, y), x, y));
                    }
                }
            }
        }

        if let Some(mask) = &filter.mask {
            for y in 0..pixels.height() {
                if cancel() {
                    return Err("Processing cancelled".into());
                }
                for x in 0..pixels.width() {
                    pixels.set(
                        x,
                        y,
                        crate::editing::mix(before.get(x, y), pixels.get(x, y), mask.at(x, y)),
                    );
                }
            }
        }
    }
    for y in 0..pixels.height() {
        if cancel() {
            return Err("Processing cancelled".into());
        }
        for x in 0..pixels.width() {
            let mut p = pixels.get(x, y);
            for e in layer.effects.iter().filter(|e| e.enabled) {
                if let Effect::ColorOverlay { color, opacity } = e.effect {
                    for c in 0..3 {
                        p[c] = (f32::from(p[c]) * (1. - opacity) + f32::from(color[c]) * opacity)
                            .round() as u8;
                    }
                }
            }
            p[3] = (f32::from(p[3]) * layer.mask.as_ref().map_or(1., |m| m.at(x, y))).round() as u8;
            pixels.set(x, y, p);
        }
    }
    let mut styles = vec![];
    for effect in layer.effects.iter().filter(|e| e.enabled) {
        if let Effect::Stroke {
            color,
            opacity,
            width,
            position,
            rounded,
        } = effect.effect
        {
            styles.extend(stroke(
                &pixels,
                color,
                opacity,
                width,
                (position, rounded),
                cancel,
            )?);
            continue;
        }
        let (color, opacity, spread, size, shift, outer, inner) = match effect.effect {
            Effect::ColorOverlay { .. } | Effect::Stroke { .. } => continue,
            Effect::DropShadow {
                color,
                opacity,
                angle,
                distance,
                spread,
                size,
            }
            | Effect::InnerShadow {
                color,
                opacity,
                angle,
                distance,
                spread,
                size,
            } => {
                let a = angle.to_radians();
                (
                    color,
                    opacity,
                    spread,
                    size,
                    [-a.cos() * distance, a.sin() * distance],
                    false,
                    matches!(effect.effect, Effect::InnerShadow { .. }),
                )
            }
            Effect::OuterGlow {
                color,
                opacity,
                spread,
                size,
            }
            | Effect::InnerGlow {
                color,
                opacity,
                spread,
                size,
            } => (
                color,
                opacity,
                spread,
                size,
                [0.; 2],
                true,
                matches!(effect.effect, Effect::InnerGlow { .. }),
            ),
        };
        let pad = effect_padding(&effect.effect).expect("underlay padding");
        let (w, h) = (pixels.width() + 2 * pad, pixels.height() + 2 * pad);
        let mut alpha = vec![0; (u64::from(w) * u64::from(h)) as usize];
        for y in 0..pixels.height() {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            for x in 0..pixels.width() {
                alpha[((y + pad) * w + x + pad) as usize] = pixels.get(x, y)[3];
            }
        }
        let r = (size * spread).ceil() as usize;
        if r > 0 {
            if inner {
                for a in &mut alpha {
                    *a = 255 - *a;
                }
            }
            alpha = dilate(&alpha, w as usize, h as usize, r, cancel)?;
            if inner {
                for a in &mut alpha {
                    *a = 255 - *a;
                }
            }
        }
        let mut silhouette = Raster::new(w, h, [0; 4])?;
        for y in 0..h {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            for x in 0..w {
                let a = alpha[(y * w + x) as usize];
                if a > 0 {
                    silhouette.set(x, y, [255, 255, 255, a]);
                }
            }
        }
        drop(alpha);
        let blurred = gaussian_blur_cancel(&silhouette, size * (1. - spread), cancel)?;
        styles.push(Style {
            pixels: blurred,
            origin: [shift[0] - pad as f32, shift[1] - pad as f32],
            color,
            opacity,
            outer,
            inner,
            inverse: inner,
        });
    }
    Ok(PreparedLayer {
        original: layer.clone(),
        pixels: Some(pixels),
        styles,
    })
}
/// Fractional box or coverage-distance morphology supports subpixel widths. Erosion
/// uses the complement dual so internal holes and canvas edges behave equally.
fn stroke(
    pixels: &Raster,
    color: [u8; 3],
    opacity: f32,
    width: f32,
    placement: (StrokePosition, bool),
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<Style>, String> {
    let (position, rounded) = placement;
    if width == 0. || opacity == 0. {
        return Ok(vec![]);
    }
    let pad = width.ceil() as u32 + 2;
    let (w, h) = (pixels.width() + 2 * pad, pixels.height() + 2 * pad);
    let mut alpha = vec![0; (u64::from(w) * u64::from(h)) as usize];
    for y in 0..pixels.height() {
        if cancel() {
            return Err("Processing cancelled".into());
        }
        for x in 0..pixels.width() {
            alpha[((y + pad) * w + x + pad) as usize] = pixels.get(x, y)[3];
        }
    }
    let radius = if position == StrokePosition::Center {
        width * 0.5
    } else {
        width
    };
    let mut styles = vec![];
    for inner in [false, true] {
        if (inner && position == StrokePosition::Outside)
            || (!inner && position == StrokePosition::Inside)
        {
            continue;
        }
        let src: Vec<_> = alpha
            .iter()
            .map(|a| if inner { 255 - *a } else { *a })
            .collect();
        let coverage = if rounded {
            super::round::grow(&src, w as usize, h as usize, radius, cancel)?
        } else {
            let low = dilate(
                &src,
                w as usize,
                h as usize,
                radius.floor() as usize,
                cancel,
            )?;
            let fraction = radius.fract();
            if fraction == 0. {
                low
            } else {
                let high = dilate(&src, w as usize, h as usize, radius.ceil() as usize, cancel)?;
                low.iter()
                    .zip(high)
                    .map(|(a, b)| {
                        (f32::from(*a) * (1. - fraction) + f32::from(b) * fraction).round() as u8
                    })
                    .collect()
            }
        };
        let mut result = Raster::new(w, h, [0; 4])?;
        for y in 0..h {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            for x in 0..w {
                let i = (y * w + x) as usize;
                let a = f32::from(coverage[i]);
                if a > 0. {
                    result.set(x, y, [255, 255, 255, a.round() as u8]);
                }
            }
        }
        styles.push(Style {
            pixels: result,
            origin: [-(pad as f32); 2],
            color,
            opacity,
            outer: !inner,
            inner,
            inverse: false,
        });
    }
    Ok(styles)
}
fn dilate(
    src: &[u8],
    w: usize,
    h: usize,
    r: usize,
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, String> {
    let mut a = src.to_vec();
    let mut b = vec![0; src.len()];
    for vertical in [false, true] {
        let (lines, len) = if vertical { (w, h) } else { (h, w) };
        for line in 0..lines {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            let index = |p: usize| if vertical { p * w + line } else { line * w + p };
            let mut queue: VecDeque<usize> = VecDeque::new();
            let mut next = 0;
            for p in 0..len {
                while next <= (p + r).min(len - 1) {
                    while queue.back().is_some_and(|i| a[index(*i)] <= a[index(next)]) {
                        queue.pop_back();
                    }
                    queue.push_back(next);
                    next += 1;
                }
                while queue.front().is_some_and(|i| *i < p.saturating_sub(r)) {
                    queue.pop_front();
                }
                b[index(p)] = a[index(*queue.front().expect("nonempty maximum window"))];
            }
        }
        std::mem::swap(&mut a, &mut b);
    }
    Ok(a)
}
