//! Deterministic reference CPU compositor. Working RGB is sRGB (not linear light).
mod blend;
use crate::document::{BlendMode, Document, Effect, Filter, Layer};
use crate::editing::PixelRect;
use crate::raster::Raster;

/// W3C source-over blending with straight input/output alpha. Transparent RGB
/// never contributes; RGB is premultiplied only for the arithmetic below.
pub fn over(back: [u8; 4], front: [u8; 4], opacity: f32, mode: BlendMode) -> [u8; 4] {
    let ab = f32::from(back[3]) / 255.0;
    let af = f32::from(front[3]) / 255.0 * opacity.clamp(0.0, 1.0);
    let a = af + ab * (1.0 - af);
    if a <= 0.0 {
        return [0; 4];
    }
    let mut out = [0; 4];
    let mixed = blend::rgb(
        mode,
        std::array::from_fn(|c| f32::from(back[c]) / 255.),
        std::array::from_fn(|c| f32::from(front[c]) / 255.),
    );
    for c in 0..3 {
        let b = f32::from(back[c]) / 255.0;
        let f = f32::from(front[c]) / 255.0;
        let blend = mixed[c];
        out[c] = (((1.0 - af) * ab * b + (1.0 - ab) * af * f + ab * af * blend) / a * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    out[3] = (a * 255.0).round() as u8;
    out
}

/// Evaluate one pixel without modifying source data; layer opacity is separate.
/// Bulk spatial callers must reuse `processing::prepare_layer` instead: this
/// compatibility helper evaluates a complete spatial layer for every call.
pub fn layer_pixel(layer: &Layer, x: u32, y: u32) -> [u8; 4] {
    if crate::processing::has_neighborhood(layer) {
        return crate::processing::prepare_layer(layer)
            .expect("validated layer")
            .pixel(i64::from(x), i64::from(y));
    }
    point_pixel(layer, x, y)
}
pub(crate) fn point_pixel(layer: &Layer, x: u32, y: u32) -> [u8; 4] {
    let mut pixel = layer.source.get(x, y);
    for instance in layer.filters.iter().filter(|f| f.enabled) {
        let original = pixel;
        for c in 0..3 {
            pixel[c] = match instance.filter {
                Filter::Invert => 255 - original[c],
                Filter::Brightness { amount } => (f32::from(original[c]) + amount * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8,
                Filter::GaussianBlur { .. } => original[c],
            };
        }
        let coverage = instance.mask.as_ref().map_or(1.0, |m| m.at(x, y));
        for c in 0..3 {
            pixel[c] = (f32::from(original[c]) * (1.0 - coverage) + f32::from(pixel[c]) * coverage)
                .round() as u8;
        }
    }
    for instance in layer.effects.iter().filter(|e| e.enabled) {
        let Effect::ColorOverlay { color, opacity } = instance.effect else {
            continue;
        };
        for c in 0..3 {
            pixel[c] = (f32::from(pixel[c]) * (1.0 - opacity) + f32::from(color[c]) * opacity)
                .round() as u8;
        }
    }
    pixel[3] =
        (f32::from(pixel[3]) * layer.mask.as_ref().map_or(1.0, |m| m.at(x, y))).round() as u8;
    pixel
}

/// Render validated layers bottom-to-top to straight-alpha RGBA8 on the CPU.
pub fn render(document: &Document) -> Raster {
    Renderer::default()
        .render(document)
        .expect("validated document")
}

/// Evaluate a canvas region with exactly the full compositor's layer order,
/// masks and alpha arithmetic. Spatial layers are prepared with their full
/// neighborhoods before cropping, so region edges cannot introduce seams.
pub fn render_region(document: &Document, region: PixelRect) -> Result<Raster, String> {
    Renderer::default().region(document, region, &|| false)
}

/// Bounded LRU of evaluated spatial layers; unchanged layers survive unrelated
/// document edits and viewport repaints. Source snapshots are included in budget.
#[derive(Default)]
pub struct Renderer {
    cache: std::collections::VecDeque<std::sync::Arc<crate::processing::PreparedLayer>>,
    bytes: usize,
    /// Number of retained spatial-layer evaluations reused by this renderer.
    pub cache_hits: u64,
    /// Number of complete spatial-layer preparations, including uncached outputs.
    pub evaluations: u64,
}
/// Canvas plus evaluated spatial-layer thumbnails from the same prepared data.
pub struct RenderFrame {
    pub pixels: Raster,
    pub thumbnails: Vec<(u64, Raster)>,
}
impl Renderer {
    /// Render a canvas and small spatial-layer thumbnails in the same job.
    pub fn render_frame(&mut self, document: &Document) -> Result<RenderFrame, String> {
        let mut thumbnails = vec![];
        let pixels = self.compose(
            document,
            PixelRect {
                x: 0,
                y: 0,
                width: document.width,
                height: document.height,
            },
            &|| false,
            Some(&mut thumbnails),
        )?;
        Ok(RenderFrame { pixels, thumbnails })
    }
    /// Composite a validated document, reusing unchanged spatial layers.
    pub fn render(&mut self, document: &Document) -> Result<Raster, String> {
        self.render_cancel(document, &|| false)
    }
    /// Cancellable full composite; cancelled partial images are never returned.
    pub fn render_cancel(
        &mut self,
        document: &Document,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Raster, String> {
        self.region(
            document,
            PixelRect {
                x: 0,
                y: 0,
                width: document.width,
                height: document.height,
            },
            cancel,
        )
    }
    fn prepared(
        &mut self,
        layer: &Layer,
        cancel: &dyn Fn() -> bool,
    ) -> Result<std::sync::Arc<crate::processing::PreparedLayer>, String> {
        if let Some(index) = self.cache.iter().position(|p| p.matches(layer)) {
            let hit = self.cache.remove(index).expect("cached entry");
            self.cache.push_back(hit.clone());
            self.cache_hits += 1;
            return Ok(hit);
        }
        let prepared = std::sync::Arc::new(crate::processing::prepare_layer_cancel(layer, cancel)?);
        self.evaluations += 1;
        let bytes = prepared.byte_len();
        const BUDGET: usize = 128 * 1024 * 1024;
        if bytes <= BUDGET {
            while self.bytes + bytes > BUDGET || self.cache.len() >= 8 {
                if let Some(old) = self.cache.pop_front() {
                    self.bytes = self.bytes.saturating_sub(old.byte_len());
                } else {
                    break;
                }
            }
            self.bytes += bytes;
            self.cache.push_back(prepared.clone());
        }
        Ok(prepared)
    }
    fn region(
        &mut self,
        document: &Document,
        region: PixelRect,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Raster, String> {
        self.compose(document, region, cancel, None)
    }
    fn compose(
        &mut self,
        document: &Document,
        region: PixelRect,
        cancel: &dyn Fn() -> bool,
        mut thumbnails: Option<&mut Vec<(u64, Raster)>>,
    ) -> Result<Raster, String> {
        if region.width == 0
            || region.height == 0
            || u64::from(region.x) + u64::from(region.width) > u64::from(document.width)
            || u64::from(region.y) + u64::from(region.height) > u64::from(document.height)
        {
            return Err("Composite region is outside the canvas".into());
        }
        let mut result = Raster::new(region.width, region.height, document.background)?;
        for layer in document
            .layers
            .iter()
            .filter(|l| l.visible && l.opacity > 0.0)
        {
            if cancel() {
                return Err("Processing cancelled".into());
            }
            let prepared = if crate::processing::has_neighborhood(layer) {
                Some(self.prepared(layer, cancel)?)
            } else {
                None
            };
            // Clip in signed 64-bit coordinates so hostile/extreme offsets cannot overflow.
            let left = i64::from(layer.offset[0]);
            let top = i64::from(layer.offset[1]);
            let bounds = prepared.as_ref().map_or(
                [
                    0,
                    0,
                    i64::from(layer.source.width()),
                    i64::from(layer.source.height()),
                ],
                |p| p.bounds(),
            );
            if let (Some(images), Some(prepared)) = (&mut thumbnails, &prepared) {
                let mut thumbnail = Raster::new(48, 36, [0; 4])?;
                for y in 0..36 {
                    for x in 0..48 {
                        let lx = bounds[0] + (i64::from(x) * (bounds[2] - bounds[0]) / 48);
                        let ly = bounds[1] + (i64::from(y) * (bounds[3] - bounds[1]) / 36);
                        let p = prepared.pixel(lx, ly);
                        let gray = if (x / 6 + y / 6) % 2 == 0 { 185 } else { 225 };
                        thumbnail.set(
                            x,
                            y,
                            over([gray, gray, gray, 255], p, layer.opacity, BlendMode::Normal),
                        );
                    }
                }
                images.push((layer.id, thumbnail));
            }
            for dy in (top + bounds[1]).max(i64::from(region.y))
                ..(top + bounds[3]).min(i64::from(region.y) + i64::from(region.height))
            {
                if cancel() {
                    return Err("Processing cancelled".into());
                }
                for dx in (left + bounds[0]).max(i64::from(region.x))
                    ..(left + bounds[2]).min(i64::from(region.x) + i64::from(region.width))
                {
                    let front = prepared.as_ref().map_or_else(
                        || point_pixel(layer, (dx - left) as u32, (dy - top) as u32),
                        |p| p.pixel(dx - left, dy - top),
                    );
                    let x = dx as u32 - region.x;
                    let y = dy as u32 - region.y;
                    result.set(
                        x,
                        y,
                        over(result.get(x, y), front, layer.opacity, layer.blend),
                    );
                }
            }
        }
        Ok(result)
    }
}
