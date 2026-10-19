//! Continuous, source-preserving selection painting. Edge-aware mode follows
//! four-connected pixels within the swept footprint that match the stroke's
//! initial sampled color. Plain mode paints grayscale coverage directly.
use super::{perceptual, *};
use crate::{brush::Brush, raster::Raster};

/// Selection brush settings in image units. Strength caps coverage per gesture;
/// separate gestures can build coverage through the selection union operation.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub diameter: f32,
    pub hardness: f32,
    pub strength: f32,
    pub tolerance: f32,
    pub edge_aware: bool,
    pub sample_merged: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            diameter: 40.,
            hardness: 0.8,
            strength: 1.,
            tolerance: 12.,
            edge_aware: true,
            sample_merged: false,
        }
    }
}
impl Settings {
    fn validate(self) -> Result<(), String> {
        if !self.diameter.is_finite()
            || !(1. ..=256.).contains(&self.diameter)
            || [self.hardness, self.strength]
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
            || !self.tolerance.is_finite()
            || !(0. ..=100.).contains(&self.tolerance)
        {
            return Err("Invalid selection brush size, hardness, strength or tolerance".into());
        }
        Ok(())
    }
}

fn valid_point(point: [f32; 2]) -> bool {
    point.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.)
}

/// Incremental stroke used by the preview worker. Sampling uses an immutable COW
/// document snapshot; selections, locks, hidden RGB and pointer frequency cannot
/// change the source mid-stroke. No image pixels are ever written.
pub struct Stroke {
    document: Document,
    merged: Option<Raster>,
    local: Option<std::sync::Arc<crate::processing::PreparedLayer>>,
    settings: Settings,
    target: [f64; 4],
    seed: [u32; 2],
    last: [f32; 2],
    candidates: Coverage,
    colors: HashMap<[u8; 4], bool>,
}
impl Stroke {
    /// Pin the initial color. A new click resamples; moving during this stroke
    /// retains the color so crossing an edge cannot silently select its far side.
    pub fn new(document: &Document, start: [f32; 2], settings: Settings) -> Result<Self, String> {
        document.validate()?;
        settings.validate()?;
        if !valid_point(start)
            || start[0] < 0.
            || start[1] < 0.
            || start[0] >= document.width as f32
            || start[1] >= document.height as f32
        {
            return Err("Selection brush must start inside the canvas".into());
        }
        let mut stroke = Self {
            document: document.clone(),
            merged: (settings.edge_aware && settings.sample_merged)
                .then(|| composite::render(document)),
            local: (settings.edge_aware && !settings.sample_merged)
                .then(|| {
                    composite::Renderer::default().layer(document, document.active_layer, &|| false)
                })
                .transpose()?,
            settings,
            target: [0.; 4],
            seed: start.map(|v| v.floor() as u32),
            last: start,
            candidates: Coverage {
                width: document.width,
                height: document.height,
                enabled: true,
                values: Arc::new(vec![0; (document.width * document.height) as usize]),
            },
            colors: HashMap::new(),
        };
        stroke.target = perceptual(stroke.sample(stroke.seed[0], stroke.seed[1]));
        stroke.extend(start)?;
        Ok(stroke)
    }
    fn sample(&self, x: u32, y: u32) -> [u8; 4] {
        if let Some(raster) = &self.merged {
            return raster.get(x, y);
        }
        let layer = self.document.active();
        let lx = i64::from(x) - i64::from(layer.offset[0]);
        let ly = i64::from(y) - i64::from(layer.offset[1]);
        let Some(local) = &self.local else {
            return [0; 4];
        };
        let mut pixel = local.pixel(lx, ly);
        pixel[3] = (f32::from(pixel[3]) * layer.opacity).round() as u8;
        pixel
    }
    fn matches(&mut self, x: u32, y: u32) -> bool {
        if !self.settings.edge_aware || self.settings.tolerance == 100. {
            return true;
        }
        let pixel = self.sample(x, y);
        if let Some(result) = self.colors.get(&pixel) {
            return *result;
        }
        let limit = f64::from(self.settings.tolerance) / 100.;
        let matches = perceptual(pixel)
            .iter()
            .zip(self.target)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            <= limit * limit;
        if self.colors.len() < 4096 {
            self.colors.insert(pixel, matches);
        }
        matches
    }
    /// Extend with a capsule, rather than isolated dabs, so fast motion has no
    /// gaps. Maximum coverage prevents overlap from building strength at joins.
    pub fn extend(&mut self, to: [f32; 2]) -> Result<(), String> {
        if !valid_point(to) {
            return Err("Invalid selection brush point".into());
        }
        let brush = Brush {
            diameter: self.settings.diameter,
            // At a one-pixel diameter there is no resolvable soft interior.
            // Use a pixel-centered antialiased tip so integer coordinates and
            // diagonal motion cannot produce an empty or disconnected stroke.
            hardness: if self.settings.diameter == 1. {
                1.
            } else {
                self.settings.hardness
            },
            opacity: self.settings.strength,
            color: [0; 4],
            erase: false,
        };
        let center = |point: [f32; 2]| {
            if self.settings.diameter == 1. {
                point.map(|v| v.floor() + 0.5)
            } else {
                point
            }
        };
        brush.visit(
            [self.document.width, self.document.height],
            center(self.last),
            center(to),
            |x, y, edge| {
                let value = (edge * brush.opacity * 255.).round() as u8;
                let index = (y * self.document.width + x) as usize;
                if value > self.candidates.values[index] && self.matches(x, y) {
                    Arc::make_mut(&mut self.candidates.values)[index] = value;
                }
            },
        );
        self.last = to;
        Ok(())
    }
    /// Coverage prior to Gaussian feathering. Connectivity prevents the brush
    /// from jumping across a contrasting barrier into a disconnected color island.
    pub fn coverage(&self) -> Result<Coverage, String> {
        if !self.settings.edge_aware {
            return Ok(self.candidates.clone());
        }
        let (width, height) = (self.document.width, self.document.height);
        let mut values = vec![0; (width * height) as usize];
        let eligible = |x: u32, y: u32, values: &[u8]| {
            let i = (y * width + x) as usize;
            self.candidates.values[i] != 0 && values[i] == 0
        };
        let mut stack = vec![(self.seed[0], self.seed[1])];
        while let Some((sx, y)) = stack.pop() {
            if !eligible(sx, y, &values) {
                continue;
            }
            let mut x = sx;
            while x > 0 && eligible(x - 1, y, &values) {
                x -= 1;
            }
            let (mut above, mut below) = (false, false);
            while x < width && eligible(x, y, &values) {
                let i = (y * width + x) as usize;
                values[i] = self.candidates.values[i];
                for (ny, run) in [
                    (y.checked_sub(1), &mut above),
                    ((y + 1 < height).then_some(y + 1), &mut below),
                ] {
                    let yes = ny.is_some_and(|ny| eligible(x, ny, &values));
                    if yes && !*run {
                        if stack.len() >= 1_048_576 {
                            return Err("Selection brush exceeds the pending-span budget".into());
                        }
                        stack.push((x, ny.expect("eligible span")));
                    }
                    *run = yes;
                }
                x += 1;
            }
        }
        Ok(Coverage {
            width,
            height,
            enabled: true,
            values: Arc::new(values),
        })
    }
}

/// Deterministic final evaluation, also used when a pending gesture must finish
/// before a save/tab/tool action. Incremental previews use `Stroke` directly.
pub fn from_points(
    document: &Document,
    points: &[[f32; 2]],
    settings: Settings,
) -> Result<Coverage, String> {
    let first = points
        .first()
        .ok_or("Selection brush needs a starting point")?;
    if points.len() > 16384 {
        return Err("Selection brush stroke exceeds 16384 samples".into());
    }
    let mut stroke = Stroke::new(document, *first, settings)?;
    for point in &points[1..] {
        stroke.extend(*point)?;
    }
    stroke.coverage()
}
