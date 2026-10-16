//! Continuous round brush rasterization in image coordinates.
use crate::{composite::over, document::BlendMode, raster::Raster};

/// Round brush/eraser footprint parameters, expressed in source-image units.
#[derive(Clone, Copy, Debug)]
pub struct Brush {
    pub diameter: f32,
    pub opacity: f32,
    pub hardness: f32,
    pub color: [u8; 4],
    pub erase: bool,
}

impl Brush {
    /// Union coverage over a segment avoids visible gaps between fast samples.
    /// A soft edge has a linear falloff between the hard core and outer radius.
    pub fn segment(&self, raster: &mut Raster, from: [f32; 2], to: [f32; 2]) {
        let radius = (self.diameter * 0.5).max(0.5);
        let x0 = ((from[0].min(to[0]) - radius).floor().max(0.0) as u32).min(raster.width());
        let y0 = ((from[1].min(to[1]) - radius).floor().max(0.0) as u32).min(raster.height());
        let x1 = ((from[0].max(to[0]) + radius).ceil().max(0.0) as u32).min(raster.width());
        let y1 = ((from[1].max(to[1]) + radius).ceil().max(0.0) as u32).min(raster.height());
        let vx = to[0] - from[0];
        let vy = to[1] - from[1];
        let len2 = vx * vx + vy * vy;
        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5 - from[0];
                let py = y as f32 + 0.5 - from[1];
                let dot = px * vx + py * vy;
                // Cross-product distance avoids cancellation along the segment
                // centerline (which otherwise causes uneven 8-bit alpha rounding).
                let distance = if len2 == 0.0 || dot <= 0.0 {
                    px.hypot(py)
                } else if dot >= len2 {
                    (px - vx).hypot(py - vy)
                } else {
                    (px * vy - py * vx).abs() / len2.sqrt()
                };
                let hard = self.hardness.clamp(0.0, 1.0);
                let edge = ((radius - distance) / (radius * (1.0 - hard)).max(0.5)
                    + if hard == 1.0 { 0.5 } else { 0.0 })
                .clamp(0.0, 1.0);
                if edge == 0.0 {
                    continue;
                }
                let old = raster.get(x, y);
                let opacity = edge * self.opacity.clamp(0.0, 1.0);
                let pixel = if self.erase {
                    // Destination-out: preserve straight RGB, reduce alpha.
                    [
                        old[0],
                        old[1],
                        old[2],
                        (f32::from(old[3]) * (1.0 - opacity)).round() as u8,
                    ]
                } else {
                    over(old, self.color, opacity, BlendMode::Normal)
                };
                raster.set(x, y, pixel);
            }
        }
    }
}
