//! Version-independent editing state. No GUI or GPU types belong here.
use crate::raster::{Coverage, Raster};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

/// Blend functions currently supported by the reference compositor.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    LinearDodge,
    LinearBurn,
    SoftLight,
    HardLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Divide,
}

impl BlendMode {
    /// Stable presentation order; serde retains the existing named variants.
    pub const ALL: [Self; 19] = [
        Self::Normal,
        Self::Darken,
        Self::Multiply,
        Self::ColorBurn,
        Self::LinearBurn,
        Self::Lighten,
        Self::Screen,
        Self::ColorDodge,
        Self::LinearDodge,
        Self::Overlay,
        Self::SoftLight,
        Self::HardLight,
        Self::Difference,
        Self::Exclusion,
        Self::Divide,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];

    /// Human-readable names shared by layer properties and reference sheets.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "Color Dodge",
            Self::ColorBurn => "Color Burn",
            Self::LinearDodge => "Linear Dodge (Add)",
            Self::LinearBurn => "Linear Burn",
            Self::SoftLight => "Soft Light",
            Self::HardLight => "Hard Light",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
            Self::Divide => "Divide",
        }
    }
}

/// Only implemented filters enter the model and the registry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Filter {
    Invert,
    Brightness {
        amount: f32,
    },
    /// Gaussian sigma in image pixels; evaluation preserves source pixels.
    GaussianBlur {
        radius: f32,
    },
    BrightnessContrast {
        brightness: f32,
        contrast: f32,
    },
    /// Stops applied in linear-light RGB, encoded back to sRGB.
    Exposure {
        stops: f32,
    },
    HueSaturation {
        hue: f32,
        saturation: f32,
        lightness: f32,
    },
    Vibrance {
        amount: f32,
    },
    BlackWhite {
        strength: f32,
    },
    Threshold {
        level: u8,
    },
    Posterize {
        levels: u8,
    },
    BoxBlur {
        radius: u8,
    },
    Median {
        radius: u8,
    },
    Levels {
        black: u8,
        white: u8,
        gamma: f32,
        output_black: u8,
        output_white: u8,
    },
    /// Cubic curve outputs at fixed input knots 0, 64, 128, 192 and 255.
    Curves {
        points: [u8; 5],
    },
    MotionBlur {
        distance: f32,
        angle: f32,
    },
    HighPass {
        radius: f32,
    },
    Mosaic {
        block_size: u8,
    },
    AddNoise {
        amount: f32,
        seed: u16,
        monochrome: bool,
        distribution: NoiseDistribution,
    },
}

/// Independently generated noise distributions, with stable serialized names.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NoiseDistribution {
    Uniform,
    Gaussian,
}
impl NoiseDistribution {
    /// Stable order for choice controls.
    pub const ALL: [Self; 2] = [Self::Uniform, Self::Gaussian];
}

/// Placement relative to the layer's alpha silhouette.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrokePosition {
    Outside,
    Inside,
    Center,
}
impl StrokePosition {
    /// Stable order for registry choice controls.
    pub const ALL: [Self; 3] = [Self::Outside, Self::Inside, Self::Center];
}

/// Stable, individually maskable instance in a layer's ordered filter pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FilterInstance {
    pub id: u64,
    pub enabled: bool,
    pub filter: Filter,
    pub mask: Option<Coverage>,
}

/// Implemented layer-style operations; new styles extend this versioned enum.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Effect {
    /// Blend constant RGB through the layer's alpha silhouette.
    ColorOverlay { color: [u8; 3], opacity: f32 },
    DropShadow {
        color: [u8; 3],
        opacity: f32,
        angle: f32,
        distance: f32,
        spread: f32,
        size: f32,
    },
    OuterGlow {
        color: [u8; 3],
        opacity: f32,
        spread: f32,
        size: f32,
    },
    InnerShadow {
        color: [u8; 3],
        opacity: f32,
        angle: f32,
        distance: f32,
        spread: f32,
        size: f32,
    },
    InnerGlow {
        color: [u8; 3],
        opacity: f32,
        spread: f32,
        size: f32,
    },
    /// Subpixel square or Euclidean round contour, preserving source pixels.
    Stroke {
        color: [u8; 3],
        opacity: f32,
        width: f32,
        position: StrokePosition,
        /// Missing in v1–v4 projects: preserve their original square joins.
        #[serde(default)]
        rounded: bool,
    },
}

/// An independently enabled, nondestructive layer-style operation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EffectInstance {
    pub id: u64,
    pub enabled: bool,
    pub effect: Effect,
}

/// Source pixels remain untouched by masks, filters, blending and positioning.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    pub source: Raster,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub offset: [i32; 2],
    pub mask: Option<Coverage>,
    pub filters: Vec<FilterInstance>,
    pub effects: Vec<EffectInstance>,
}

/// Layers are stored bottom to top; IDs are monotonic within a document.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    /// Canvas backing color, separate from editable raster layers (HEIF iovl).
    #[serde(default)]
    pub background: [u8; 4],
    pub layers: Vec<Layer>,
    pub active_layer: u64,
    pub selection: Option<Coverage>,
    /// Original selection contour retained separately from feathered editing
    /// coverage. Shared immutable data participates in history and native saving.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_base: Option<Coverage>,
    pub(crate) next_id: u64,
    /// Opaque source metadata retained by native projects. Pixels currently use
    /// the editor's RGBA8/sRGB working interpretation; profiles are not applied.
    #[serde(default)]
    pub metadata: DocumentMetadata,
}

/// Bounded original metadata, preserved independently from raster source data.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct DocumentMetadata {
    pub icc_profile: Arc<Vec<u8>>,
    pub exif: Arc<Vec<u8>>,
}

impl Document {
    pub(crate) fn allocations(&self) -> std::collections::HashMap<usize, usize> {
        let mut output = std::collections::HashMap::new();
        for data in [&self.metadata.icc_profile, &self.metadata.exif] {
            output.insert(Arc::as_ptr(data) as usize, data.capacity());
        }
        let mut mask = |mask: &Coverage| {
            output.insert(
                std::sync::Arc::as_ptr(&mask.values) as usize,
                mask.values.capacity(),
            );
        };
        if let Some(selection) = &self.selection {
            mask(selection);
        }
        if let Some(base) = &self.selection_base {
            mask(base);
        }
        for layer in &self.layers {
            if let Some(m) = &layer.mask {
                mask(m);
            }
            for filter in &layer.filters {
                if let Some(m) = &filter.mask {
                    mask(m);
                }
            }
        }
        for layer in &self.layers {
            layer.source.allocations(&mut output);
        }
        output
    }
    /// Create a white default background of the requested bounded dimensions.
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let source = Raster::new(width, height, [255; 4])?;
        Ok(Self::from_raster(source))
    }

    /// Wrap a raster as an editable single-layer document.
    pub fn from_raster(source: Raster) -> Self {
        Self {
            width: source.width(),
            height: source.height(),
            background: [0; 4],
            layers: vec![Layer {
                id: 1,
                name: "Background".into(),
                source,
                visible: true,
                locked: false,
                opacity: 1.0,
                blend: BlendMode::Normal,
                offset: [0, 0],
                mask: None,
                filters: vec![],
                effects: vec![],
            }],
            active_layer: 1,
            selection: None,
            selection_base: None,
            next_id: 2,
            metadata: DocumentMetadata::default(),
        }
    }

    /// Allocate a stable ID within this document's current command history.
    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Return the active layer; requires a validated document.
    pub fn active(&self) -> &Layer {
        self.layers
            .iter()
            .find(|l| l.id == self.active_layer)
            .expect("validated active layer")
    }
    /// Mutably access the active layer; requires a validated document.
    pub fn active_mut(&mut self) -> &mut Layer {
        self.layers
            .iter_mut()
            .find(|l| l.id == self.active_layer)
            .expect("validated active layer")
    }
    /// Add and activate a transparent, full-canvas layer at the top of the stack.
    pub fn add_layer(&mut self) {
        let id = self.allocate_id();
        self.layers.push(Layer {
            id,
            name: format!("Layer {id}"),
            source: Raster::new(self.width, self.height, [0; 4]).expect("valid canvas"),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend: BlendMode::Normal,
            offset: [0, 0],
            mask: None,
            filters: vec![],
            effects: vec![],
        });
        self.active_layer = id;
    }
    /// Duplicate the active layer and allocate fresh layer/filter/effect IDs.
    pub fn duplicate_layer(&mut self) {
        let mut copy = self.active().clone();
        copy.id = self.allocate_id();
        for filter in &mut copy.filters {
            filter.id = self.allocate_id();
        }
        for effect in &mut copy.effects {
            effect.id = self.allocate_id();
        }
        copy.name.push_str(" copy");
        let index = self
            .layers
            .iter()
            .position(|l| l.id == self.active_layer)
            .expect("active");
        self.active_layer = copy.id;
        self.layers.insert(index + 1, copy);
    }
    /// Remove the active layer while retaining at least one raster layer.
    pub fn delete_layer(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        let index = self
            .layers
            .iter()
            .position(|l| l.id == self.active_layer)
            .expect("active");
        self.layers.remove(index);
        self.active_layer = self.layers[index.min(self.layers.len() - 1)].id;
    }

    /// Logical raster/mask bytes; shared backing allocations may use less RAM.
    pub fn byte_len(&self) -> usize {
        self.layers
            .iter()
            .map(|l| {
                l.source.byte_len()
                    + l.mask.as_ref().map_or(0, |m| m.values.len())
                    + l.filters
                        .iter()
                        .map(|f| f.mask.as_ref().map_or(0, |m| m.values.len()))
                        .sum::<usize>()
            })
            .sum::<usize>()
            + self.selection.as_ref().map_or(0, |m| m.values.len())
            + self.selection_base.as_ref().map_or(0, |m| m.values.len())
            + self.metadata.icc_profile.len()
            + self.metadata.exif.len()
    }

    /// Validate all serialized invariants before any indexing or rendering.
    pub fn validate(&self) -> Result<(), String> {
        if self.metadata.icc_profile.len() > 4 * 1024 * 1024
            || self.metadata.exif.len() > 4 * 1024 * 1024
        {
            return Err("ICC/Exif metadata exceeds the 4 MiB per-field limit".into());
        }
        if self.layers.is_empty() || self.layers.len() > 256 || self.next_id > u64::MAX - 1024 {
            return Err("Invalid layer count or ID counter".into());
        }
        Raster::new(self.width, self.height, [0; 4])?;
        let mut ids = HashSet::new();
        for layer in &self.layers {
            if layer.id == 0
                || layer.id >= self.next_id
                || !ids.insert(layer.id)
                || !layer.opacity.is_finite()
                || !(0.0..=1.0).contains(&layer.opacity)
                || layer.name.len() > 4096
                || layer.filters.len() > 128
                || layer.effects.len() > 128
            {
                return Err("Invalid layer properties".into());
            }
            layer.source.validate()?;
            if let Some(mask) = &layer.mask {
                mask.validate()?;
            }
            for filter in &layer.filters {
                if filter.id == 0 || filter.id >= self.next_id || !ids.insert(filter.id) {
                    return Err("Invalid filter ID".into());
                }
                crate::processing::Operation::Filter(filter.filter.clone()).validate()?;
                if let Some(mask) = &filter.mask {
                    mask.validate()?;
                }
            }
        }
        for effect in self.layers.iter().flat_map(|l| &l.effects) {
            if effect.id == 0 || effect.id >= self.next_id || !ids.insert(effect.id) {
                return Err("Invalid effect ID".into());
            }
            crate::processing::Operation::Effect(effect.effect.clone()).validate()?;
        }
        for layer in &self.layers {
            let mut prepared = layer.source.byte_len() as u64;
            for effect in layer.effects.iter().filter(|e| e.enabled) {
                if let Some(pad) = crate::processing::effect_padding(&effect.effect) {
                    Raster::new(
                        layer.source.width() + 2 * pad,
                        layer.source.height() + 2 * pad,
                        [0; 4],
                    )?;
                    let w = u64::from(layer.source.width() + 2 * pad)
                        .div_ceil(u64::from(crate::raster::TILE));
                    let h = u64::from(layer.source.height() + 2 * pad)
                        .div_ceil(u64::from(crate::raster::TILE));
                    let planes = if matches!(
                        effect.effect,
                        Effect::Stroke {
                            position: StrokePosition::Center,
                            ..
                        }
                    ) {
                        2
                    } else {
                        1
                    };
                    prepared += w * h * u64::from(crate::raster::TILE).pow(2) * 4 * planes;
                }
            }
            if prepared > 512 * 1024 * 1024 {
                return Err("Layer processing exceeds the 512 MiB prepared-image budget".into());
            }
        }
        if !self.layers.iter().any(|l| l.id == self.active_layer) {
            return Err("Active layer is missing".into());
        }
        if self.selection_base.is_some() && self.selection.is_none() {
            return Err("Original selection requires editing coverage".into());
        }
        for selection in self.selection.iter().chain(&self.selection_base) {
            selection.validate()?;
            if selection.width != self.width || selection.height != self.height {
                return Err("Selection dimensions must match the canvas".into());
            }
        }
        Ok(())
    }
}
