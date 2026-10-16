//! Deterministic reference CPU compositor. Working RGB is sRGB (not linear light).
use crate::document::{BlendMode, Document, Effect, Filter, Layer};
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
    for c in 0..3 {
        let b = f32::from(back[c]) / 255.0;
        let f = f32::from(front[c]) / 255.0;
        let blend = match mode {
            BlendMode::Normal => f,
            BlendMode::Multiply => b * f,
            BlendMode::Screen => b + f - b * f,
        };
        out[c] = (((1.0 - af) * ab * b + (1.0 - ab) * af * f + ab * af * blend) / a * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    out[3] = (a * 255.0).round() as u8;
    out
}

/// Evaluate point filters, color styles and mask without modifying source pixels.
/// Layer opacity is applied by the compositor.
pub fn layer_pixel(layer: &Layer, x: u32, y: u32) -> [u8; 4] {
    let mut pixel = layer.source.get(x, y);
    for instance in layer.filters.iter().filter(|f| f.enabled) {
        let original = pixel;
        for c in 0..3 {
            pixel[c] = match instance.filter {
                Filter::Invert => 255 - original[c],
                Filter::Brightness { amount } => (f32::from(original[c]) + amount * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8,
            };
        }
        let coverage = instance.mask.as_ref().map_or(1.0, |m| m.at(x, y));
        for c in 0..3 {
            pixel[c] = (f32::from(original[c]) * (1.0 - coverage) + f32::from(pixel[c]) * coverage)
                .round() as u8;
        }
    }
    for instance in layer.effects.iter().filter(|e| e.enabled) {
        let Effect::ColorOverlay { color, opacity } = instance.effect;
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
    let mut result =
        Raster::new(document.width, document.height, [0; 4]).expect("validated document");
    for layer in document
        .layers
        .iter()
        .filter(|l| l.visible && l.opacity > 0.0)
    {
        // Clip in signed 64-bit coordinates so hostile/extreme offsets cannot overflow.
        let left = i64::from(layer.offset[0]);
        let top = i64::from(layer.offset[1]);
        for dy in
            top.max(0)..(top + i64::from(layer.source.height())).min(i64::from(document.height))
        {
            for dx in
                left.max(0)..(left + i64::from(layer.source.width())).min(i64::from(document.width))
            {
                let front = layer_pixel(layer, (dx - left) as u32, (dy - top) as u32);
                let x = dx as u32;
                let y = dy as u32;
                result.set(
                    x,
                    y,
                    over(result.get(x, y), front, layer.opacity, layer.blend),
                );
            }
        }
    }
    result
}
