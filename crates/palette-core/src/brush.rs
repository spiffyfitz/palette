//! Continuous round brush rasterization in image coordinates.
use crate::{
    composite::over,
    document::BlendMode,
    raster::{Coverage, Raster, TILE},
};
use std::collections::HashMap;
mod styles;
pub use styles::{Sample, Stroke, Style};

/// Round brush/eraser footprint parameters, expressed in source-image units.
#[derive(Clone, Copy, Debug)]
pub struct Brush {
    pub diameter: f32,
    pub opacity: f32,
    pub hardness: f32,
    pub color: [u8; 4],
    pub erase: bool,
}

fn selected(selection: Option<(&Coverage, [i32; 2])>, x: u32, y: u32) -> f32 {
    selection.map_or(1.0, |(mask, offset)| {
        crate::editing::coverage(
            Some(mask),
            i64::from(x) + i64::from(offset[0]),
            i64::from(y) + i64::from(offset[1]),
            mask.width,
            mask.height,
        )
    })
}

fn apply(brush: Brush, before: [u8; 4], opacity: f32) -> [u8; 4] {
    if brush.erase {
        // Destination-out preserves straight RGB and attenuates only alpha.
        [
            before[0],
            before[1],
            before[2],
            (f32::from(before[3]) * (1.0 - opacity)).round() as u8,
        ]
    } else {
        over(before, brush.color, opacity, BlendMode::Normal)
    }
}

impl Brush {
    /// Paint one isolated capsule segment with source-over/destination-out.
    pub fn segment(&self, raster: &mut Raster, from: [f32; 2], to: [f32; 2]) {
        self.segment_selected(raster, from, to, None);
    }

    /// Paint through a canvas-space selection. Use PaintStroke for a continuous
    /// gesture so shared segment footprints do not accumulate opacity at joins.
    pub fn segment_selected(
        &self,
        raster: &mut Raster,
        from: [f32; 2],
        to: [f32; 2],
        selection: Option<(&Coverage, [i32; 2])>,
    ) {
        self.visit([raster.width(), raster.height()], from, to, |x, y, edge| {
            let opacity = edge * self.opacity.clamp(0.0, 1.0) * selected(selection, x, y);
            if opacity > 0.0 {
                raster.set(x, y, apply(*self, raster.get(x, y), opacity));
            }
        });
    }

    pub(crate) fn visit(
        &self,
        dimensions: [u32; 2],
        from: [f32; 2],
        to: [f32; 2],
        mut pixel: impl FnMut(u32, u32, f32),
    ) {
        let radius = (self.diameter * 0.5).max(0.5);
        let x0 = ((from[0].min(to[0]) - radius).floor().max(0.0) as u32).min(dimensions[0]);
        let y0 = ((from[1].min(to[1]) - radius).floor().max(0.0) as u32).min(dimensions[1]);
        let x1 = ((from[0].max(to[0]) + radius).ceil().max(0.0) as u32).min(dimensions[0]);
        let y1 = ((from[1].max(to[1]) + radius).ceil().max(0.0) as u32).min(dimensions[1]);
        let vx = to[0] - from[0];
        let vy = to[1] - from[1];
        let len2 = vx * vx + vy * vy;
        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5 - from[0];
                let py = y as f32 + 0.5 - from[1];
                let dot = px * vx + py * vy;
                // Cross-product distance avoids cancellation on the centerline.
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
                if edge > 0.0 {
                    pixel(x, y, edge);
                }
            }
        }
    }
}

/// One gesture's source snapshot and sparse maximum-coverage tiles. Each pixel
/// is evaluated from its pre-stroke value, independent of pointer sample count.
/// Retracing within a stroke does not darken it; a new stroke can build opacity.
pub struct PaintStroke {
    before: Raster,
    coverage: HashMap<u32, Box<[f32]>>,
    dirty: Option<crate::editing::PixelRect>,
}

impl PaintStroke {
    fn write(&mut self, raster: &mut Raster, x: u32, y: u32, pixel: [u8; 4]) {
        if raster.get(x, y) == pixel {
            return;
        }
        raster.set(x, y, pixel);
        let bounds = self.dirty.get_or_insert(crate::editing::PixelRect {
            x,
            y,
            width: 1,
            height: 1,
        });
        let right = (bounds.x + bounds.width).max(x + 1);
        let bottom = (bounds.y + bounds.height).max(y + 1);
        bounds.x = bounds.x.min(x);
        bounds.y = bounds.y.min(y);
        bounds.width = right - bounds.x;
        bounds.height = bottom - bounds.y;
    }
    /// Share the pre-stroke tiles; allocate coverage only for touched tiles.
    pub fn new(source: &Raster) -> Self {
        Self {
            before: source.clone(),
            coverage: HashMap::new(),
            dirty: None,
        }
    }

    /// Extend the stroke using a continuous capsule and selection coverage.
    pub fn segment(
        &mut self,
        brush: Brush,
        raster: &mut Raster,
        from: [f32; 2],
        to: [f32; 2],
        selection: Option<(&Coverage, [i32; 2])>,
    ) {
        brush.visit([raster.width(), raster.height()], from, to, |x, y, edge| {
            let opacity = edge * brush.opacity.clamp(0.0, 1.0) * selected(selection, x, y);
            if opacity == 0.0 {
                return;
            }
            let key = (y / TILE) * raster.width().div_ceil(TILE) + x / TILE;
            let tile = self
                .coverage
                .entry(key)
                .or_insert_with(|| vec![0.0; (TILE * TILE) as usize].into_boxed_slice());
            let previous = &mut tile[((y % TILE) * TILE + x % TILE) as usize];
            if opacity <= *previous {
                return;
            }
            *previous = opacity;
            let pixel = apply(brush, self.before.get(x, y), opacity);
            if raster.get(x, y) != pixel {
                raster.set(x, y, pixel);
                let bounds = self.dirty.get_or_insert(crate::editing::PixelRect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                });
                let right = (bounds.x + bounds.width).max(x + 1);
                let bottom = (bounds.y + bounds.height).max(y + 1);
                bounds.x = bounds.x.min(x);
                bounds.y = bounds.y.min(y);
                bounds.width = right - bounds.x;
                bounds.height = bottom - bounds.y;
            }
        });
    }

    /// Consume source-space bounds changed since the last display update.
    pub fn take_dirty_region(&mut self) -> Option<crate::editing::PixelRect> {
        self.dirty.take()
    }
}
