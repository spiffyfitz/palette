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

/// Independent Paint Daubs kernel geometries; stable stored names.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaubStyle {
    Round,
    Square,
    Diagonal,
}
impl DaubStyle {
    /// Presentation order used by the common parameter editor.
    pub const ALL: [Self; 3] = [Self::Round, Self::Square, Self::Diagonal];
}

/// Only implemented filters enter the model and the registry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Filter {
    ColoredPencil {
        width: f32,
        pressure: f32,
        brightness: f32,
        paper: [u8; 3],
        seed: u16,
    },
    DryBrush {
        radius: f32,
        detail: f32,
        texture: f32,
        seed: u16,
    },
    PaintDaubs {
        radius: f32,
        sharpness: f32,
        style: DaubStyle,
    },
    SmudgeStick {
        length: f32,
        highlight: f32,
        intensity: f32,
    },
    AngledStrokes {
        balance: f32,
        length: f32,
        sharpness: f32,
    },
    Crosshatch {
        length: f32,
        sharpness: f32,
        strength: f32,
    },
    DarkStrokes {
        balance: f32,
        black: f32,
        white: f32,
    },
    Spatter {
        radius: f32,
        smoothness: f32,
        seed: u16,
    },

    /// Tonal ranges (shadows/midtones/highlights), each with signed RGB balance.
    ColorBalance {
        balance: [[f32; 3]; 3],
        preserve_luminosity: bool,
    },
    PhotoFilter {
        color: [u8; 3],
        density: f32,
        preserve_luminosity: bool,
    },
    /// Rows are output RGB; columns are input RGB plus an additive constant.
    /// Monochrome uses the first row, retaining the other rows for later editing.
    ChannelMixer {
        matrix: [[f32; 4]; 3],
        monochrome: bool,
    },
    /// Two-stop encoded-sRGB gradient, with an editable 50% transition location.
    GradientMap {
        low: [u8; 3],
        high: [u8; 3],
        midpoint: f32,
        reverse: bool,
    },
    /// Rows: reds/yellows/greens/cyans/blues/magentas/whites/neutrals/blacks.
    /// Columns: cyan/magenta/yellow/black. All range edits coexist in one instance.
    SelectiveColor {
        adjustments: [[f32; 4]; 9],
        relative: bool,
    },
    /// Separate alpha-weighted local guides for the two tonal corrections.
    ShadowsHighlights {
        shadows: f32,
        shadow_width: f32,
        shadow_radius: f32,
        highlights: f32,
        highlight_width: f32,
        highlight_radius: f32,
    },
    Blur,
    BlurMore,
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
    Average,
    FindEdges,
    Solarize,
    Emboss {
        angle: f32,
        height: u8,
        amount: f32,
    },
    UnsharpMask {
        radius: f32,
        amount: f32,
        threshold: u8,
    },
    Minimum {
        radius: u8,
    },
    Maximum {
        radius: u8,
    },
    Offset {
        x: i32,
        y: i32,
        edges: FilterEdges,
    },
    Twirl {
        /// Normalized source pixel-center coordinates, independent of preview size.
        #[serde(default = "default_filter_center")]
        center: [f32; 2],
        radius: f32,
        angle: f32,
    },
    Wave {
        /// Normalized source pixel-center coordinates, independent of preview size.
        #[serde(default)]
        center: [f32; 2],
        amplitude: f32,
        wavelength: f32,
        vertical: bool,
        edges: FilterEdges,
    },
    SurfaceBlur {
        radius: u8,
        threshold: u8,
    },
    RadialBlur {
        /// Normalized source pixel-center coordinates, independent of preview size.
        #[serde(default = "default_filter_center")]
        center: [f32; 2],
        amount: f32,
        samples: u8,
        zoom: bool,
    },
    Clouds {
        seed: u16,
        scale: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    DifferenceClouds {
        seed: u16,
        scale: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    PosterEdges {
        thickness: u8,
        intensity: f32,
        levels: u8,
    },
    Grain {
        amount: f32,
        contrast: f32,
        size: f32,
        seed: u16,
        monochrome: bool,
        style: GrainStyle,
    },
    Fragment {
        distance: f32,
    },
    Despeckle,
    DustScratches {
        radius: u8,
        threshold: u8,
    },
    ReduceNoise {
        radius: u8,
        strength: f32,
        preserve_details: f32,
        color_noise: f32,
        sharpen: f32,
    },
    Pinch {
        /// Normalized source pixel-center coordinates, independent of preview size.
        #[serde(default = "default_filter_center")]
        center: [f32; 2],
        amount: f32,
        radius: f32,
    },
    Spherize {
        /// Normalized source pixel-center coordinates, independent of preview size.
        #[serde(default = "default_filter_center")]
        center: [f32; 2],
        amount: f32,
        radius: f32,
    },
    Ripple {
        /// Normalized source pixel-center coordinates, independent of preview size.
        #[serde(default = "default_filter_center")]
        center: [f32; 2],
        amplitude: f32,
        wavelength: f32,
    },
    Sharpen,
    SharpenMore,
    PolarCoordinates {
        center: [f32; 2],
        to_polar: bool,
    },
    ZigZag {
        center: [f32; 2],
        amount: f32,
        radius: f32,
        ridges: u8,
        style: ZigZagStyle,
    },
    Fibers {
        seed: u16,
        variance: f32,
        strength: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    GlowingEdges {
        width: f32,
        brightness: f32,
        smoothness: f32,
    },
    Cutout {
        levels: u8,
        simplicity: f32,
        fidelity: f32,
    },
    FilmGrain {
        grain: f32,
        highlight: f32,
        intensity: f32,
        seed: u16,
    },
    AccentedEdges {
        width: f32,
        brightness: f32,
        smoothness: f32,
    },
    InkOutlines {
        length: f32,
        dark: f32,
        light: f32,
    },
    Photocopy {
        detail: f32,
        darkness: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    HalftonePattern {
        size: f32,
        contrast: f32,
        pattern: HalftoneStyle,
        low: [u8; 3],
        high: [u8; 3],
    },
    Patchwork {
        square: u8,
        relief: f32,
    },
    Craquelure {
        spacing: f32,
        depth: f32,
        brightness: f32,
        seed: u16,
    },
    ColorHalftone {
        radius: f32,
        cyan: f32,
        magenta: f32,
        yellow: f32,
        black: f32,
    },
    Crystallize {
        size: u8,
        seed: u16,
    },
    Facet,
    Mezzotint {
        pattern: MezzotintStyle,
        scale: f32,
        seed: u16,
        monochrome: bool,
    },
    Pointillize {
        size: u8,
        dot: f32,
        color: [u8; 3],
        seed: u16,
    },
    StainedGlass {
        size: u8,
        width: f32,
        light: f32,
        color: [u8; 3],
        seed: u16,
    },
    MosaicTiles {
        size: u8,
        grout: f32,
        lighten: bool,
    },
    Diffuse {
        radius: u8,
        mode: DiffuseMode,
        seed: u16,
    },
}

/// Conventional monochrome screen patterns; names are stable project values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum HalftoneStyle {
    Dot,
    Line,
    Circle,
}
impl HalftoneStyle {
    /// Stable UI choice order.
    pub const ALL: [Self; 3] = [Self::Dot, Self::Line, Self::Circle];
}

/// Stable conventional mezzotint marks; dimensions are defined by the evaluator.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MezzotintStyle {
    FineDots,
    MediumDots,
    GrainyDots,
    CoarseDots,
    ShortLines,
    MediumLines,
    LongLines,
    ShortStrokes,
    MediumStrokes,
    LongStrokes,
}
impl MezzotintStyle {
    /// Stable UI choice order.
    pub const ALL: [Self; 10] = [
        Self::FineDots,
        Self::MediumDots,
        Self::GrainyDots,
        Self::CoarseDots,
        Self::ShortLines,
        Self::MediumLines,
        Self::LongLines,
        Self::ShortStrokes,
        Self::MediumStrokes,
        Self::LongStrokes,
    ];
}
/// Stochastic displacement or one edge-preserving diffusion step.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffuseMode {
    Normal,
    DarkenOnly,
    LightenOnly,
    Anisotropic,
}
impl DiffuseMode {
    /// Stable UI choice order.
    pub const ALL: [Self; 4] = [
        Self::Normal,
        Self::DarkenOnly,
        Self::LightenOnly,
        Self::Anisotropic,
    ];
}

/// Historical source center for projects saved before movable origins.
fn default_filter_center() -> [f32; 2] {
    [0.5; 2]
}

/// Independent radial wave directions, persisted by stable names.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZigZagStyle {
    AroundCenter,
    OutFromCenter,
    PondRipples,
}
impl ZigZagStyle {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 3] = [Self::AroundCenter, Self::OutFromCenter, Self::PondRipples];
}
impl Filter {
    /// Optional normalized source-local origin. Pixel-center zero is the first
    /// pixel; one is the last, including one-pixel axes (which map to zero).
    pub fn center(&self) -> Option<[f32; 2]> {
        match self {
            Self::Twirl { center, .. }
            | Self::Wave { center, .. }
            | Self::RadialBlur { center, .. }
            | Self::Pinch { center, .. }
            | Self::Spherize { center, .. }
            | Self::Ripple { center, .. }
            | Self::PolarCoordinates { center, .. }
            | Self::ZigZag { center, .. } => Some(*center),
            _ => None,
        }
    }
    pub(crate) fn center_mut(&mut self) -> Option<&mut [f32; 2]> {
        match self {
            Self::Twirl { center, .. }
            | Self::Wave { center, .. }
            | Self::RadialBlur { center, .. }
            | Self::Pinch { center, .. }
            | Self::Spherize { center, .. }
            | Self::Ripple { center, .. }
            | Self::PolarCoordinates { center, .. }
            | Self::ZigZag { center, .. } => Some(center),
            _ => None,
        }
    }
    /// Resolve an origin to the sampling grid without depending on a UI.
    pub fn source_center(&self, width: u32, height: u32) -> [f64; 2] {
        let c = self.center().unwrap_or([0.5; 2]);
        [
            f64::from(c[0]) * f64::from(width.saturating_sub(1)),
            f64::from(c[1]) * f64::from(height.saturating_sub(1)),
        ]
    }
}

/// Independently implemented grain patterns, serialized with stable names.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrainStyle {
    Regular,
    Soft,
    Speckle,
    Horizontal,
    Vertical,
}
impl GrainStyle {
    /// Stable order for the implemented pattern selector.
    pub const ALL: [Self; 5] = [
        Self::Regular,
        Self::Soft,
        Self::Speckle,
        Self::Horizontal,
        Self::Vertical,
    ];
}

/// Sampling outside source bounds; stable named values in native projects.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilterEdges {
    Wrap,
    Repeat,
    Transparent,
}
impl FilterEdges {
    /// Stable order for the common sampling choice control.
    pub const ALL: [Self; 3] = [Self::Wrap, Self::Repeat, Self::Transparent];
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
