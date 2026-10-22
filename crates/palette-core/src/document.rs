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

/// Stored stroke-axis choices, shared by ink and sprayed-color filters.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrokeDirection {
    RightDiagonal,
    LeftDiagonal,
    Horizontal,
    Vertical,
}
impl StrokeDirection {
    /// Conventional presentation order; wire values are names, not indices.
    pub const ALL: [Self; 4] = [
        Self::RightDiagonal,
        Self::LeftDiagonal,
        Self::Horizontal,
        Self::Vertical,
    ];
    /// Unit axis in source coordinates (positive y points down).
    pub(crate) fn vector(self) -> [f64; 2] {
        use std::f64::consts::FRAC_1_SQRT_2 as D;
        match self {
            Self::RightDiagonal => [D, D],
            Self::LeftDiagonal => [D, -D],
            Self::Horizontal => [1., 0.],
            Self::Vertical => [0., 1.],
        }
    }
}

/// Procedural paper/surface materials; no proprietary texture assets are used.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SurfaceTexture {
    Canvas,
    Brick,
    Burlap,
    Sandstone,
}
impl SurfaceTexture {
    /// Stable UI order; native serialization stores variant names.
    pub const ALL: [Self; 4] = [Self::Canvas, Self::Brick, Self::Burlap, Self::Sandstone];
}

/// Stable named options for the independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GlassSurface {
    Frosted,
    TinyLens,
    Blocks,
    Canvas,
}
impl GlassSurface {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 4] = [Self::Frosted, Self::TinyLens, Self::Blocks, Self::Canvas];
}
/// Stable named options for the independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApertureShape {
    Disk,
    Triangle,
    Square,
    Hexagon,
    Octagon,
}
impl ApertureShape {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 5] = [
        Self::Disk,
        Self::Triangle,
        Self::Square,
        Self::Hexagon,
        Self::Octagon,
    ];
}
/// Stable named options for the independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SmartBlurMode {
    Normal,
    EdgeOnly,
    OverlayEdge,
}
impl SmartBlurMode {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 3] = [Self::Normal, Self::EdgeOnly, Self::OverlayEdge];
}

/// Stable named options for an independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtrudeShape {
    Blocks,
    Pyramids,
}
impl ExtrudeShape {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 2] = [Self::Blocks, Self::Pyramids];
}
/// Stable named options for an independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileFill {
    Unaltered,
    Transparent,
    Foreground,
    Background,
}
impl TileFill {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 4] = [
        Self::Unaltered,
        Self::Transparent,
        Self::Foreground,
        Self::Background,
    ];
}
/// Stable named options for an independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindMethod {
    Wind,
    Blast,
    Stagger,
}
impl WindMethod {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 3] = [Self::Wind, Self::Blast, Self::Stagger];
}
/// Stable named options for an independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DepthSource {
    Uniform,
    Alpha,
    Luminance,
}
impl DepthSource {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 3] = [Self::Uniform, Self::Alpha, Self::Luminance];
}
/// Stable named options for an independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlareStyle {
    Zoom,
    Prime,
    Movie,
    Anamorphic,
}
impl FlareStyle {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 4] = [Self::Zoom, Self::Prime, Self::Movie, Self::Anamorphic];
}
/// Stable named options for an independently implemented filter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LightKind {
    Point,
    Spot,
    Directional,
}
impl LightKind {
    /// Stable parameter-choice order.
    pub const ALL: [Self; 3] = [Self::Point, Self::Spot, Self::Directional];
}

/// Embedded three-dimensional RGB lookup table, with red varying fastest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Lut3D {
    pub name: String,
    pub size: u8,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    pub values: Arc<Vec<[f32; 3]>>,
}
/// Stable named filter options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapFit {
    Stretch,
    Tile,
}
impl MapFit {
    /// Stable control order.
    pub const ALL: [Self; 2] = [Self::Stretch, Self::Tile];
}
/// Stable named filter options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShearInterpolation {
    Linear,
    Smooth,
}
impl ShearInterpolation {
    /// Stable control order.
    pub const ALL: [Self; 2] = [Self::Linear, Self::Smooth];
}

/// A normalized source-local field-blur control point.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct BlurPin {
    pub center: [f32; 2],
    pub radius: f32,
}
/// Stable named processing options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrameStyle {
    Plain,
    Beveled,
    Wood,
    Ornate,
}
impl FrameStyle {
    /// Stable control order.
    pub const ALL: [Self; 4] = [Self::Plain, Self::Beveled, Self::Wood, Self::Ornate];
}
/// Stable named processing options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TreeForm {
    Deciduous,
    Conifer,
    Bare,
}
impl TreeForm {
    /// Stable control order.
    pub const ALL: [Self; 3] = [Self::Deciduous, Self::Conifer, Self::Bare];
}
/// Stable named processing options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoField {
    Odd,
    Even,
}
impl VideoField {
    /// Stable control order.
    pub const ALL: [Self; 2] = [Self::Odd, Self::Even];
}
/// Stable named processing options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldInterpolation {
    Duplicate,
    Interpolate,
}
impl FieldInterpolation {
    /// Stable control order.
    pub const ALL: [Self; 2] = [Self::Duplicate, Self::Interpolate];
}
/// Stable named processing options.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelSpace {
    Rgb,
    Hsb,
    Hsl,
}
impl ChannelSpace {
    /// Stable control order.
    pub const ALL: [Self; 3] = [Self::Rgb, Self::Hsb, Self::Hsl];
}

/// Only implemented filters enter the model and the registry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Filter {
    FieldBlur {
        pins: [BlurPin; 8],
        pin: u8,
        count: u8,
    },
    IrisBlur {
        radius: f32,
        width: f32,
        height: f32,
        angle: f32,
        feather: f32,
        center: [f32; 2],
    },
    TiltShift {
        radius: f32,
        width: f32,
        transition: f32,
        angle: f32,
        center: [f32; 2],
    },
    PathBlur {
        distance: f32,
        taper: f32,
        centered: bool,
        path: [[f32; 2]; 3],
        point: u8,
    },
    SpinBlur {
        angle: f32,
        width: f32,
        height: f32,
        feather: f32,
        rotation: f32,
        center: [f32; 2],
    },
    Flame {
        width: f32,
        height: f32,
        turbulence: f32,
        detail: f32,
        seed: u16,
        color: [u8; 3],
        opacity: f32,
        angle: f32,
        center: [f32; 2],
    },
    PictureFrame {
        width: f32,
        margin: f32,
        bevel: f32,
        style: FrameStyle,
        color: [u8; 3],
        opacity: f32,
    },
    Tree {
        height: f32,
        width: f32,
        branching: u8,
        foliage: f32,
        seed: u16,
        form: TreeForm,
        leaf_color: [u8; 3],
        bark_color: [u8; 3],
        opacity: f32,
        angle: f32,
        center: [f32; 2],
    },
    NtscColors {
        setup: bool,
    },
    Deinterlace {
        field: VideoField,
        interpolation: FieldInterpolation,
    },
    HsbHsl {
        input: ChannelSpace,
        output: ChannelSpace,
    },
    ColorLookup {
        amount: f32,
        lut: Option<Lut3D>,
    },
    Displace {
        horizontal: f32,
        vertical: f32,
        fit: MapFit,
        edges: FilterEdges,
        center: [f32; 2],
        map: Option<Raster>,
        map_name: String,
    },
    Shear {
        interpolation: ShearInterpolation,
        edges: FilterEdges,
        center: [f32; 2],
        curve: [f32; 5],
    },
    Custom {
        divisor: f32,
        bias: f32,
        alpha: bool,
        edges: FilterEdges,
        kernel: [f32; 25],
    },

    Extrude {
        cell_size: u16,
        depth: f32,
        shape: ExtrudeShape,
        random: bool,
        seed: u16,
        center: [f32; 2],
    },
    OilPaint {
        radius: f32,
        stylization: f32,
        cleanliness: f32,
        scale: f32,
        bristles: f32,
        lighting: bool,
        angle: f32,
        shine: f32,
    },
    Tiles {
        tile_size: u16,
        offset: f32,
        fill: TileFill,
        seed: u16,
        low: [u8; 3],
        high: [u8; 3],
    },
    TraceContour {
        level: u8,
        upper: bool,
    },
    Wind {
        length: f32,
        method: WindMethod,
        from_right: bool,
        seed: u16,
    },
    LensBlur {
        radius: f32,
        shape: ApertureShape,
        angle: f32,
        depth_source: DepthSource,
        focus: f32,
        invert: bool,
        highlights: f32,
        threshold: f32,
        noise: f32,
        seed: u16,
    },
    LensFlare {
        brightness: f32,
        radius: f32,
        style: FlareStyle,
        color: [u8; 3],
        center: [f32; 2],
    },
    LightingEffects {
        kind: LightKind,
        intensity: f32,
        ambient: f32,
        radius: f32,
        angle: f32,
        elevation: f32,
        gloss: f32,
        metallic: f32,
        relief: f32,
        color: [u8; 3],
        center: [f32; 2],
    },

    DiffuseGlow {
        amount: f32,
        clear: f32,
        radius: f32,
        grain: f32,
        seed: u16,
        color: [u8; 3],
        center: [f32; 2],
    },
    Glass {
        distortion: f32,
        smoothness: f32,
        scale: f32,
        texture: GlassSurface,
        invert: bool,
        seed: u16,
        edges: FilterEdges,
        center: [f32; 2],
    },
    OceanRipple {
        amplitude: f32,
        wavelength: f32,
        seed: u16,
        edges: FilterEdges,
        center: [f32; 2],
    },
    Texturizer {
        texture: SurfaceTexture,
        scale: f32,
        relief: f32,
        angle: f32,
        invert: bool,
        seed: u16,
    },
    ShapeBlur {
        radius: f32,
        shape: ApertureShape,
        angle: f32,
    },
    SmartBlur {
        radius: u8,
        threshold: f32,
        mode: SmartBlurMode,
    },
    SharpenEdges,
    SmartSharpen {
        radius: f32,
        amount: f32,
        reduce_noise: f32,
        iterations: u8,
    },

    Fresco {
        radius: f32,
        detail: f32,
        texture: f32,
        seed: u16,
    },
    NeonGlow {
        size: f32,
        brightness: f32,
        color: [u8; 3],
        low: [u8; 3],
        high: [u8; 3],
    },
    PaletteKnife {
        length: f32,
        detail: f32,
        smoothness: f32,
    },
    PlasticWrap {
        strength: f32,
        detail: f32,
        smoothness: f32,
    },
    RoughPastels {
        length: f32,
        detail: f32,
        texture: SurfaceTexture,
        scale: f32,
        relief: f32,
        angle: f32,
        invert: bool,
        seed: u16,
    },
    Sponge {
        radius: f32,
        definition: f32,
        smoothness: f32,
        seed: u16,
    },
    Underpainting {
        radius: f32,
        coverage: f32,
        texture: SurfaceTexture,
        scale: f32,
        relief: f32,
        angle: f32,
        invert: bool,
        seed: u16,
    },
    Watercolor {
        detail: f32,
        shadows: f32,
        texture: f32,
        seed: u16,
    },

    Chrome {
        detail: f32,
        smoothness: f32,
    },
    ConteCrayon {
        foreground: f32,
        background: f32,
        texture: SurfaceTexture,
        scale: f32,
        relief: f32,
        angle: f32,
        invert: bool,
        seed: u16,
        low: [u8; 3],
        high: [u8; 3],
    },
    Plaster {
        balance: f32,
        smoothness: f32,
        angle: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    Reticulation {
        density: f32,
        foreground: f32,
        background: f32,
        grain_size: f32,
        seed: u16,
        low: [u8; 3],
        high: [u8; 3],
    },
    TornEdges {
        balance: f32,
        smoothness: f32,
        contrast: f32,
        seed: u16,
        low: [u8; 3],
        high: [u8; 3],
    },
    WaterPaper {
        length: f32,
        brightness: f32,
        contrast: f32,
        seed: u16,
    },

    SprayedStrokes {
        length: f32,
        radius: f32,
        direction: StrokeDirection,
        seed: u16,
    },
    SumiE {
        width: f32,
        pressure: f32,
        contrast: f32,
    },
    BasRelief {
        detail: f32,
        smoothness: f32,
        angle: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    ChalkCharcoal {
        charcoal: f32,
        chalk: f32,
        pressure: f32,
        seed: u16,
        low: [u8; 3],
        high: [u8; 3],
    },
    Charcoal {
        width: f32,
        detail: f32,
        balance: f32,
        low: [u8; 3],
        high: [u8; 3],
    },
    GraphicPen {
        length: f32,
        balance: f32,
        direction: StrokeDirection,
        low: [u8; 3],
        high: [u8; 3],
    },
    NotePaper {
        balance: f32,
        grain: f32,
        relief: f32,
        seed: u16,
        low: [u8; 3],
        high: [u8; 3],
    },
    Stamp {
        balance: f32,
        smoothness: f32,
        low: [u8; 3],
        high: [u8; 3],
    },

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
    /// Logical bytes retained by embedded filter assets, excluding shared copies.
    pub fn asset_bytes(&self) -> usize {
        match self {
            Self::Displace { map: Some(m), .. } => m.byte_len(),
            Self::ColorLookup { lut: Some(l), .. } => l.values.len() * 12,
            _ => 0,
        }
    }

    /// Optional normalized source-local origin. Pixel-center zero is the first
    /// pixel; one is the last, including one-pixel axes (which map to zero).
    pub fn center(&self) -> Option<[f32; 2]> {
        match self {
            Self::FieldBlur { pins, pin, .. } => pins.get(*pin as usize).map(|p| p.center),
            Self::PathBlur { path, point, .. } => path.get(*point as usize).copied(),
            Self::IrisBlur { center, .. }
            | Self::TiltShift { center, .. }
            | Self::SpinBlur { center, .. }
            | Self::Flame { center, .. }
            | Self::Tree { center, .. }
            | Self::DiffuseGlow { center, .. }
            | Self::Glass { center, .. }
            | Self::OceanRipple { center, .. }
            | Self::Extrude { center, .. }
            | Self::LensFlare { center, .. }
            | Self::LightingEffects { center, .. }
            | Self::Displace { center, .. }
            | Self::Shear { center, .. }
            | Self::Twirl { center, .. }
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
            Self::FieldBlur { pins, pin, .. } => pins.get_mut(*pin as usize).map(|p| &mut p.center),
            Self::PathBlur { path, point, .. } => path.get_mut(*point as usize),
            Self::IrisBlur { center, .. }
            | Self::TiltShift { center, .. }
            | Self::SpinBlur { center, .. }
            | Self::Flame { center, .. }
            | Self::Tree { center, .. }
            | Self::DiffuseGlow { center, .. }
            | Self::Glass { center, .. }
            | Self::OceanRipple { center, .. }
            | Self::Extrude { center, .. }
            | Self::LensFlare { center, .. }
            | Self::LightingEffects { center, .. }
            | Self::Displace { center, .. }
            | Self::Shear { center, .. }
            | Self::Twirl { center, .. }
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BevelStyle {
    Inner,
    Outer,
    Emboss,
    Pillow,
    Stroke,
}
impl BevelStyle {
    pub const ALL: [Self; 5] = [
        Self::Inner,
        Self::Outer,
        Self::Emboss,
        Self::Pillow,
        Self::Stroke,
    ];
}
/// Bounded analytic relief contours, independent of proprietary preset assets.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StyleContour {
    Linear,
    Rounded,
    Ring,
    Cone,
}
impl StyleContour {
    pub const ALL: [Self; 4] = [Self::Linear, Self::Rounded, Self::Ring, Self::Cone];
}
/// Source-relative gradient geometry; centers stay normalized when resized.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GradientStyle {
    Linear,
    Radial,
    Angle,
    Reflected,
    Diamond,
}
impl GradientStyle {
    pub const ALL: [Self; 5] = [
        Self::Linear,
        Self::Radial,
        Self::Angle,
        Self::Reflected,
        Self::Diamond,
    ];
}
/// Original procedural layer patterns. No third-party pattern assets are used.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LayerPattern {
    Checker,
    Stripes,
    Dots,
    Weave,
}
impl LayerPattern {
    pub const ALL: [Self; 4] = [Self::Checker, Self::Stripes, Self::Dots, Self::Weave];
}

/// Live styles retain all parameters and never overwrite layer source pixels.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Effect {
    /// Extended controls decorate an existing effect. Legacy variants keep
    /// their original rendering until the user changes an extended setting.
    Configured {
        effect: Box<Effect>,
        options: Box<StyleOptions>,
    },
    BevelEmboss {
        style: BevelStyle,
        depth: f32,
        size: f32,
        soften: f32,
        angle: f32,
        altitude: f32,
        down: bool,
        highlight: [u8; 3],
        highlight_opacity: f32,
        shadow: [u8; 3],
        shadow_opacity: f32,
        contour: StyleContour,
        texture: bool,
        surface: SurfaceTexture,
        texture_scale: f32,
        texture_depth: f32,
    },
    Satin {
        color: [u8; 3],
        opacity: f32,
        angle: f32,
        distance: f32,
        size: f32,
        contour: StyleContour,
        invert: bool,
    },
    GradientOverlay {
        start_color: [u8; 3],
        end_color: [u8; 3],
        opacity: f32,
        style: GradientStyle,
        angle: f32,
        scale: f32,
        reverse: bool,
        center: [f32; 2],
    },
    PatternOverlay {
        color: [u8; 3],
        background: [u8; 3],
        opacity: f32,
        pattern: LayerPattern,
        tile_size: f32,
        angle: f32,
        offset: [f32; 2],
    },
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

impl Effect {
    /// The original effect family, without its optional extended controls.
    pub fn base(&self) -> &Self {
        match self {
            Self::Configured { effect, .. } => effect,
            _ => self,
        }
    }
    /// Mutable original settings, retaining the extended controls alongside them.
    pub fn base_mut(&mut self) -> &mut Self {
        match self {
            Self::Configured { effect, .. } => effect,
            _ => self,
        }
    }
    /// Extended controls, if the instance has been explicitly customized.
    pub fn options(&self) -> Option<&StyleOptions> {
        match self {
            Self::Configured { options, .. } => Some(options),
            _ => None,
        }
    }
    /// Upgrade an effect without replacing or reinterpreting its old settings.
    pub fn options_mut(&mut self) -> &mut StyleOptions {
        if !matches!(self, Self::Configured { .. }) {
            *self = Self::Configured {
                effect: Box::new(self.clone()),
                options: Box::default(),
            };
        }
        match self {
            Self::Configured { options, .. } => options,
            _ => unreachable!(),
        }
    }
}

/// An editable gradient stop with straight alpha and an outgoing midpoint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StyleStop {
    pub position: f32,
    pub color: [u8; 3],
    pub opacity: f32,
    pub midpoint: f32,
}
/// Original user-supplied pattern pixels; projects embed these losslessly.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StylePattern {
    pub name: String,
    pub pixels: crate::raster::Raster,
}
/// Family-specific extended controls. A single wrapper avoids changing the
/// meaning of legacy serialized variants. Parameter registries expose only the
/// settings meaningful for that family; vectors/assets are independently bounded.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct StyleOptions {
    pub blend: BlendMode,
    pub highlight_blend: BlendMode,
    pub shadow_blend: BlendMode,
    pub technique: usize,
    pub contour: Vec<[f32; 2]>,
    pub contour_enabled: bool,
    pub gloss: Vec<[f32; 2]>,
    pub gloss_enabled: bool,
    pub gloss_antialias: bool,
    pub contour_antialias: bool,
    pub range: f32,
    pub noise: f32,
    pub jitter: f32,
    pub seed: u32,
    pub inner_center: bool,
    pub knockout: bool,
    pub use_global_light: bool,
    pub global_light: [f32; 2],
    pub fill: usize,
    pub stops: Vec<StyleStop>,
    pub gradient_method: usize,
    pub gradient_smoothness: f32,
    pub material_style: usize,
    pub material_reverse: bool,
    pub dither: bool,
    pub align: bool,
    pub material_scale: f32,
    pub material_angle: f32,
    pub material_offset: [f32; 2],
    pub pattern: Option<StylePattern>,
    pub texture_invert: bool,
    pub texture_link: bool,
    pub texture_offset: [f32; 2],
    /// Resolved document-space phase participates in render-cache identity.
    #[serde(skip)]
    pub resolved_origin: [f32; 2],
    #[serde(skip)]
    pub resolved_size: [f32; 2],
}
impl Default for StyleOptions {
    fn default() -> Self {
        Self {
            blend: BlendMode::Normal,
            highlight_blend: BlendMode::Normal,
            shadow_blend: BlendMode::Normal,
            technique: 0,
            contour: vec![[0., 0.], [1., 1.]],
            contour_enabled: false,
            gloss: vec![[0., 0.], [1., 1.]],
            gloss_enabled: false,
            gloss_antialias: true,
            contour_antialias: true,
            range: 1.,
            noise: 0.,
            jitter: 0.,
            seed: 0,
            inner_center: false,
            knockout: false,
            use_global_light: false,
            global_light: [120., 35.],
            fill: 0,
            stops: vec![],
            gradient_method: 0,
            gradient_smoothness: 0.,
            material_style: 0,
            material_reverse: false,
            dither: false,
            align: true,
            material_scale: 100.,
            material_angle: 90.,
            material_offset: [0.; 2],
            pattern: None,
            texture_invert: false,
            texture_link: true,
            texture_offset: [0.; 2],
            resolved_origin: [0.; 2],
            resolved_size: [0.; 2],
        }
    }
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
    /// Shared style lighting: azimuth and altitude in degrees.
    #[serde(default = "default_global_light")]
    pub global_light: [f32; 2],
    pub width: u32,
    pub height: u32,
    /// Canvas backing color, separate from editable raster layers (HEIF iovl).
    #[serde(default)]
    pub background: [u8; 4],
    /// Visibility is independent of the backing's original RGBA, including alpha.
    #[serde(default = "default_background_visible")]
    pub background_visible: bool,
    pub layers: Vec<Layer>,
    pub active_layer: u64,
    pub selection: Option<Coverage>,
    /// Original selection contour retained separately from feathered editing
    /// coverage. Shared immutable data participates in history and native saving.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_base: Option<Coverage>,
    /// Temporary shape adjustment, retained in undo snapshots. `Layer::source`
    /// holds its composited display, while the original and shape pixels stay
    /// separate until commitment. Native files store the committed raster copy.
    #[serde(skip)]
    pub floating_shape: Option<Box<crate::floating::Shape>>,
    pub(crate) next_id: u64,
    /// Opaque source metadata retained by native projects. Pixels currently use
    /// the editor's RGBA8/sRGB working interpretation; profiles are not applied.
    #[serde(default)]
    pub metadata: DocumentMetadata,
}

fn default_background_visible() -> bool {
    true
}
fn default_global_light() -> [f32; 2] {
    [120., 35.]
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
            for e in &layer.effects {
                if let Some(p) = e.effect.options().and_then(|o| o.pattern.as_ref()) {
                    p.pixels.allocations(&mut output);
                }
            }
            for f in &layer.filters {
                match &f.filter {
                    Filter::Displace { map: Some(map), .. } => map.allocations(&mut output),
                    Filter::ColorLookup { lut: Some(lut), .. } => {
                        output.insert(
                            Arc::as_ptr(&lut.values) as usize,
                            lut.values.capacity() * 12,
                        );
                    }
                    _ => {}
                }
            }
        }
        if let Some(shape) = &self.floating_shape {
            shape.base.allocations(&mut output);
            shape.pixels.allocations(&mut output);
        }
        output
    }
    /// Create a white default background of the requested bounded dimensions.
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let source = Raster::new(width, height, [0; 4])?;
        let mut document = Self::from_raster(source);
        document.background = [255; 4];
        document.active_mut().name = "Layer 1".into();
        Ok(document)
    }

    /// Wrap a raster as an editable single-layer document.
    pub fn from_raster(source: Raster) -> Self {
        Self {
            global_light: default_global_light(),
            width: source.width(),
            height: source.height(),
            background: [0; 4],
            background_visible: true,
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
            floating_shape: None,
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

    /// Effective canvas backing used by composition and compatible exports.
    pub fn backing_color(&self) -> [u8; 4] {
        if self.background_visible {
            self.background
        } else {
            [0; 4]
        }
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
        self.floating_shape = None;
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
        self.floating_shape = None;
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
        self.floating_shape = None;
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
                        .map(|f| {
                            f.mask.as_ref().map_or(0, |m| m.values.len()) + f.filter.asset_bytes()
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
            + self.selection.as_ref().map_or(0, |m| m.values.len())
            + self.selection_base.as_ref().map_or(0, |m| m.values.len())
            + self
                .floating_shape
                .as_ref()
                .map_or(0, |s| s.base.byte_len() + s.pixels.byte_len())
            + self.metadata.icc_profile.len()
            + self.metadata.exif.len()
    }

    /// Validate all serialized invariants before any indexing or rendering.
    pub fn validate(&self) -> Result<(), String> {
        if !self.global_light[0].is_finite()
            || !(-180. ..=180.).contains(&self.global_light[0])
            || !self.global_light[1].is_finite()
            || !(0. ..=90.).contains(&self.global_light[1])
        {
            return Err("Invalid document global light".into());
        }
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
                    let planes = if let Effect::BevelEmboss { style, .. } = effect.effect.base() {
                        if matches!(style, BevelStyle::Inner | BevelStyle::Outer) {
                            2
                        } else {
                            4
                        }
                    } else if matches!(
                        effect.effect.base(),
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
        if let Some(shape) = &self.floating_shape {
            shape.base.validate()?;
            shape.pixels.validate()?;
            let source = &self.active().source;
            if shape.layer != self.active_layer
                || shape.base.width() != source.width()
                || shape.base.height() != source.height()
                || shape.pixels.width() != source.width()
                || shape.pixels.height() != source.height()
            {
                return Err("Floating shape must match the active source geometry".into());
            }
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
