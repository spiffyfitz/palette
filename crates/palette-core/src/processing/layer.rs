//! Ordered filters -> color overlays -> mask -> live inner/outer styles.
use super::*;
use crate::{
    composite::over,
    document::{BlendMode, Layer, StrokePosition},
    raster::Raster,
};
use std::collections::VecDeque;

pub(super) struct Style {
    pub(super) blend: BlendMode,
    pub(super) colored: bool,
    pub(super) pixels: Raster,
    pub(super) origin: [f32; 2],
    pub(super) color: [u8; 3],
    pub(super) opacity: f32,
    pub(super) outer: bool,
    pub(super) inner: bool,
    pub(super) inverse: bool,
}
impl Style {
    fn sample(&self, x: i64, y: i64) -> ([u8; 3], f32) {
        let px = x as f32 - self.origin[0];
        let py = y as f32 - self.origin[1];
        let (ix, iy) = (px.floor() as i64, py.floor() as i64);
        let (fx, fy) = (px - px.floor(), py - py.floor());
        let mut a = 0.;
        let mut rgb = [0.; 3];
        for (dx, dy, w) in [
            (0, 0, (1. - fx) * (1. - fy)),
            (1, 0, fx * (1. - fy)),
            (0, 1, (1. - fx) * fy),
            (1, 1, fx * fy),
        ] {
            let (x, y) = (ix + dx, iy + dy);
            if x >= 0
                && y >= 0
                && x < i64::from(self.pixels.width())
                && y < i64::from(self.pixels.height())
            {
                let p = self.pixels.get(x as u32, y as u32);
                let coverage = f32::from(p[3]) * w;
                a += coverage;
                if self.colored {
                    for c in 0..3 {
                        rgb[c] += f32::from(p[c]) * coverage;
                    }
                }
            }
        }
        let color = if self.colored && a > 0. {
            rgb.map(|c| (c / a).round() as u8)
        } else {
            self.color
        };
        (color, if self.inverse { 255. - a } else { a })
    }
    fn tint(&self, mut front: [u8; 4], color: [u8; 3], a: f32) -> [u8; 4] {
        let amount = a / 255. * self.opacity;
        if self.blend == BlendMode::Normal {
            for c in 0..3 {
                front[c] = (f32::from(front[c]) * (1. - amount) + f32::from(color[c]) * amount)
                    .round() as u8;
            }
        } else {
            let out = over(
                [front[0], front[1], front[2], 255],
                [color[0], color[1], color[2], (amount * 255.).round() as u8],
                1.,
                self.blend,
            );
            front[..3].copy_from_slice(&out[..3]);
        }
        front
    }
}
/// Immutable evaluated appearance, including spatial style extents.
pub struct PreparedLayer {
    visible: std::sync::OnceLock<Option<[i64; 4]>>,
    pub(super) original: Layer,
    pixels: Option<Raster>,
    styles: Vec<Style>,
}
/// Device-independent RGBA planes for a cached renderer. The first plane is the
/// isolated appearance. Complex exterior blending also retains inner-styled
/// content (second plane) and ordered exterior RGBA planes after it.
pub struct Appearance {
    /// Source-space upper-left pixel, including exterior styles.
    pub origin: [i32; 2],
    /// Tightly packed plane width.
    pub width: u32,
    /// Tightly packed plane height.
    pub height: u32,
    /// Straight-alpha sRGB RGBA8; all planes have identical dimensions.
    pub images: Vec<Vec<u8>>,
    /// Blend modes for exterior planes, starting at image index two.
    pub modes: Vec<BlendMode>,
}
impl Appearance {
    /// Retained packed pixel bytes, excluding small descriptors.
    pub fn byte_len(&self) -> usize {
        self.images.iter().map(Vec::len).sum()
    }
}
impl PreparedLayer {
    /// Resolved source inputs, without retaining derived CPU style/blur outputs.
    pub fn cache_key(&self) -> &Layer {
        &self.original
    }
    /// Compare already-resolved source appearances across independent caches.
    pub fn same_as(&self, other: &Self) -> bool {
        self.matches(&other.original)
    }

    /// Evaluate upload planes once; translation and layer blending do not belong
    /// in this cache. Limits are checked before allocating any packed buffers.
    pub fn appearance(
        &self,
        max_side: u32,
        max_bytes: usize,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Appearance, String> {
        let b = self.visible_bounds(cancel)?.unwrap_or([0, 0, 1, 1]);
        let (width, height) = ((b[2] - b[0]) as u32, (b[3] - b[1]) as u32);
        let complex = self
            .styles
            .iter()
            .any(|s| !s.inner && s.blend != BlendMode::Normal);
        let exterior: Vec<_> = self.styles.iter().filter(|s| !s.inner).collect();
        let count = if complex { 2 + exterior.len() } else { 1 };
        let bytes = u64::from(width) * u64::from(height) * 4;
        if width > max_side
            || height > max_side
            || bytes.saturating_mul(count as u64) > max_bytes as u64
        {
            return Err("Layer appearance exceeds GPU texture/cache limits".into());
        }
        let mut images: Vec<_> = (0..count)
            .map(|_| Vec::with_capacity(bytes as usize))
            .collect();
        for y in b[1]..b[3] {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            for x in b[0]..b[2] {
                images[0].extend_from_slice(&self.pixel(x, y));
                if complex {
                    let mut front = self.content(x, y);
                    for s in self.styles.iter().filter(|s| s.inner) {
                        let (color, a) = s.sample(x, y);
                        front = s.tint(front, color, a);
                    }
                    images[1].extend_from_slice(&front);
                    for (i, s) in exterior.iter().enumerate() {
                        let (color, a) = s.sample(x, y);
                        let a = a
                            * s.opacity
                            * if s.outer {
                                1. - f32::from(front[3]) / 255.
                            } else {
                                1.
                            };
                        images[2 + i].extend_from_slice(&[
                            color[0],
                            color[1],
                            color[2],
                            a.round() as u8,
                        ]);
                    }
                }
            }
        }
        Ok(Appearance {
            origin: [b[0] as i32, b[1] as i32],
            width,
            height,
            images,
            modes: if complex {
                exterior.iter().map(|s| s.blend).collect()
            } else {
                vec![]
            },
        })
    }

    /// Signed source-space lookup includes styles outside the source bounds.
    pub fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        let mut front = self.content(x, y);
        // Isolated filtered pixels retain their invisible RGB. Only the final
        // document composite canonicalizes zero alpha; styles need source-over.
        if self.styles.is_empty() {
            return front;
        }
        let mut under = [0; 4];
        for effect in &self.styles {
            let (color, a) = effect.sample(x, y);
            if effect.inner {
                front = effect.tint(front, color, a);
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
                [color[0], color[1], color[2], alpha.round() as u8],
                1.,
                effect.blend,
            );
        }
        over(under, front, 1., BlendMode::Normal)
    }
    fn content(&self, x: i64, y: i64) -> [u8; 4] {
        if x < 0
            || y < 0
            || x >= i64::from(self.original.source.width())
            || y >= i64::from(self.original.source.height())
        {
            [0; 4]
        } else if let Some(p) = &self.pixels {
            p.get(x as u32, y as u32)
        } else {
            crate::composite::point_pixel(&self.original, x as u32, y as u32)
        }
    }
    /// Evaluate exterior style blend modes against the actual document backdrop.
    /// Recover a straight-alpha group color before applying layer opacity/mode;
    /// this preserves group opacity instead of independently fading each plane.
    pub fn composite(
        &self,
        back: [u8; 4],
        x: i64,
        y: i64,
        opacity: f32,
        mode: BlendMode,
    ) -> [u8; 4] {
        let isolated = self.pixel(x, y);
        if !self
            .styles
            .iter()
            .any(|s| !s.inner && s.blend != BlendMode::Normal)
        {
            return over(back, isolated, opacity, mode);
        }
        let mut front = self.content(x, y);
        let mut under = back;
        for effect in &self.styles {
            let (color, a) = effect.sample(x, y);
            if effect.inner {
                front = effect.tint(front, color, a);
                continue;
            }
            let a = a
                * effect.opacity
                * if effect.outer {
                    1. - f32::from(front[3]) / 255.
                } else {
                    1.
                };
            under = over(
                under,
                [color[0], color[1], color[2], a.round() as u8],
                1.,
                effect.blend,
            );
        }
        let desired = over(under, front, 1., BlendMode::Normal);
        let (a, ab, ao) = (
            f32::from(isolated[3]) / 255.,
            f32::from(back[3]) / 255.,
            f32::from(desired[3]) / 255.,
        );
        if a == 0. {
            return back;
        }
        let mut group = isolated;
        for c in 0..3 {
            group[c] = ((f32::from(desired[c]) * ao - f32::from(back[c]) * ab * (1. - a)) / a)
                .round()
                .clamp(0., 255.) as u8;
        }
        over(back, group, opacity, mode)
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
        input_byte_len(&self.original)
            + self.pixels.as_ref().map_or(0, Raster::byte_len)
            + self
                .styles
                .iter()
                .map(|e| e.pixels.byte_len())
                .sum::<usize>()
    }
    /// Appearance equality excludes placement and presentation-only properties.
    /// World-anchored materials have their resolved phase in the effect options.
    pub fn matches(&self, layer: &Layer) -> bool {
        same_appearance(&self.original, layer)
    }
    /// Conservative nontransparent source-space bounds, including outer styles.
    /// Scan coverage once per prepared appearance, rather than treating a small
    /// drawing on a canvas-sized transparent raster as a full-canvas dirty area.
    pub fn visible_bounds(&self, cancel: &dyn Fn() -> bool) -> Result<Option<[i64; 4]>, String> {
        if cancel() {
            return Err("Processing cancelled".into());
        }
        if let Some(bounds) = self.visible.get() {
            return Ok(*bounds);
        }
        let mut bounds: Option<[i64; 4]> = None;
        let mut include = |b: [i64; 4]| {
            bounds = Some(bounds.map_or(b, |a| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            }));
        };
        for y in 0..self.original.source.height() {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            for x in 0..self.original.source.width() {
                if self.content(x.into(), y.into())[3] != 0 {
                    include([x.into(), y.into(), i64::from(x) + 1, i64::from(y) + 1]);
                }
            }
        }
        for s in self.styles.iter().filter(|s| !s.inner && s.opacity > 0.) {
            for y in 0..s.pixels.height() {
                if cancel() {
                    return Err("Processing cancelled".into());
                }
                for x in 0..s.pixels.width() {
                    let a = s.pixels.get(x, y)[3];
                    if (if s.inverse { 255 - a } else { a }) != 0 {
                        include([
                            (s.origin[0] + x as f32).floor() as i64,
                            (s.origin[1] + y as f32).floor() as i64,
                            (s.origin[0] + x as f32 + 1.).ceil() as i64,
                            (s.origin[1] + y as f32 + 1.).ceil() as i64,
                        ]);
                    }
                }
            }
        }
        let _ = self.visible.set(bounds);
        Ok(bounds)
    }
}
/// Compare source-space appearance without position, opacity or UI metadata.
/// Callers resolving document-dependent styles must compare resolved options.
pub fn same_appearance(a: &Layer, b: &Layer) -> bool {
    a.id == b.id
        && a.source == b.source
        && a.mask == b.mask
        && a.filters == b.filters
        && a.effects == b.effects
}
/// Conservative retained source-input bytes; shared buffers count repeatedly.
/// Derived images are excluded so independent caches can budget their keys.
pub fn input_byte_len(layer: &Layer) -> usize {
    layer.source.byte_len()
        + layer.mask.as_ref().map_or(0, |m| m.values.len())
        + layer
            .filters
            .iter()
            .map(|f| f.filter.asset_bytes() + f.mask.as_ref().map_or(0, |m| m.values.len()))
            .sum::<usize>()
        + layer
            .effects
            .iter()
            .filter_map(|e| e.effect.options().and_then(|o| o.pattern.as_ref()))
            .map(|p| p.pixels.byte_len())
            .sum::<usize>()
}
/// Whether evaluating this layer requires neighborhood/padded image stages.
pub fn has_neighborhood(layer: &Layer) -> bool {
    layer.filters.iter().any(|f| {
        f.enabled
            && match f.filter {
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
    for instance in layer
        .effects
        .iter()
        .filter(|e| e.enabled && matches!(e.effect, Effect::Configured { .. }))
    {
        Operation::Effect(instance.effect.clone()).validate()?;
    }
    if !has_neighborhood(layer) {
        return Ok(PreparedLayer {
            visible: std::sync::OnceLock::new(),
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
                if let Some(result) = super::style_options::overlay(
                    p,
                    &e.effect,
                    x,
                    y,
                    pixels.width(),
                    pixels.height(),
                ) {
                    p = result;
                    continue;
                }
                if let Some(result) =
                    super::styles::overlay(p, &e.effect, x, y, pixels.width(), pixels.height())
                {
                    p = result;
                }
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
        if let Some(planes) = super::styles::planes(&pixels, &effect.effect, cancel)? {
            let start = styles.len();
            styles.extend(planes);
            super::style_options::finish(&mut styles[start..], &effect.effect, &pixels, cancel)?;
            continue;
        }
        if let Effect::Stroke {
            color,
            opacity,
            width,
            position,
            rounded,
        } = *effect.effect.base()
        {
            let start = styles.len();
            styles.extend(stroke(
                &pixels,
                color,
                opacity,
                width,
                (position, rounded),
                cancel,
            )?);
            super::style_options::finish(&mut styles[start..], &effect.effect, &pixels, cancel)?;
            continue;
        }
        let (color, opacity, spread, size, shift, outer, inner) = match *effect.effect.base() {
            Effect::Configured { .. } => unreachable!("validated base effect"),
            Effect::ColorOverlay { .. }
            | Effect::Stroke { .. }
            | Effect::BevelEmboss { .. }
            | Effect::Satin { .. }
            | Effect::GradientOverlay { .. }
            | Effect::PatternOverlay { .. } => continue,
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
                    matches!(effect.effect.base(), Effect::InnerShadow { .. }),
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
                matches!(effect.effect.base(), Effect::InnerGlow { .. }),
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
            alpha = if effect.effect.options().is_some_and(|o| o.technique == 1)
                && matches!(
                    effect.effect.base(),
                    Effect::OuterGlow { .. } | Effect::InnerGlow { .. }
                ) {
                super::round::grow(&alpha, w as usize, h as usize, size * spread, cancel)?
            } else {
                dilate(&alpha, w as usize, h as usize, r, cancel)?
            };
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
        let precise = effect.effect.options().is_some_and(|o| o.technique == 1)
            && matches!(
                effect.effect.base(),
                Effect::OuterGlow { .. } | Effect::InnerGlow { .. }
            );
        let blurred = gaussian_blur_cancel(
            &silhouette,
            size * (1. - spread) * if precise { 0.5 } else { 1. },
            cancel,
        )?;
        styles.push(Style {
            pixels: blurred,
            origin: [shift[0] - pad as f32, shift[1] - pad as f32],
            color,
            opacity,
            outer,
            inner,
            inverse: inner,
            blend: BlendMode::Normal,
            colored: false,
        });
        let count = styles.len();
        super::style_options::finish(&mut styles[count - 1..], &effect.effect, &pixels, cancel)?;
    }
    Ok(PreparedLayer {
        visible: std::sync::OnceLock::new(),
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
            blend: BlendMode::Normal,
            colored: false,
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
