//! Implemented processing catalog and shared, UI-independent parameter metadata.
//! The renderer never knows widget types; adding an operation extends this
//! registry and its evaluator rather than introducing a second UI parameter model.
mod adjustments;
mod artistic;
mod cellular;
mod cinematic;
mod color_space;
mod conventional;
mod creative;
mod cutout;
mod distort;
mod drawing;
pub mod editable;
mod extended;
mod gallery;
mod gaussian;
mod generators;
mod layer;
mod media;
pub mod menu;
mod neighborhood;
mod optical;
mod painting;
mod pixelate;
mod point;
mod retouch;
mod round;
mod sketch;
mod studio;
pub mod style_options;
mod styles;
mod stylize;
mod video;
use crate::document::{
    ApertureShape, BlurPin, ChannelSpace, DaubStyle, DepthSource, DiffuseMode, Effect,
    ExtrudeShape, FieldInterpolation, Filter, FilterEdges, FlareStyle, FrameStyle, GlassSurface,
    GrainStyle, HalftoneStyle, LightKind, MapFit, MezzotintStyle, NoiseDistribution,
    ShearInterpolation, SmartBlurMode, StrokeDirection, StrokePosition, SurfaceTexture, TileFill,
    TreeForm, VideoField, WindMethod, ZigZagStyle,
};
pub use extended::{high_pass_cancel, mosaic_cancel, motion_blur_cancel};
pub use gaussian::{gaussian_blur, gaussian_blur_cancel};
pub use layer::{
    Appearance, PreparedLayer, has_neighborhood, input_byte_len, prepare_layer,
    prepare_layer_cancel, same_appearance,
};
pub use neighborhood::{box_blur_cancel, median_cancel};
pub(crate) use point::point_filter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Stable identifiers for the implemented processing catalog.
pub enum Kind {
    BevelEmboss,
    Satin,
    GradientOverlay,
    PatternOverlay,
    FieldBlur,
    IrisBlur,
    TiltShift,
    PathBlur,
    SpinBlur,
    Flame,
    PictureFrame,
    Tree,
    NtscColors,
    Deinterlace,
    HsbHsl,

    ColorLookup,
    Displace,
    Shear,
    Custom,

    Extrude,
    OilPaint,
    Tiles,
    TraceContour,
    Wind,
    LensBlur,
    LensFlare,
    LightingEffects,

    DiffuseGlow,
    Glass,
    OceanRipple,
    Texturizer,
    ShapeBlur,
    SmartBlur,
    SharpenEdges,
    SmartSharpen,

    Fresco,
    NeonGlow,
    PaletteKnife,
    PlasticWrap,
    RoughPastels,
    Sponge,
    Underpainting,
    Watercolor,

    Chrome,
    ConteCrayon,
    Plaster,
    Reticulation,
    TornEdges,
    WaterPaper,

    SprayedStrokes,
    SumiE,
    BasRelief,
    ChalkCharcoal,
    Charcoal,
    GraphicPen,
    NotePaper,
    Stamp,

    ColoredPencil,
    DryBrush,
    PaintDaubs,
    SmudgeStick,
    AngledStrokes,
    Crosshatch,
    DarkStrokes,
    Spatter,

    ColorBalance,
    PhotoFilter,
    ChannelMixer,
    GradientMap,
    SelectiveColor,
    ShadowsHighlights,
    Blur,
    BlurMore,

    Invert,
    Brightness,
    GaussianBlur,
    ColorOverlay,
    DropShadow,
    OuterGlow,
    BrightnessContrast,
    Exposure,
    HueSaturation,
    Vibrance,
    BlackWhite,
    Threshold,
    Posterize,
    BoxBlur,
    Median,
    InnerShadow,
    InnerGlow,
    Stroke,
    Levels,
    Curves,
    MotionBlur,
    HighPass,
    Mosaic,
    AddNoise,
    Average,
    FindEdges,
    Solarize,
    Emboss,
    UnsharpMask,
    Minimum,
    Maximum,
    Offset,
    Twirl,
    Wave,
    SurfaceBlur,
    RadialBlur,
    Clouds,
    DifferenceClouds,
    PosterEdges,
    Grain,
    Fragment,
    Despeckle,
    DustScratches,
    ReduceNoise,
    Pinch,
    Spherize,
    Ripple,
    Sharpen,
    SharpenMore,
    PolarCoordinates,
    ZigZag,
    Fibers,
    GlowingEdges,
    Cutout,
    FilmGrain,
    AccentedEdges,
    InkOutlines,
    Photocopy,
    HalftonePattern,
    Patchwork,
    Craquelure,
    ColorHalftone,
    Crystallize,
    Facet,
    Mezzotint,
    Pointillize,
    StainedGlass,
    MosaicTiles,
    Diffuse,
}
#[derive(Clone, Copy, Debug)]
/// UI-independent input constraints, shared by model validation and controls.
pub enum ParameterType {
    Number { min: f32, max: f32, step: f32 },
    Color,
    Boolean,
    Choice(&'static [&'static str]),
}
#[derive(Clone, Copy, Debug)]
/// A named operation parameter and its accepted input domain.
pub struct Parameter {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ParameterType,
}
#[derive(Clone, Copy, Debug)]
/// Catalog metadata for an operation that has a real renderer implementation.
pub struct Descriptor {
    pub kind: Kind,
    pub label: &'static str,
    pub category: &'static str,
    pub parameters: &'static [Parameter],
}
const fn number(
    key: &'static str,
    label: &'static str,
    min: f32,
    max: f32,
    step: f32,
) -> Parameter {
    Parameter {
        key,
        label,
        kind: ParameterType::Number { min, max, step },
    }
}
const COLOR: Parameter = Parameter {
    key: "color",
    label: "Color",
    kind: ParameterType::Color,
};
const OPACITY: Parameter = number("opacity", "Opacity", 0., 1., 0.01);
const SPREAD: Parameter = number("spread", "Spread", 0., 1., 0.01);
const EDGES: Parameter = Parameter {
    key: "edges",
    label: "Undefined areas",
    kind: ParameterType::Choice(&["Wrap around", "Repeat edge pixels", "Transparent"]),
};
const CENTER_X: Parameter = number("center_x", "Origin X", 0., 1., 0.001);
const CENTER_Y: Parameter = number("center_y", "Origin Y", 0., 1., 0.001);
const SIZE: Parameter = number("size", "Size (px)", 0., 64., 0.1);
/// Implemented operations only; unfinished catalog entries are never registered.
pub const REGISTRY: &[Descriptor] = &[
    styles::BEVEL,
    styles::SATIN,
    styles::GRADIENT,
    styles::PATTERN,
    Descriptor {
        kind: Kind::ColorBalance,
        label: "Color Balance",
        category: "Adjustments",
        parameters: &[
            number("shadow_red", "Shad. Cyan–Red", -1.0, 1.0, 0.01),
            number("shadow_green", "Shad. Mag.–Green", -1.0, 1.0, 0.01),
            number("shadow_blue", "Shad. Yellow–Blue", -1.0, 1.0, 0.01),
            number("midtone_red", "Mid. Cyan–Red", -1.0, 1.0, 0.01),
            number("midtone_green", "Mid. Mag.–Green", -1.0, 1.0, 0.01),
            number("midtone_blue", "Mid. Yellow–Blue", -1.0, 1.0, 0.01),
            number("highlight_red", "High. Cyan–Red", -1.0, 1.0, 0.01),
            number("highlight_green", "High. Mag.–Green", -1.0, 1.0, 0.01),
            number("highlight_blue", "High. Yellow–Blue", -1.0, 1.0, 0.01),
            Parameter {
                key: "preserve_luminosity",
                label: "Preserve luminosity",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::PhotoFilter,
        label: "Photo Filter",
        category: "Adjustments",
        parameters: &[
            COLOR,
            number("density", "Density", 0.0, 1.0, 0.01),
            Parameter {
                key: "preserve_luminosity",
                label: "Preserve luminosity",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::ChannelMixer,
        label: "Channel Mixer",
        category: "Adjustments",
        parameters: &[
            number("red_red", "Red <- Red", -2.0, 2.0, 0.01),
            number("red_green", "Red <- Green", -2.0, 2.0, 0.01),
            number("red_blue", "Red <- Blue", -2.0, 2.0, 0.01),
            number("red_constant", "Red <- Constant", -1.0, 1.0, 0.01),
            number("green_red", "Green <- Red", -2.0, 2.0, 0.01),
            number("green_green", "Green <- Green", -2.0, 2.0, 0.01),
            number("green_blue", "Green <- Blue", -2.0, 2.0, 0.01),
            number("green_constant", "Green <- Constant", -1.0, 1.0, 0.01),
            number("blue_red", "Blue <- Red", -2.0, 2.0, 0.01),
            number("blue_green", "Blue <- Green", -2.0, 2.0, 0.01),
            number("blue_blue", "Blue <- Blue", -2.0, 2.0, 0.01),
            number("blue_constant", "Blue <- Constant", -1.0, 1.0, 0.01),
            Parameter {
                key: "monochrome",
                label: "Monochrome",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::GradientMap,
        label: "Gradient Map",
        category: "Adjustments",
        parameters: &[
            Parameter {
                key: "low",
                label: "Shadow color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Highlight color",
                kind: ParameterType::Color,
            },
            number("midpoint", "Midpoint", 0.01, 0.99, 0.01),
            Parameter {
                key: "reverse",
                label: "Reverse",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::SelectiveColor,
        label: "Selective Color",
        category: "Adjustments",
        parameters: &[
            number("reds_cyan", "Reds · Cyan", -1.0, 1.0, 0.01),
            number("reds_magenta", "Reds · Magenta", -1.0, 1.0, 0.01),
            number("reds_yellow", "Reds · Yellow", -1.0, 1.0, 0.01),
            number("reds_black", "Reds · Black", -1.0, 1.0, 0.01),
            number("yellows_cyan", "Yellows · Cyan", -1.0, 1.0, 0.01),
            number("yellows_magenta", "Yellows · Magenta", -1.0, 1.0, 0.01),
            number("yellows_yellow", "Yellows · Yellow", -1.0, 1.0, 0.01),
            number("yellows_black", "Yellows · Black", -1.0, 1.0, 0.01),
            number("greens_cyan", "Greens · Cyan", -1.0, 1.0, 0.01),
            number("greens_magenta", "Greens · Magenta", -1.0, 1.0, 0.01),
            number("greens_yellow", "Greens · Yellow", -1.0, 1.0, 0.01),
            number("greens_black", "Greens · Black", -1.0, 1.0, 0.01),
            number("cyans_cyan", "Cyans · Cyan", -1.0, 1.0, 0.01),
            number("cyans_magenta", "Cyans · Magenta", -1.0, 1.0, 0.01),
            number("cyans_yellow", "Cyans · Yellow", -1.0, 1.0, 0.01),
            number("cyans_black", "Cyans · Black", -1.0, 1.0, 0.01),
            number("blues_cyan", "Blues · Cyan", -1.0, 1.0, 0.01),
            number("blues_magenta", "Blues · Magenta", -1.0, 1.0, 0.01),
            number("blues_yellow", "Blues · Yellow", -1.0, 1.0, 0.01),
            number("blues_black", "Blues · Black", -1.0, 1.0, 0.01),
            number("magentas_cyan", "Magentas · Cyan", -1.0, 1.0, 0.01),
            number("magentas_magenta", "Magentas · Magenta", -1.0, 1.0, 0.01),
            number("magentas_yellow", "Magentas · Yellow", -1.0, 1.0, 0.01),
            number("magentas_black", "Magentas · Black", -1.0, 1.0, 0.01),
            number("whites_cyan", "Whites · Cyan", -1.0, 1.0, 0.01),
            number("whites_magenta", "Whites · Magenta", -1.0, 1.0, 0.01),
            number("whites_yellow", "Whites · Yellow", -1.0, 1.0, 0.01),
            number("whites_black", "Whites · Black", -1.0, 1.0, 0.01),
            number("neutrals_cyan", "Neutrals · Cyan", -1.0, 1.0, 0.01),
            number("neutrals_magenta", "Neutrals · Magenta", -1.0, 1.0, 0.01),
            number("neutrals_yellow", "Neutrals · Yellow", -1.0, 1.0, 0.01),
            number("neutrals_black", "Neutrals · Black", -1.0, 1.0, 0.01),
            number("blacks_cyan", "Blacks · Cyan", -1.0, 1.0, 0.01),
            number("blacks_magenta", "Blacks · Magenta", -1.0, 1.0, 0.01),
            number("blacks_yellow", "Blacks · Yellow", -1.0, 1.0, 0.01),
            number("blacks_black", "Blacks · Black", -1.0, 1.0, 0.01),
            Parameter {
                key: "relative",
                label: "Relative",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::ShadowsHighlights,
        label: "Shadows/Highlights",
        category: "Adjustments",
        parameters: &[
            number("shadows", "Shadows", 0.0, 1.0, 0.01),
            number("shadow_width", "Shadow tone", 0.01, 1.0, 0.01),
            number("shadow_radius", "Shadow radius (px)", 0.0, 64.0, 0.1),
            number("highlights", "Highlights", 0.0, 1.0, 0.01),
            number("highlight_width", "Highlight tone", 0.01, 1.0, 0.01),
            number("highlight_radius", "Highlight radius (px)", 0.0, 64.0, 0.1),
        ],
    },
    Descriptor {
        kind: Kind::Blur,
        label: "Blur",
        category: "Blur",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::BlurMore,
        label: "Blur More",
        category: "Blur",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::Invert,
        label: "Invert",
        category: "Adjustments",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::Brightness,
        label: "Brightness",
        category: "Adjustments",
        parameters: &[number("amount", "Amount", -1., 1., 0.01)],
    },
    Descriptor {
        kind: Kind::GaussianBlur,
        label: "Gaussian Blur",
        category: "Blur",
        parameters: &[number("radius", "Radius (px)", 0., 64., 0.1)],
    },
    Descriptor {
        kind: Kind::ColorOverlay,
        label: "Color Overlay",
        category: "Layer styles",
        parameters: &[COLOR, OPACITY, style_options::BLEND],
    },
    Descriptor {
        kind: Kind::DropShadow,
        label: "Drop Shadow",
        category: "Layer styles",
        parameters: &[
            COLOR,
            OPACITY,
            number("angle", "Light angle (°)", -180., 180., 1.),
            number("distance", "Distance (px)", 0., 256., 0.1),
            SPREAD,
            SIZE,
            style_options::BLEND,
            style_options::CONTOUR,
            style_options::AA,
            style_options::NOISE,
            style_options::SEED,
            style_options::GLOBAL,
            style_options::GLOBAL_ANGLE,
            Parameter {
                key: "shadow_knockout",
                label: "Layer knocks out shadow",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::OuterGlow,
        label: "Outer Glow",
        category: "Layer styles",
        parameters: &[
            COLOR,
            OPACITY,
            SPREAD,
            number("radius", "Radius (px)", 0., 64., 0.1),
            style_options::BLEND,
            style_options::GRADIENT_FILL,
            style_options::SOFTER,
            style_options::CONTOUR,
            style_options::AA,
            style_options::RANGE,
            style_options::NOISE,
            style_options::JITTER,
            style_options::SEED,
            style_options::METHOD,
            style_options::SMOOTHNESS,
        ],
    },
    Descriptor {
        kind: Kind::BrightnessContrast,
        label: "Brightness/Contrast",
        category: "Adjustments",
        parameters: &[
            number("brightness", "Brightness", -1., 1., 0.01),
            number("contrast", "Contrast", -0.99, 0.99, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Exposure,
        label: "Exposure",
        category: "Adjustments",
        parameters: &[number("stops", "Exposure (EV)", -8., 8., 0.01)],
    },
    Descriptor {
        kind: Kind::HueSaturation,
        label: "Hue/Saturation",
        category: "Adjustments",
        parameters: &[
            number("hue", "Hue (°)", -180., 180., 1.),
            number("saturation", "Saturation", -1., 1., 0.01),
            number("lightness", "Lightness", -1., 1., 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Vibrance,
        label: "Vibrance",
        category: "Adjustments",
        parameters: &[number("amount", "Amount", -1., 1., 0.01)],
    },
    Descriptor {
        kind: Kind::BlackWhite,
        label: "Black & White",
        category: "Adjustments",
        parameters: &[number("strength", "Strength", 0., 1., 0.01)],
    },
    Descriptor {
        kind: Kind::Threshold,
        label: "Threshold",
        category: "Adjustments",
        parameters: &[number("level", "Threshold", 0., 255., 1.)],
    },
    Descriptor {
        kind: Kind::Posterize,
        label: "Posterize",
        category: "Adjustments",
        parameters: &[number("levels", "Levels", 2., 32., 1.)],
    },
    Descriptor {
        kind: Kind::BoxBlur,
        label: "Box Blur",
        category: "Blur",
        parameters: &[number("radius", "Radius (px)", 0., 32., 1.)],
    },
    Descriptor {
        kind: Kind::Median,
        label: "Median",
        category: "Noise",
        parameters: &[number("radius", "Radius (px)", 0., 5., 1.)],
    },
    Descriptor {
        kind: Kind::InnerShadow,
        label: "Inner Shadow",
        category: "Layer styles",
        parameters: &[
            COLOR,
            number("opacity", "Opacity", 0., 1., 0.01),
            number("angle", "Light angle (°)", -180., 180., 1.),
            number("distance", "Distance (px)", 0., 256., 0.1),
            number("spread", "Spread", 0., 1., 0.01),
            number("size", "Size (px)", 0., 64., 0.1),
            style_options::BLEND,
            style_options::CONTOUR,
            style_options::AA,
            style_options::NOISE,
            style_options::SEED,
            style_options::GLOBAL,
            style_options::GLOBAL_ANGLE,
        ],
    },
    Descriptor {
        kind: Kind::InnerGlow,
        label: "Inner Glow",
        category: "Layer styles",
        parameters: &[
            COLOR,
            number("opacity", "Opacity", 0., 1., 0.01),
            number("spread", "Spread", 0., 1., 0.01),
            number("radius", "Radius (px)", 0., 64., 0.1),
            style_options::BLEND,
            style_options::GRADIENT_FILL,
            style_options::SOFTER,
            style_options::CONTOUR,
            style_options::AA,
            style_options::RANGE,
            style_options::NOISE,
            style_options::JITTER,
            style_options::SEED,
            style_options::METHOD,
            style_options::SMOOTHNESS,
            Parameter {
                key: "inner_center",
                label: "Glow from center",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::Stroke,
        label: "Stroke",
        category: "Layer styles",
        parameters: &[
            COLOR,
            number("opacity", "Opacity", 0., 1., 0.01),
            number("width", "Width (px)", 0., 64., 0.1),
            Parameter {
                key: "position",
                label: "Position",
                kind: ParameterType::Choice(&["Outside", "Inside", "Center"]),
            },
            Parameter {
                key: "rounded",
                label: "Rounded corners",
                kind: ParameterType::Boolean,
            },
            style_options::BLEND,
            style_options::FILL,
            Parameter {
                key: "material_style",
                label: "Gradient style",
                kind: ParameterType::Choice(&[
                    "Linear",
                    "Radial",
                    "Angle",
                    "Reflected",
                    "Diamond",
                    "Shape burst",
                ]),
            },
            Parameter {
                key: "material_reverse",
                label: "Reverse gradient",
                kind: ParameterType::Boolean,
            },
            style_options::METHOD,
            style_options::SMOOTHNESS,
            style_options::DITHER,
            style_options::ALIGN,
            style_options::MATERIAL_SCALE,
            style_options::MATERIAL_ANGLE,
            number("material_x", "Fill X", -4096., 4096., 0.5),
            number("material_y", "Fill Y", -4096., 4096., 0.5),
        ],
    },
    Descriptor {
        kind: Kind::Levels,
        label: "Levels",
        category: "Adjustments",
        parameters: &[
            number("black", "Input black", 0., 254., 1.),
            number("white", "Input white", 1., 255., 1.),
            number("gamma", "Gamma", 0.1, 9.99, 0.01),
            number("output_black", "Output black", 0., 255., 1.),
            number("output_white", "Output white", 0., 255., 1.),
        ],
    },
    Descriptor {
        kind: Kind::Curves,
        label: "Curves",
        category: "Adjustments",
        parameters: &[
            number("point0", "Black", 0., 255., 1.),
            number("point1", "Shadows", 0., 255., 1.),
            number("point2", "Midtones", 0., 255., 1.),
            number("point3", "Highlights", 0., 255., 1.),
            number("point4", "White", 0., 255., 1.),
        ],
    },
    Descriptor {
        kind: Kind::MotionBlur,
        label: "Motion Blur",
        category: "Blur",
        parameters: &[
            number("distance", "Distance (px)", 0., 64., 0.1),
            number("angle", "Angle (°)", -180., 180., 1.),
        ],
    },
    Descriptor {
        kind: Kind::HighPass,
        label: "High Pass",
        category: "Other",
        parameters: &[number("radius", "Radius (px)", 0., 64., 0.1)],
    },
    Descriptor {
        kind: Kind::Mosaic,
        label: "Mosaic",
        category: "Pixelate",
        parameters: &[number("block_size", "Cell size (px)", 1., 128., 1.)],
    },
    Descriptor {
        kind: Kind::AddNoise,
        label: "Add Noise",
        category: "Noise",
        parameters: &[
            number("strength", "Amount", 0., 1., 0.01),
            number("seed", "Seed", 0., 65535., 1.),
            Parameter {
                key: "monochrome",
                label: "Monochrome",
                kind: ParameterType::Boolean,
            },
            Parameter {
                key: "distribution",
                label: "Distribution",
                kind: ParameterType::Choice(&["Uniform", "Gaussian"]),
            },
        ],
    },
    Descriptor {
        kind: Kind::Average,
        label: "Average",
        category: "Blur",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::FindEdges,
        label: "Find Edges",
        category: "Stylize",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::Solarize,
        label: "Solarize",
        category: "Stylize",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::Emboss,
        label: "Emboss",
        category: "Stylize",
        parameters: &[
            number("angle", "Angle (°)", -180., 180., 1.),
            number("height", "Height (px)", 1., 16., 1.),
            number("strength", "Amount", 0., 5., 0.01),
        ],
    },
    Descriptor {
        kind: Kind::UnsharpMask,
        label: "Unsharp Mask",
        category: "Sharpen",
        parameters: &[
            number("strength", "Amount", 0., 5., 0.01),
            number("radius", "Radius (px)", 0., 64., 0.1),
            number("threshold", "Threshold", 0., 255., 1.),
        ],
    },
    Descriptor {
        kind: Kind::Minimum,
        label: "Minimum",
        category: "Other",
        parameters: &[number("radius", "Radius (px)", 0., 16., 1.)],
    },
    Descriptor {
        kind: Kind::Maximum,
        label: "Maximum",
        category: "Other",
        parameters: &[number("radius", "Radius (px)", 0., 16., 1.)],
    },
    Descriptor {
        kind: Kind::Offset,
        label: "Offset",
        category: "Other",
        parameters: &[
            number("x", "Horizontal (px)", -16384., 16384., 1.),
            number("y", "Vertical (px)", -16384., 16384., 1.),
            EDGES,
        ],
    },
    Descriptor {
        kind: Kind::Twirl,
        label: "Twirl",
        category: "Distort",
        parameters: &[
            number("angle", "Angle (°)", -720., 720., 1.),
            number("radius", "Radius (px)", 0., 4096., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Wave,
        label: "Wave",
        category: "Distort",
        parameters: &[
            number("amplitude", "Amplitude (px)", -128., 128., 0.1),
            number("wavelength", "Wavelength (px)", 1., 1024., 0.1),
            Parameter {
                key: "vertical",
                label: "Vertical",
                kind: ParameterType::Boolean,
            },
            EDGES,
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::SurfaceBlur,
        label: "Surface Blur",
        category: "Blur",
        parameters: &[
            number("radius", "Radius (px)", 0., 8., 1.),
            number("threshold", "Threshold", 0., 255., 1.),
        ],
    },
    Descriptor {
        kind: Kind::RadialBlur,
        label: "Radial Blur",
        category: "Blur",
        parameters: &[
            number("amount", "Amount", 0., 100., 0.1),
            number("samples", "Samples", 2., 64., 1.),
            Parameter {
                key: "zoom",
                label: "Method",
                kind: ParameterType::Choice(&["Spin", "Zoom"]),
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Clouds,
        label: "Clouds",
        category: "Render",
        parameters: &[
            number("seed", "Seed", 0., 65535., 1.),
            number("scale", "Scale (px)", 1., 1024., 0.1),
            Parameter {
                key: "low",
                label: "First color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Second color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::DifferenceClouds,
        label: "Difference Clouds",
        category: "Render",
        parameters: &[
            number("seed", "Seed", 0., 65535., 1.),
            number("scale", "Scale (px)", 1., 1024., 0.1),
            Parameter {
                key: "low",
                label: "First color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Second color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::PosterEdges,
        label: "Poster Edges",
        category: "Artistic",
        parameters: &[
            number("thickness", "Edge thickness (px)", 0., 4., 1.),
            number("intensity", "Edge intensity", 0., 1., 0.01),
            number("levels", "Levels", 2., 16., 1.),
        ],
    },
    Descriptor {
        kind: Kind::Grain,
        label: "Grain",
        category: "Texture",
        parameters: &[
            number("strength", "Amount", 0., 1., 0.01),
            number("contrast", "Contrast", 0., 1., 0.01),
            number("grain_size", "Grain size (px)", 0.5, 32., 0.1),
            number("seed", "Seed", 0., 65535., 1.),
            Parameter {
                key: "monochrome",
                label: "Monochrome",
                kind: ParameterType::Boolean,
            },
            Parameter {
                key: "style",
                label: "Grain type",
                kind: ParameterType::Choice(&[
                    "Regular",
                    "Soft",
                    "Speckle",
                    "Horizontal",
                    "Vertical",
                ]),
            },
        ],
    },
    Descriptor {
        kind: Kind::Fragment,
        label: "Fragment",
        category: "Pixelate",
        parameters: &[number("distance", "Distance (px)", 0., 32., 0.1)],
    },
    Descriptor {
        kind: Kind::Despeckle,
        label: "Despeckle",
        category: "Noise",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::DustScratches,
        label: "Dust & Scratches",
        category: "Noise",
        parameters: &[
            number("radius", "Radius (px)", 0., 5., 1.),
            number("threshold", "Threshold", 0., 255., 1.),
        ],
    },
    Descriptor {
        kind: Kind::ReduceNoise,
        label: "Reduce Noise",
        category: "Noise",
        parameters: &[
            number("strength", "Strength", 0., 1., 0.01),
            number("radius", "Radius (px)", 0., 4., 1.),
            number("preserve_details", "Preserve details", 0., 1., 0.01),
            number("color_noise", "Reduce color noise", 0., 1., 0.01),
            number("sharpen", "Sharpen details", 0., 1., 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Pinch,
        label: "Pinch",
        category: "Distort",
        parameters: &[
            number("strength", "Amount", -1., 1., 0.01),
            number("radius", "Radius (px)", 0., 4096., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Spherize,
        label: "Spherize",
        category: "Distort",
        parameters: &[
            number("strength", "Amount", -1., 1., 0.01),
            number("radius", "Radius (px)", 0., 4096., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Ripple,
        label: "Ripple",
        category: "Distort",
        parameters: &[
            number("amplitude", "Amount (px)", -64., 64., 0.1),
            number("wavelength", "Wavelength (px)", 1., 1024., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Sharpen,
        label: "Sharpen",
        category: "Sharpen",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::SharpenMore,
        label: "Sharpen More",
        category: "Sharpen",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::PolarCoordinates,
        label: "Polar Coordinates",
        category: "Distort",
        parameters: &[
            Parameter {
                key: "to_polar",
                label: "Conversion",
                kind: ParameterType::Choice(&["Rectangular to Polar", "Polar to Rectangular"]),
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::ZigZag,
        label: "ZigZag",
        category: "Distort",
        parameters: &[
            number("amount", "Amount (px)", -100., 100., 0.1),
            number("radius", "Radius (px)", 0., 4096., 0.1),
            number("ridges", "Ridges", 1., 20., 1.),
            Parameter {
                key: "style",
                label: "Style",
                kind: ParameterType::Choice(&["Around Center", "Out From Center", "Pond Ripples"]),
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Fibers,
        label: "Fibers",
        category: "Render",
        parameters: &[
            number("variance", "Variance", 1., 64., 0.1),
            number("strength", "Strength", 0., 1., 0.01),
            number("seed", "Seed", 0., 65535., 1.),
            Parameter {
                key: "low",
                label: "Foreground color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Background color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::GlowingEdges,
        label: "Glowing Edges",
        category: "Stylize",
        parameters: &[
            number("width", "Edge width (px)", 0.5, 10., 0.1),
            number("brightness", "Edge brightness", 0., 4., 0.01),
            number("smoothness", "Smoothness (px)", 0., 8., 0.1),
        ],
    },
    Descriptor {
        kind: Kind::Cutout,
        label: "Cutout",
        category: "Artistic",
        parameters: &[
            number("levels", "Number of levels", 2., 8., 1.),
            number("simplicity", "Edge simplicity (px)", 0., 8., 0.1),
            number("fidelity", "Edge fidelity", 0., 1., 0.01),
        ],
    },
    Descriptor {
        kind: Kind::FilmGrain,
        label: "Film Grain",
        category: "Artistic",
        parameters: &[
            number("grain", "Grain", 0., 1., 0.01),
            number("highlight", "Highlight area", 0., 1., 0.01),
            number("intensity", "Intensity", 0., 1., 0.01),
            number("seed", "Seed", 0., 65535., 1.),
        ],
    },
    Descriptor {
        kind: Kind::AccentedEdges,
        label: "Accented Edges",
        category: "Brush Strokes",
        parameters: &[
            number("width", "Edge width (px)", 0.5, 8., 0.1),
            number("brightness", "Edge brightness", 0., 1., 0.01),
            number("smoothness", "Smoothness (px)", 0., 4., 0.1),
        ],
    },
    Descriptor {
        kind: Kind::InkOutlines,
        label: "Ink Outlines",
        category: "Brush Strokes",
        parameters: &[
            number("length", "Stroke length (px)", 0.5, 12., 0.1),
            number("dark", "Dark intensity", 0., 1., 0.01),
            number("light", "Light intensity", 0., 1., 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Photocopy,
        label: "Photocopy",
        category: "Sketch",
        parameters: &[
            number("detail", "Detail", 0., 1., 0.01),
            number("darkness", "Darkness", 0., 1., 0.01),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::HalftonePattern,
        label: "Halftone Pattern",
        category: "Sketch",
        parameters: &[
            number("size", "Size (px)", 1., 64., 0.1),
            number("contrast", "Contrast", 0., 1., 0.01),
            Parameter {
                key: "pattern",
                label: "Pattern",
                kind: ParameterType::Choice(&["Dot", "Line", "Circle"]),
            },
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::Patchwork,
        label: "Patchwork",
        category: "Texture",
        parameters: &[
            number("square", "Square size (px)", 1., 64., 1.),
            number("relief", "Relief", 0., 32., 0.1),
        ],
    },
    Descriptor {
        kind: Kind::Craquelure,
        label: "Craquelure",
        category: "Texture",
        parameters: &[
            number("spacing", "Crack spacing (px)", 2., 128., 0.1),
            number("depth", "Crack depth", 0., 1., 0.01),
            number("brightness", "Crack brightness", 0., 1., 0.01),
            number("seed", "Seed", 0., 65535., 1.),
        ],
    },
    Descriptor {
        kind: Kind::ColorHalftone,
        label: "Color Halftone",
        category: "Pixelate",
        parameters: &[
            number("radius", "Maximum radius (px)", 0.5, 32., 0.1),
            number("cyan", "Cyan angle (°)", 0., 180., 1.),
            number("magenta", "Magenta angle (°)", 0., 180., 1.),
            number("yellow", "Yellow angle (°)", 0., 180., 1.),
            number("black", "Black angle (°)", 0., 180., 1.),
        ],
    },
    Descriptor {
        kind: Kind::Crystallize,
        label: "Crystallize",
        category: "Pixelate",
        parameters: &[
            number("cell_size", "Cell size (px)", 2., 128., 1.),
            number("seed", "Seed", 0., 65535., 1.),
        ],
    },
    Descriptor {
        kind: Kind::Facet,
        label: "Facet",
        category: "Pixelate",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::Mezzotint,
        label: "Mezzotint",
        category: "Pixelate",
        parameters: &[
            Parameter {
                key: "pattern",
                label: "Type",
                kind: ParameterType::Choice(&[
                    "Fine Dots",
                    "Medium Dots",
                    "Grainy Dots",
                    "Coarse Dots",
                    "Short Lines",
                    "Medium Lines",
                    "Long Lines",
                    "Short Strokes",
                    "Medium Strokes",
                    "Long Strokes",
                ]),
            },
            number("pattern_scale", "Scale", 0.05, 8., 0.05),
            number("seed", "Seed", 0., 65535., 1.),
            Parameter {
                key: "monochrome",
                label: "Monochrome",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::Pointillize,
        label: "Pointillize",
        category: "Pixelate",
        parameters: &[
            number("cell_size", "Cell size (px)", 2., 128., 1.),
            number("dot", "Dot size", 0., 1., 0.01),
            Parameter {
                key: "color",
                label: "Canvas color",
                kind: ParameterType::Color,
            },
            number("seed", "Seed", 0., 65535., 1.),
        ],
    },
    Descriptor {
        kind: Kind::StainedGlass,
        label: "Stained Glass",
        category: "Texture",
        parameters: &[
            number("cell_size", "Cell size (px)", 2., 128., 1.),
            number("width", "Border width (px)", 0., 16., 0.1),
            number("light", "Light intensity", 0., 1., 0.01),
            Parameter {
                key: "color",
                label: "Border color",
                kind: ParameterType::Color,
            },
            number("seed", "Seed", 0., 65535., 1.),
        ],
    },
    Descriptor {
        kind: Kind::MosaicTiles,
        label: "Mosaic Tiles",
        category: "Texture",
        parameters: &[
            number("tile_size", "Tile size (px)", 1., 64., 1.),
            number("grout", "Grout width (px)", 0., 16., 0.1),
            Parameter {
                key: "lighten",
                label: "Lighten grout",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::Diffuse,
        label: "Diffuse",
        category: "Stylize",
        parameters: &[
            Parameter {
                key: "mode",
                label: "Mode",
                kind: ParameterType::Choice(&[
                    "Normal",
                    "Darken Only",
                    "Lighten Only",
                    "Anisotropic",
                ]),
            },
            number("radius", "Radius (px)", 0., 8., 1.),
            number("seed", "Seed", 0., 65535., 1.),
        ],
    },
    Descriptor {
        kind: Kind::ColoredPencil,
        label: "Colored Pencil",
        category: "Artistic",
        parameters: &[
            number("width", "Pencil width (px)", 0.5, 16.0, 0.1),
            number("pressure", "Stroke pressure", 0.0, 1.0, 0.01),
            number("brightness", "Paper brightness", 0.0, 1.0, 0.01),
            Parameter {
                key: "paper",
                label: "Paper color",
                kind: ParameterType::Color,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::DryBrush,
        label: "Dry Brush",
        category: "Artistic",
        parameters: &[
            number("radius", "Brush size (px)", 0.0, 16.0, 0.1),
            number("detail", "Brush detail", 0.0, 1.0, 0.01),
            number("texture", "Texture", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::PaintDaubs,
        label: "Paint Daubs",
        category: "Artistic",
        parameters: &[
            number("radius", "Brush size (px)", 0.0, 16.0, 0.1),
            number("sharpness", "Sharpness", 0.0, 1.0, 0.01),
            Parameter {
                key: "style",
                label: "Brush type",
                kind: ParameterType::Choice(&["Round", "Square", "Diagonal"]),
            },
        ],
    },
    Descriptor {
        kind: Kind::SmudgeStick,
        label: "Smudge Stick",
        category: "Artistic",
        parameters: &[
            number("length", "Stroke length (px)", 0.5, 32.0, 0.1),
            number("highlight", "Highlight area", 0.0, 1.0, 0.01),
            number("intensity", "Intensity", 0.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::AngledStrokes,
        label: "Angled Strokes",
        category: "Brush Strokes",
        parameters: &[
            number("balance", "Direction balance", 0.0, 1.0, 0.01),
            number("length", "Stroke length (px)", 0.5, 32.0, 0.1),
            number("sharpness", "Sharpness", 0.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Crosshatch,
        label: "Crosshatch",
        category: "Brush Strokes",
        parameters: &[
            number("length", "Stroke length (px)", 2.0, 32.0, 0.1),
            number("sharpness", "Sharpness", 0.0, 1.0, 0.01),
            number("strength", "Strength", 0.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::DarkStrokes,
        label: "Dark Strokes",
        category: "Brush Strokes",
        parameters: &[
            number("balance", "Stroke balance", 0.0, 1.0, 0.01),
            number("black", "Black intensity", 0.0, 1.0, 0.01),
            number("white", "White intensity", 0.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Spatter,
        label: "Spatter",
        category: "Brush Strokes",
        parameters: &[
            number("radius", "Spray radius (px)", 0.0, 16.0, 0.1),
            number("smoothness", "Smoothness (px)", 0.0, 4.0, 0.1),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::SprayedStrokes,
        label: "Sprayed Strokes",
        category: "Brush Strokes",
        parameters: &[
            number("length", "Stroke length (px)", 0.0, 32.0, 0.1),
            number("radius", "Spray radius (px)", 0.0, 16.0, 0.1),
            Parameter {
                key: "direction",
                label: "Stroke direction",
                kind: ParameterType::Choice(&[
                    "Right diagonal",
                    "Left diagonal",
                    "Horizontal",
                    "Vertical",
                ]),
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::SumiE,
        label: "Sumi-e",
        category: "Brush Strokes",
        parameters: &[
            number("width", "Stroke width (px)", 0.0, 16.0, 0.1),
            number("pressure", "Stroke pressure", 0.0, 1.0, 0.01),
            number("contrast", "Contrast", 0.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::BasRelief,
        label: "Bas Relief",
        category: "Sketch",
        parameters: &[
            number("detail", "Detail", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::ChalkCharcoal,
        label: "Chalk & Charcoal",
        category: "Sketch",
        parameters: &[
            number("charcoal", "Charcoal area", 0.0, 1.0, 0.01),
            number("chalk", "Chalk area", 0.0, 1.0, 0.01),
            number("pressure", "Stroke pressure", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::Charcoal,
        label: "Charcoal",
        category: "Sketch",
        parameters: &[
            number("width", "Thickness (px)", 0.5, 8.0, 0.1),
            number("detail", "Detail", 0.0, 1.0, 0.01),
            number("balance", "Light/dark balance", 0.0, 1.0, 0.01),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::GraphicPen,
        label: "Graphic Pen",
        category: "Sketch",
        parameters: &[
            number("length", "Stroke length (px)", 0.5, 32.0, 0.1),
            number("balance", "Light/dark balance", 0.0, 1.0, 0.01),
            Parameter {
                key: "direction",
                label: "Stroke direction",
                kind: ParameterType::Choice(&[
                    "Right diagonal",
                    "Left diagonal",
                    "Horizontal",
                    "Vertical",
                ]),
            },
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::NotePaper,
        label: "Note Paper",
        category: "Sketch",
        parameters: &[
            number("balance", "Image balance", 0.0, 1.0, 0.01),
            number("grain", "Graininess", 0.0, 1.0, 0.01),
            number("relief", "Relief", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::Stamp,
        label: "Stamp",
        category: "Sketch",
        parameters: &[
            number("balance", "Light/dark balance", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::Chrome,
        label: "Chrome",
        category: "Sketch",
        parameters: &[
            number("detail", "Detail", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
        ],
    },
    Descriptor {
        kind: Kind::ConteCrayon,
        label: "Conté Crayon",
        category: "Sketch",
        parameters: &[
            number("foreground", "Foreground level", 0.0, 1.0, 0.01),
            number("background", "Background level", 0.0, 1.0, 0.01),
            Parameter {
                key: "texture",
                label: "Texture",
                kind: ParameterType::Choice(&["Canvas", "Brick", "Burlap", "Sandstone"]),
            },
            number("scale", "Texture size (px)", 1.0, 32.0, 0.1),
            number("relief", "Relief", 0.0, 1.0, 0.01),
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "invert",
                label: "Invert texture",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::Plaster,
        label: "Plaster",
        category: "Sketch",
        parameters: &[
            number("balance", "Image balance", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::Reticulation,
        label: "Reticulation",
        category: "Sketch",
        parameters: &[
            number("density", "Density", 0.0, 1.0, 0.01),
            number("foreground", "Foreground level", 0.0, 1.0, 0.01),
            number("background", "Background level", 0.0, 1.0, 0.01),
            number("grain_size", "Grain size (px)", 0.5, 16.0, 0.1),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::TornEdges,
        label: "Torn Edges",
        category: "Sketch",
        parameters: &[
            number("balance", "Image balance", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
            number("contrast", "Contrast", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "low",
                label: "Ink color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Paper color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::WaterPaper,
        label: "Water Paper",
        category: "Sketch",
        parameters: &[
            number("length", "Fiber length (px)", 0.5, 32.0, 0.1),
            number("brightness", "Brightness", -1.0, 1.0, 0.01),
            number("contrast", "Contrast", -0.99, 0.99, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::Fresco,
        label: "Fresco",
        category: "Artistic",
        parameters: &[
            number("radius", "Brush radius (px)", 0.0, 24.0, 0.1),
            number("detail", "Brush detail", 0.0, 1.0, 0.01),
            number("texture", "Texture", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::NeonGlow,
        label: "Neon Glow",
        category: "Artistic",
        parameters: &[
            number("size", "Glow size", -1.0, 1.0, 0.01),
            number("brightness", "Glow brightness", 0.0, 1.0, 0.01),
            Parameter {
                key: "color",
                label: "Glow color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "low",
                label: "Foreground color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Background color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::PaletteKnife,
        label: "Palette Knife",
        category: "Artistic",
        parameters: &[
            number("length", "Stroke length (px)", 0.5, 64.0, 0.1),
            number("detail", "Stroke detail", 0.0, 1.0, 0.01),
            number("smoothness", "Softness (px)", 0.0, 8.0, 0.1),
        ],
    },
    Descriptor {
        kind: Kind::PlasticWrap,
        label: "Plastic Wrap",
        category: "Artistic",
        parameters: &[
            number("strength", "Highlight strength", 0.0, 1.0, 0.01),
            number("detail", "Detail", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
        ],
    },
    Descriptor {
        kind: Kind::RoughPastels,
        label: "Rough Pastels",
        category: "Artistic",
        parameters: &[
            number("length", "Stroke length (px)", 0.5, 48.0, 0.1),
            number("detail", "Stroke detail", 0.0, 1.0, 0.01),
            Parameter {
                key: "texture",
                label: "Texture",
                kind: ParameterType::Choice(&["Canvas", "Brick", "Burlap", "Sandstone"]),
            },
            number("scale", "Texture size (px)", 1.0, 32.0, 0.1),
            number("relief", "Relief", 0.0, 1.0, 0.01),
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "invert",
                label: "Invert texture",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::Sponge,
        label: "Sponge",
        category: "Artistic",
        parameters: &[
            number("radius", "Brush radius (px)", 0.0, 24.0, 0.1),
            number("definition", "Definition", 0.0, 1.0, 0.01),
            number("smoothness", "Smoothness (px)", 0.0, 8.0, 0.1),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::Underpainting,
        label: "Underpainting",
        category: "Artistic",
        parameters: &[
            number("radius", "Brush radius (px)", 0.0, 24.0, 0.1),
            number("coverage", "Texture coverage", 0.0, 1.0, 0.01),
            Parameter {
                key: "texture",
                label: "Texture",
                kind: ParameterType::Choice(&["Canvas", "Brick", "Burlap", "Sandstone"]),
            },
            number("scale", "Texture size (px)", 1.0, 32.0, 0.1),
            number("relief", "Relief", 0.0, 1.0, 0.01),
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "invert",
                label: "Invert texture",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::Watercolor,
        label: "Watercolor",
        category: "Artistic",
        parameters: &[
            number("detail", "Brush detail", 0.0, 1.0, 0.01),
            number("shadows", "Shadow intensity", 0.0, 1.0, 0.01),
            number("texture", "Texture", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::DiffuseGlow,
        label: "Diffuse Glow",
        category: "Distort",
        parameters: &[
            number("amount", "Glow amount", 0.0, 1.0, 0.01),
            number("clear", "Clear amount", 0.0, 1.0, 0.01),
            number("radius", "Glow radius (px)", 0.0, 32.0, 0.1),
            number("grain", "Grain", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "color",
                label: "Glow color",
                kind: ParameterType::Color,
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Glass,
        label: "Glass",
        category: "Distort",
        parameters: &[
            number("distortion", "Distortion (px)", 0.0, 32.0, 0.1),
            number("smoothness", "Smoothness", 0.0, 1.0, 0.01),
            number("scale", "Texture size (px)", 1.0, 64.0, 0.1),
            Parameter {
                key: "texture",
                label: "Texture",
                kind: ParameterType::Choice(&["Frosted", "Tiny lens", "Blocks", "Canvas"]),
            },
            Parameter {
                key: "invert",
                label: "Invert texture",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "edges",
                label: "Edges",
                kind: ParameterType::Choice(&["Wrap", "Repeat", "Transparent"]),
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::OceanRipple,
        label: "Ocean Ripple",
        category: "Distort",
        parameters: &[
            number("amplitude", "Magnitude (px)", 0.0, 32.0, 0.1),
            number("wavelength", "Ripple size (px)", 1.0, 96.0, 0.1),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "edges",
                label: "Edges",
                kind: ParameterType::Choice(&["Wrap", "Repeat", "Transparent"]),
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Texturizer,
        label: "Texturizer",
        category: "Texture",
        parameters: &[
            Parameter {
                key: "texture",
                label: "Texture",
                kind: ParameterType::Choice(&["Canvas", "Brick", "Burlap", "Sandstone"]),
            },
            number("scale", "Texture size (px)", 1.0, 32.0, 0.1),
            number("relief", "Relief", 0.0, 1.0, 0.01),
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "invert",
                label: "Invert texture",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::ShapeBlur,
        label: "Shape Blur",
        category: "Blur",
        parameters: &[
            number("radius", "Radius (px)", 0.0, 16.0, 0.1),
            Parameter {
                key: "shape",
                label: "Shape",
                kind: ParameterType::Choice(&["Disk", "Triangle", "Square", "Hexagon", "Octagon"]),
            },
            number("angle", "Angle (°)", -180.0, 180.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::SmartBlur,
        label: "Smart Blur",
        category: "Blur",
        parameters: &[
            number("radius", "Radius (px)", 0.0, 8.0, 1.0),
            number("threshold", "Threshold", 0.0, 255.0, 1.0),
            Parameter {
                key: "mode",
                label: "Mode",
                kind: ParameterType::Choice(&["Normal", "Edge only", "Overlay edge"]),
            },
        ],
    },
    Descriptor {
        kind: Kind::SharpenEdges,
        label: "Sharpen Edges",
        category: "Sharpen",
        parameters: &[],
    },
    Descriptor {
        kind: Kind::SmartSharpen,
        label: "Smart Sharpen",
        category: "Sharpen",
        parameters: &[
            number("radius", "Radius (px)", 0.0, 8.0, 0.1),
            number("amount", "Amount", 0.0, 5.0, 0.01),
            number("reduce_noise", "Reduce noise", 0.0, 1.0, 0.01),
            number("iterations", "Iterations", 1.0, 6.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::Extrude,
        label: "Extrude",
        category: "Stylize",
        parameters: &[
            number("cell_size", "Block size (px)", 4.0, 96.0, 1.0),
            number("depth", "Depth (px)", 0.0, 64.0, 0.1),
            Parameter {
                key: "shape",
                label: "Shape",
                kind: ParameterType::Choice(&["Blocks", "Pyramids"]),
            },
            Parameter {
                key: "random",
                label: "Random depth",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::OilPaint,
        label: "Oil Paint",
        category: "Stylize",
        parameters: &[
            number("radius", "Brush radius (px)", 0.0, 12.0, 0.1),
            number("stylization", "Stylization", 0.0, 1.0, 0.01),
            number("cleanliness", "Cleanliness", 0.0, 1.0, 0.01),
            number("scale", "Bristle size (px)", 0.5, 24.0, 0.1),
            number("bristles", "Bristle detail", 0.0, 1.0, 0.01),
            Parameter {
                key: "lighting",
                label: "Lighting",
                kind: ParameterType::Boolean,
            },
            number("angle", "Light angle (°)", -180.0, 180.0, 1.0),
            number("shine", "Shine", 0.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Tiles,
        label: "Tiles",
        category: "Stylize",
        parameters: &[
            number("tile_size", "Tile size (px)", 2.0, 128.0, 1.0),
            number("offset", "Maximum offset", 0.0, 0.5, 0.01),
            Parameter {
                key: "fill",
                label: "Fill gaps",
                kind: ParameterType::Choice(&[
                    "Unaltered",
                    "Transparent",
                    "Foreground",
                    "Background",
                ]),
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
            Parameter {
                key: "low",
                label: "Foreground color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "high",
                label: "Background color",
                kind: ParameterType::Color,
            },
        ],
    },
    Descriptor {
        kind: Kind::TraceContour,
        label: "Trace Contour",
        category: "Stylize",
        parameters: &[
            number("level", "Level", 0.0, 255.0, 1.0),
            Parameter {
                key: "upper",
                label: "Upper edge",
                kind: ParameterType::Boolean,
            },
        ],
    },
    Descriptor {
        kind: Kind::Wind,
        label: "Wind",
        category: "Stylize",
        parameters: &[
            number("length", "Length (px)", 0.0, 64.0, 0.1),
            Parameter {
                key: "method",
                label: "Method",
                kind: ParameterType::Choice(&["Wind", "Blast", "Stagger"]),
            },
            Parameter {
                key: "from_right",
                label: "From right",
                kind: ParameterType::Boolean,
            },
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::LensBlur,
        label: "Lens Blur",
        category: "Blur",
        parameters: &[
            number("radius", "Radius (px)", 0.0, 16.0, 0.1),
            Parameter {
                key: "shape",
                label: "Iris shape",
                kind: ParameterType::Choice(&["Disk", "Triangle", "Square", "Hexagon", "Octagon"]),
            },
            number("angle", "Rotation (°)", -180.0, 180.0, 1.0),
            Parameter {
                key: "depth_source",
                label: "Depth source",
                kind: ParameterType::Choice(&["Uniform", "Alpha", "Luminance"]),
            },
            number("focus", "Focal depth", 0.0, 1.0, 0.01),
            Parameter {
                key: "invert",
                label: "Invert depth",
                kind: ParameterType::Boolean,
            },
            number("highlights", "Highlight gain", 0.0, 1.0, 0.01),
            number("threshold", "Highlight threshold", 0.0, 255.0, 1.0),
            number("noise", "Noise", 0.0, 1.0, 0.01),
            number("seed", "Seed", 0.0, 65535.0, 1.0),
        ],
    },
    Descriptor {
        kind: Kind::LensFlare,
        label: "Lens Flare",
        category: "Render",
        parameters: &[
            number("brightness", "Brightness", 0.0, 2.0, 0.01),
            number("radius", "Radius (px)", 0.0, 256.0, 0.1),
            Parameter {
                key: "style",
                label: "Lens type",
                kind: ParameterType::Choice(&["Zoom", "Prime", "Movie", "Anamorphic"]),
            },
            Parameter {
                key: "color",
                label: "Color",
                kind: ParameterType::Color,
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::LightingEffects,
        label: "Lighting Effects",
        category: "Render",
        parameters: &[
            Parameter {
                key: "kind",
                label: "Light type",
                kind: ParameterType::Choice(&["Point", "Spot", "Directional"]),
            },
            number("intensity", "Intensity", 0.0, 2.0, 0.01),
            number("ambient", "Ambience", 0.0, 1.0, 0.01),
            number("radius", "Radius (px)", 0.0, 1024.0, 0.1),
            number("angle", "Angle (°)", -180.0, 180.0, 1.0),
            number("elevation", "Elevation (°)", 5.0, 90.0, 1.0),
            number("gloss", "Gloss", 0.0, 1.0, 0.01),
            number("metallic", "Metallic", 0.0, 1.0, 0.01),
            number("relief", "Texture height", 0.0, 8.0, 0.1),
            Parameter {
                key: "color",
                label: "Light color",
                kind: ParameterType::Color,
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::ColorLookup,
        label: "Color Lookup",
        category: "Adjustments",
        parameters: &[number("amount", "Amount", 0.0, 1.0, 0.01)],
    },
    Descriptor {
        kind: Kind::Displace,
        label: "Displace",
        category: "Distort",
        parameters: &[
            number("horizontal", "Horizontal (px)", -128.0, 128.0, 0.1),
            number("vertical", "Vertical (px)", -128.0, 128.0, 0.1),
            Parameter {
                key: "fit",
                label: "Map fitting",
                kind: ParameterType::Choice(&["Stretch", "Tile"]),
            },
            Parameter {
                key: "edges",
                label: "Edges",
                kind: ParameterType::Choice(&["Wrap", "Repeat", "Transparent"]),
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Shear,
        label: "Shear",
        category: "Distort",
        parameters: &[
            Parameter {
                key: "interpolation",
                label: "Interpolation",
                kind: ParameterType::Choice(&["Linear", "Smooth"]),
            },
            Parameter {
                key: "edges",
                label: "Edges",
                kind: ParameterType::Choice(&["Wrap", "Repeat", "Transparent"]),
            },
            CENTER_X,
            CENTER_Y,
            number("curve0", "Curve 1", -1.0, 1.0, 0.01),
            number("curve1", "Curve 2", -1.0, 1.0, 0.01),
            number("curve2", "Curve 3", -1.0, 1.0, 0.01),
            number("curve3", "Curve 4", -1.0, 1.0, 0.01),
            number("curve4", "Curve 5", -1.0, 1.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Custom,
        label: "Custom",
        category: "Other",
        parameters: &[
            number("divisor", "Scale", -1024.0, 1024.0, 0.1),
            number("bias", "Offset", -255.0, 255.0, 1.0),
            Parameter {
                key: "alpha",
                label: "Filter alpha",
                kind: ParameterType::Boolean,
            },
            Parameter {
                key: "edges",
                label: "Edges",
                kind: ParameterType::Choice(&["Wrap", "Repeat", "Transparent"]),
            },
            number("k0", "Kernel 1,1", -64.0, 64.0, 0.01),
            number("k1", "Kernel 1,2", -64.0, 64.0, 0.01),
            number("k2", "Kernel 1,3", -64.0, 64.0, 0.01),
            number("k3", "Kernel 1,4", -64.0, 64.0, 0.01),
            number("k4", "Kernel 1,5", -64.0, 64.0, 0.01),
            number("k5", "Kernel 2,1", -64.0, 64.0, 0.01),
            number("k6", "Kernel 2,2", -64.0, 64.0, 0.01),
            number("k7", "Kernel 2,3", -64.0, 64.0, 0.01),
            number("k8", "Kernel 2,4", -64.0, 64.0, 0.01),
            number("k9", "Kernel 2,5", -64.0, 64.0, 0.01),
            number("k10", "Kernel 3,1", -64.0, 64.0, 0.01),
            number("k11", "Kernel 3,2", -64.0, 64.0, 0.01),
            number("k12", "Kernel 3,3", -64.0, 64.0, 0.01),
            number("k13", "Kernel 3,4", -64.0, 64.0, 0.01),
            number("k14", "Kernel 3,5", -64.0, 64.0, 0.01),
            number("k15", "Kernel 4,1", -64.0, 64.0, 0.01),
            number("k16", "Kernel 4,2", -64.0, 64.0, 0.01),
            number("k17", "Kernel 4,3", -64.0, 64.0, 0.01),
            number("k18", "Kernel 4,4", -64.0, 64.0, 0.01),
            number("k19", "Kernel 4,5", -64.0, 64.0, 0.01),
            number("k20", "Kernel 5,1", -64.0, 64.0, 0.01),
            number("k21", "Kernel 5,2", -64.0, 64.0, 0.01),
            number("k22", "Kernel 5,3", -64.0, 64.0, 0.01),
            number("k23", "Kernel 5,4", -64.0, 64.0, 0.01),
            number("k24", "Kernel 5,5", -64.0, 64.0, 0.01),
        ],
    },
    Descriptor {
        kind: Kind::FieldBlur,
        label: "Field Blur",
        category: "Blur Gallery",
        parameters: &[
            Parameter {
                key: "pin",
                label: "Selected pin",
                kind: ParameterType::Choice(&[
                    "Pin 1", "Pin 2", "Pin 3", "Pin 4", "Pin 5", "Pin 6", "Pin 7", "Pin 8",
                ]),
            },
            number("count", "Active pins", 1., 8., 1.),
            number("radius", "Blur (px)", 0., 128., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::IrisBlur,
        label: "Iris Blur",
        category: "Blur Gallery",
        parameters: &[
            number("radius", "Blur (px)", 0., 128., 0.1),
            number("width", "Width (px)", 0., 2048., 0.1),
            number("height", "Height (px)", 0., 2048., 0.1),
            number("angle", "Angle (°)", -180., 180., 0.1),
            number("feather", "Feather", 0., 1., 0.01),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::TiltShift,
        label: "Tilt-Shift",
        category: "Blur Gallery",
        parameters: &[
            number("radius", "Blur (px)", 0., 128., 0.1),
            number("width", "Focus (px)", 0., 2048., 0.1),
            number("transition", "Transition (px)", 0., 2048., 0.1),
            number("angle", "Angle (°)", -180., 180., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::PathBlur,
        label: "Path Blur",
        category: "Blur Gallery",
        parameters: &[
            Parameter {
                key: "point",
                label: "Path handle",
                kind: ParameterType::Choice(&["Start", "Bend", "End"]),
            },
            number("distance", "Distance (px)", 0., 256., 0.1),
            number("taper", "Taper", 0., 1., 0.01),
            Parameter {
                key: "centered",
                label: "Centered blur",
                kind: ParameterType::Boolean,
            },
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::SpinBlur,
        label: "Spin Blur",
        category: "Blur Gallery",
        parameters: &[
            number("angle", "Blur angle (°)", 0., 360., 0.1),
            number("width", "Width (px)", 0., 2048., 0.1),
            number("height", "Height (px)", 0., 2048., 0.1),
            number("feather", "Feather", 0., 1., 0.01),
            number("rotation", "Ellipse angle (°)", -180., 180., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::Flame,
        label: "Flame",
        category: "Render",
        parameters: &[
            number("width", "Width (px)", 0., 2048., 0.1),
            number("height", "Height (px)", 0., 2048., 0.1),
            number("turbulence", "Turbulence", 0., 1., 0.01),
            number("detail", "Detail", 0., 1., 0.01),
            number("seed", "Seed", 0., 65535., 1.),
            Parameter {
                key: "color",
                label: "Flame color",
                kind: ParameterType::Color,
            },
            number("opacity", "Opacity", 0., 1., 0.01),
            number("angle", "Angle (°)", -180., 180., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::PictureFrame,
        label: "Picture Frame",
        category: "Render",
        parameters: &[
            number("width", "Width (px)", 0., 512., 0.1),
            number("margin", "Inset (px)", 0., 512., 0.1),
            number("bevel", "Bevel (px)", 0., 128., 0.1),
            Parameter {
                key: "style",
                label: "Style",
                kind: ParameterType::Choice(&["Plain", "Beveled", "Wood", "Ornate"]),
            },
            Parameter {
                key: "color",
                label: "Frame color",
                kind: ParameterType::Color,
            },
            number("opacity", "Opacity", 0., 1., 0.01),
        ],
    },
    Descriptor {
        kind: Kind::Tree,
        label: "Tree",
        category: "Render",
        parameters: &[
            number("height", "Height (px)", 0., 2048., 0.1),
            number("width", "Crown width (px)", 0., 2048., 0.1),
            number("branching", "Branch levels", 2., 9., 1.),
            number("foliage", "Foliage", 0., 1., 0.01),
            number("seed", "Seed", 0., 65535., 1.),
            Parameter {
                key: "form",
                label: "Tree form",
                kind: ParameterType::Choice(&["Deciduous", "Conifer", "Bare"]),
            },
            Parameter {
                key: "leaf_color",
                label: "Leaf color",
                kind: ParameterType::Color,
            },
            Parameter {
                key: "bark_color",
                label: "Bark color",
                kind: ParameterType::Color,
            },
            number("opacity", "Opacity", 0., 1., 0.01),
            number("angle", "Angle (°)", -180., 180., 0.1),
            CENTER_X,
            CENTER_Y,
        ],
    },
    Descriptor {
        kind: Kind::NtscColors,
        label: "NTSC Colors",
        category: "Video",
        parameters: &[Parameter {
            key: "setup",
            label: "7.5 IRE black setup",
            kind: ParameterType::Boolean,
        }],
    },
    Descriptor {
        kind: Kind::Deinterlace,
        label: "De-Interlace",
        category: "Video",
        parameters: &[
            Parameter {
                key: "field",
                label: "Remove field",
                kind: ParameterType::Choice(&["Odd (rows 1, 3, …)", "Even (rows 2, 4, …)"]),
            },
            Parameter {
                key: "interpolation",
                label: "Fill method",
                kind: ParameterType::Choice(&["Duplicate", "Interpolate"]),
            },
        ],
    },
    Descriptor {
        kind: Kind::HsbHsl,
        label: "HSB/HSL",
        category: "Other",
        parameters: &[
            Parameter {
                key: "input",
                label: "Input channels",
                kind: ParameterType::Choice(&["RGB", "HSB", "HSL"]),
            },
            Parameter {
                key: "output",
                label: "Output channels",
                kind: ParameterType::Choice(&["RGB", "HSB", "HSL"]),
            },
        ],
    },
];
impl Kind {
    /// Metadata for this implemented operation.
    pub fn descriptor(self) -> &'static Descriptor {
        REGISTRY
            .iter()
            .find(|d| d.kind == self)
            .expect("registered kind")
    }
    /// Construct valid editable defaults without allocating a document ID.
    pub fn default_operation(self) -> Operation {
        if let Some(effect) = styles::default(self) {
            return Operation::Effect(effect);
        }
        match self {
            Self::BevelEmboss | Self::Satin | Self::GradientOverlay | Self::PatternOverlay => {
                unreachable!("handled style")
            }

            Self::FieldBlur => Operation::Filter(Filter::FieldBlur {
                pins: std::array::from_fn(|i| BlurPin {
                    center: [0.15 + (i % 4) as f32 * 0.23, 0.35 + (i / 4) as f32 * 0.3],
                    radius: if i == 0 { 0. } else { 16. },
                }),
                pin: 1,
                count: 2,
            }),
            Self::IrisBlur => Operation::Filter(Filter::IrisBlur {
                radius: 16.,
                width: 180.,
                height: 120.,
                angle: 0.,
                feather: 0.5,
                center: [0.5; 2],
            }),
            Self::TiltShift => Operation::Filter(Filter::TiltShift {
                radius: 16.,
                width: 60.,
                transition: 100.,
                angle: 0.,
                center: [0.5; 2],
            }),
            Self::PathBlur => Operation::Filter(Filter::PathBlur {
                distance: 30.,
                taper: 0.5,
                centered: true,
                path: [[0.2, 0.6], [0.5, 0.3], [0.8, 0.6]],
                point: 1,
            }),
            Self::SpinBlur => Operation::Filter(Filter::SpinBlur {
                angle: 15.,
                width: 180.,
                height: 120.,
                feather: 0.5,
                rotation: 0.,
                center: [0.5; 2],
            }),
            Self::Flame => Operation::Filter(Filter::Flame {
                width: 240.,
                height: 280.,
                turbulence: 0.6,
                detail: 0.5,
                seed: 17,
                color: [255, 100, 20],
                opacity: 1.,
                angle: 0.,
                center: [0.5, 0.9],
            }),
            Self::PictureFrame => Operation::Filter(Filter::PictureFrame {
                width: 24.,
                margin: 8.,
                bevel: 6.,
                style: FrameStyle::Wood,
                color: [150, 95, 45],
                opacity: 1.,
            }),
            Self::Tree => Operation::Filter(Filter::Tree {
                height: 380.,
                width: 280.,
                branching: 7,
                foliage: 0.8,
                seed: 17,
                form: TreeForm::Deciduous,
                leaf_color: [65, 135, 48],
                bark_color: [100, 68, 40],
                opacity: 1.,
                angle: 0.,
                center: [0.5, 0.9],
            }),
            Self::NtscColors => Operation::Filter(Filter::NtscColors { setup: false }),
            Self::Deinterlace => Operation::Filter(Filter::Deinterlace {
                field: VideoField::Odd,
                interpolation: FieldInterpolation::Interpolate,
            }),
            Self::HsbHsl => Operation::Filter(Filter::HsbHsl {
                input: ChannelSpace::Rgb,
                output: ChannelSpace::Hsl,
            }),
            Self::ColorLookup => Operation::Filter(Filter::ColorLookup {
                amount: 1.,
                lut: Some(editable::warm_lut()),
            }),
            Self::Displace => Operation::Filter(Filter::Displace {
                horizontal: 10.,
                vertical: 10.,
                fit: MapFit::Stretch,
                edges: FilterEdges::Repeat,
                center: [0.5; 2],
                map: Some(editable::wave_map()),
                map_name: "Built-in waves".into(),
            }),
            Self::Shear => Operation::Filter(Filter::Shear {
                interpolation: ShearInterpolation::Smooth,
                edges: FilterEdges::Repeat,
                center: [0.5; 2],
                curve: [0., 0.15, 0., -0.15, 0.],
            }),
            Self::Custom => Operation::Filter(Filter::Custom {
                divisor: 1.,
                bias: 0.,
                alpha: false,
                edges: FilterEdges::Repeat,
                kernel: editable::identity_kernel(),
            }),
            Self::Extrude => Operation::Filter(Filter::Extrude {
                cell_size: 16,
                depth: 16.,
                shape: ExtrudeShape::Blocks,
                random: false,
                seed: 1,
                center: [0.5; 2],
            }),
            Self::OilPaint => Operation::Filter(Filter::OilPaint {
                radius: 4.,
                stylization: 0.7,
                cleanliness: 0.5,
                scale: 6.,
                bristles: 0.4,
                lighting: true,
                angle: -45.,
                shine: 0.3,
            }),
            Self::Tiles => Operation::Filter(Filter::Tiles {
                tile_size: 24,
                offset: 0.3,
                fill: TileFill::Unaltered,
                seed: 1,
                low: [0, 0, 0],
                high: [255, 255, 255],
            }),
            Self::TraceContour => Operation::Filter(Filter::TraceContour {
                level: 128,
                upper: true,
            }),
            Self::Wind => Operation::Filter(Filter::Wind {
                length: 12.,
                method: WindMethod::Wind,
                from_right: false,
                seed: 1,
            }),
            Self::LensBlur => Operation::Filter(Filter::LensBlur {
                radius: 6.,
                shape: ApertureShape::Hexagon,
                angle: 0.,
                depth_source: DepthSource::Uniform,
                focus: 0.5,
                invert: false,
                highlights: 0.3,
                threshold: 200.,
                noise: 0.,
                seed: 1,
            }),
            Self::LensFlare => Operation::Filter(Filter::LensFlare {
                brightness: 0.8,
                radius: 48.,
                style: FlareStyle::Zoom,
                color: [255, 240, 215],
                center: [0.5; 2],
            }),
            Self::LightingEffects => Operation::Filter(Filter::LightingEffects {
                kind: LightKind::Point,
                intensity: 1.,
                ambient: 0.3,
                radius: 200.,
                angle: -45.,
                elevation: 45.,
                gloss: 0.3,
                metallic: 0.,
                relief: 2.,
                color: [255, 255, 255],
                center: [0.5; 2],
            }),
            Self::DiffuseGlow => Operation::Filter(Filter::DiffuseGlow {
                amount: 0.5,
                clear: 0.4,
                radius: 6.,
                grain: 0.2,
                seed: 1,
                color: [255, 255, 255],
                center: [0.5; 2],
            }),
            Self::Glass => Operation::Filter(Filter::Glass {
                distortion: 5.,
                smoothness: 0.3,
                scale: 12.,
                texture: GlassSurface::Frosted,
                invert: false,
                seed: 1,
                edges: FilterEdges::Repeat,
                center: [0.5; 2],
            }),
            Self::OceanRipple => Operation::Filter(Filter::OceanRipple {
                amplitude: 5.,
                wavelength: 16.,
                seed: 1,
                edges: FilterEdges::Repeat,
                center: [0.5; 2],
            }),
            Self::Texturizer => Operation::Filter(Filter::Texturizer {
                texture: SurfaceTexture::Canvas,
                scale: 6.,
                relief: 0.5,
                angle: -45.,
                invert: false,
                seed: 1,
            }),
            Self::ShapeBlur => Operation::Filter(Filter::ShapeBlur {
                radius: 4.,
                shape: ApertureShape::Hexagon,
                angle: 0.,
            }),
            Self::SmartBlur => Operation::Filter(Filter::SmartBlur {
                radius: 2,
                threshold: 32.,
                mode: SmartBlurMode::Normal,
            }),
            Self::SharpenEdges => Operation::Filter(Filter::SharpenEdges),
            Self::SmartSharpen => Operation::Filter(Filter::SmartSharpen {
                radius: 1.,
                amount: 1.,
                reduce_noise: 0.2,
                iterations: 3,
            }),
            Self::Fresco => Operation::Filter(Filter::Fresco {
                radius: 4.,
                detail: 0.4,
                texture: 0.4,
                seed: 0,
            }),
            Self::NeonGlow => Operation::Filter(Filter::NeonGlow {
                size: 0.,
                brightness: 0.6,
                color: [60, 140, 255],
                low: [0, 0, 0],
                high: [255, 255, 255],
            }),
            Self::PaletteKnife => Operation::Filter(Filter::PaletteKnife {
                length: 10.,
                detail: 0.4,
                smoothness: 1.,
            }),
            Self::PlasticWrap => Operation::Filter(Filter::PlasticWrap {
                strength: 0.6,
                detail: 0.5,
                smoothness: 1.5,
            }),
            Self::RoughPastels => Operation::Filter(Filter::RoughPastels {
                length: 8.,
                detail: 0.5,
                texture: SurfaceTexture::Canvas,
                scale: 6.,
                relief: 0.5,
                angle: -45.,
                invert: false,
                seed: 0,
            }),
            Self::Sponge => Operation::Filter(Filter::Sponge {
                radius: 5.,
                definition: 0.6,
                smoothness: 1.,
                seed: 0,
            }),
            Self::Underpainting => Operation::Filter(Filter::Underpainting {
                radius: 4.,
                coverage: 0.5,
                texture: SurfaceTexture::Canvas,
                scale: 6.,
                relief: 0.5,
                angle: -45.,
                invert: false,
                seed: 0,
            }),
            Self::Watercolor => Operation::Filter(Filter::Watercolor {
                detail: 0.5,
                shadows: 0.5,
                texture: 0.35,
                seed: 0,
            }),
            Self::Chrome => Operation::Filter(Filter::Chrome {
                detail: 0.5,
                smoothness: 1.,
            }),
            Self::ConteCrayon => Operation::Filter(Filter::ConteCrayon {
                foreground: 0.8,
                background: 0.8,
                texture: SurfaceTexture::Canvas,
                scale: 6.,
                relief: 0.3,
                angle: -135.,
                invert: false,
                seed: 1,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::Plaster => Operation::Filter(Filter::Plaster {
                balance: 0.5,
                smoothness: 1.,
                angle: -135.,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::Reticulation => Operation::Filter(Filter::Reticulation {
                density: 0.5,
                foreground: 0.7,
                background: 0.7,
                grain_size: 3.,
                seed: 1,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::TornEdges => Operation::Filter(Filter::TornEdges {
                balance: 0.5,
                smoothness: 1.,
                contrast: 0.5,
                seed: 1,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::WaterPaper => Operation::Filter(Filter::WaterPaper {
                length: 8.,
                brightness: 0.,
                contrast: 0.2,
                seed: 1,
            }),

            Self::SprayedStrokes => Operation::Filter(Filter::SprayedStrokes {
                length: 12.,
                radius: 3.,
                direction: StrokeDirection::RightDiagonal,
                seed: 1,
            }),
            Self::SumiE => Operation::Filter(Filter::SumiE {
                width: 3.,
                pressure: 0.6,
                contrast: 0.3,
            }),
            Self::BasRelief => Operation::Filter(Filter::BasRelief {
                detail: 0.5,
                smoothness: 1.,
                angle: -135.,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::ChalkCharcoal => Operation::Filter(Filter::ChalkCharcoal {
                charcoal: 0.5,
                chalk: 0.5,
                pressure: 0.8,
                seed: 1,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::Charcoal => Operation::Filter(Filter::Charcoal {
                width: 2.,
                detail: 0.6,
                balance: 0.5,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::GraphicPen => Operation::Filter(Filter::GraphicPen {
                length: 8.,
                balance: 0.5,
                direction: StrokeDirection::RightDiagonal,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::NotePaper => Operation::Filter(Filter::NotePaper {
                balance: 0.5,
                grain: 0.3,
                relief: 0.5,
                seed: 1,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::Stamp => Operation::Filter(Filter::Stamp {
                balance: 0.5,
                smoothness: 1.,
                low: [0; 3],
                high: [255; 3],
            }),

            Self::ColoredPencil => Operation::Filter(Filter::ColoredPencil {
                width: 2.,
                pressure: 0.8,
                brightness: 1.,
                paper: [255; 3],
                seed: 1,
            }),
            Self::DryBrush => Operation::Filter(Filter::DryBrush {
                radius: 4.,
                detail: 0.5,
                texture: 0.3,
                seed: 1,
            }),
            Self::PaintDaubs => Operation::Filter(Filter::PaintDaubs {
                radius: 5.,
                sharpness: 0.4,
                style: DaubStyle::Round,
            }),
            Self::SmudgeStick => Operation::Filter(Filter::SmudgeStick {
                length: 8.,
                highlight: 0.7,
                intensity: 0.7,
            }),
            Self::AngledStrokes => Operation::Filter(Filter::AngledStrokes {
                balance: 0.5,
                length: 12.,
                sharpness: 0.4,
            }),
            Self::Crosshatch => Operation::Filter(Filter::Crosshatch {
                length: 12.,
                sharpness: 0.6,
                strength: 0.6,
            }),
            Self::DarkStrokes => Operation::Filter(Filter::DarkStrokes {
                balance: 0.5,
                black: 0.5,
                white: 0.5,
            }),
            Self::Spatter => Operation::Filter(Filter::Spatter {
                radius: 4.,
                smoothness: 0.8,
                seed: 1,
            }),

            Self::ColorBalance => Operation::Filter(Filter::ColorBalance {
                balance: [[0.; 3]; 3],
                preserve_luminosity: true,
            }),
            Self::PhotoFilter => Operation::Filter(Filter::PhotoFilter {
                color: [255, 155, 55],
                density: 0.25,
                preserve_luminosity: true,
            }),
            Self::ChannelMixer => Operation::Filter(Filter::ChannelMixer {
                matrix: [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]],
                monochrome: false,
            }),
            Self::GradientMap => Operation::Filter(Filter::GradientMap {
                low: [0; 3],
                high: [255; 3],
                midpoint: 0.5,
                reverse: false,
            }),
            Self::SelectiveColor => Operation::Filter(Filter::SelectiveColor {
                adjustments: [[0.; 4]; 9],
                relative: true,
            }),
            Self::ShadowsHighlights => Operation::Filter(Filter::ShadowsHighlights {
                shadows: 0.35,
                shadow_width: 0.5,
                shadow_radius: 30.,
                highlights: 0.,
                highlight_width: 0.5,
                highlight_radius: 30.,
            }),
            Self::Blur => Operation::Filter(Filter::Blur),
            Self::BlurMore => Operation::Filter(Filter::BlurMore),

            Self::ColorHalftone => Operation::Filter(Filter::ColorHalftone {
                radius: 4.,
                cyan: 108.,
                magenta: 162.,
                yellow: 90.,
                black: 45.,
            }),
            Self::Crystallize => Operation::Filter(Filter::Crystallize { size: 16, seed: 1 }),
            Self::Facet => Operation::Filter(Filter::Facet),
            Self::Mezzotint => Operation::Filter(Filter::Mezzotint {
                pattern: MezzotintStyle::FineDots,
                scale: 1.,
                seed: 1,
                monochrome: true,
            }),
            Self::Pointillize => Operation::Filter(Filter::Pointillize {
                size: 16,
                dot: 0.75,
                color: [255; 3],
                seed: 1,
            }),
            Self::StainedGlass => Operation::Filter(Filter::StainedGlass {
                size: 24,
                width: 2.,
                light: 0.3,
                color: [0; 3],
                seed: 1,
            }),
            Self::MosaicTiles => Operation::Filter(Filter::MosaicTiles {
                size: 12,
                grout: 2.,
                lighten: true,
            }),
            Self::Diffuse => Operation::Filter(Filter::Diffuse {
                radius: 1,
                mode: DiffuseMode::Normal,
                seed: 1,
            }),

            Self::Cutout => Operation::Filter(Filter::Cutout {
                levels: 4,
                simplicity: 2.,
                fidelity: 0.5,
            }),
            Self::FilmGrain => Operation::Filter(Filter::FilmGrain {
                grain: 0.25,
                highlight: 0.5,
                intensity: 0.5,
                seed: 1,
            }),
            Self::AccentedEdges => Operation::Filter(Filter::AccentedEdges {
                width: 1.,
                brightness: 0.8,
                smoothness: 1.,
            }),
            Self::InkOutlines => Operation::Filter(Filter::InkOutlines {
                length: 4.,
                dark: 0.7,
                light: 0.2,
            }),
            Self::Photocopy => Operation::Filter(Filter::Photocopy {
                detail: 0.5,
                darkness: 0.4,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::HalftonePattern => Operation::Filter(Filter::HalftonePattern {
                size: 8.,
                contrast: 0.2,
                pattern: HalftoneStyle::Dot,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::Patchwork => Operation::Filter(Filter::Patchwork {
                square: 8,
                relief: 8.,
            }),
            Self::Craquelure => Operation::Filter(Filter::Craquelure {
                spacing: 16.,
                depth: 0.6,
                brightness: 0.1,
                seed: 1,
            }),

            Self::PolarCoordinates => Operation::Filter(Filter::PolarCoordinates {
                center: [0.5; 2],
                to_polar: true,
            }),
            Self::ZigZag => Operation::Filter(Filter::ZigZag {
                center: [0.5; 2],
                amount: 12.,
                radius: 128.,
                ridges: 5,
                style: ZigZagStyle::PondRipples,
            }),
            Self::Fibers => Operation::Filter(Filter::Fibers {
                seed: 1,
                variance: 16.,
                strength: 0.6,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::GlowingEdges => Operation::Filter(Filter::GlowingEdges {
                width: 1.,
                brightness: 1.5,
                smoothness: 1.,
            }),
            Self::Despeckle => Operation::Filter(Filter::Despeckle),
            Self::DustScratches => Operation::Filter(Filter::DustScratches {
                radius: 1,
                threshold: 12,
            }),
            Self::ReduceNoise => Operation::Filter(Filter::ReduceNoise {
                radius: 2,
                strength: 0.5,
                preserve_details: 0.5,
                color_noise: 0.3,
                sharpen: 0.,
            }),
            Self::Pinch => Operation::Filter(Filter::Pinch {
                amount: 0.5,
                radius: 128.,
                center: [0.5; 2],
            }),
            Self::Spherize => Operation::Filter(Filter::Spherize {
                amount: 0.5,
                radius: 128.,
                center: [0.5; 2],
            }),
            Self::Ripple => Operation::Filter(Filter::Ripple {
                amplitude: 6.,
                wavelength: 48.,
                center: [0.5; 2],
            }),
            Self::Sharpen => Operation::Filter(Filter::Sharpen),
            Self::SharpenMore => Operation::Filter(Filter::SharpenMore),
            Self::SurfaceBlur => Operation::Filter(Filter::SurfaceBlur {
                radius: 2,
                threshold: 24,
            }),
            Self::RadialBlur => Operation::Filter(Filter::RadialBlur {
                amount: 10.,
                samples: 16,
                zoom: false,
                center: [0.5; 2],
            }),
            Self::Clouds => Operation::Filter(Filter::Clouds {
                seed: 1,
                scale: 128.,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::DifferenceClouds => Operation::Filter(Filter::DifferenceClouds {
                seed: 1,
                scale: 128.,
                low: [0; 3],
                high: [255; 3],
            }),
            Self::PosterEdges => Operation::Filter(Filter::PosterEdges {
                thickness: 1,
                intensity: 0.8,
                levels: 6,
            }),
            Self::Grain => Operation::Filter(Filter::Grain {
                amount: 0.12,
                contrast: 0.2,
                size: 1.,
                seed: 1,
                monochrome: true,
                style: GrainStyle::Regular,
            }),
            Self::Fragment => Operation::Filter(Filter::Fragment { distance: 2. }),
            Self::Average => Operation::Filter(Filter::Average),
            Self::FindEdges => Operation::Filter(Filter::FindEdges),
            Self::Solarize => Operation::Filter(Filter::Solarize),
            Self::Emboss => Operation::Filter(Filter::Emboss {
                angle: 135.,
                height: 1,
                amount: 1.,
            }),
            Self::UnsharpMask => Operation::Filter(Filter::UnsharpMask {
                radius: 2.,
                amount: 1.,
                threshold: 0,
            }),
            Self::Minimum => Operation::Filter(Filter::Minimum { radius: 2 }),
            Self::Maximum => Operation::Filter(Filter::Maximum { radius: 2 }),
            Self::Offset => Operation::Filter(Filter::Offset {
                x: 16,
                y: 16,
                edges: FilterEdges::Wrap,
            }),
            Self::Twirl => Operation::Filter(Filter::Twirl {
                radius: 128.,
                angle: 90.,
                center: [0.5; 2],
            }),
            Self::Wave => Operation::Filter(Filter::Wave {
                amplitude: 8.,
                wavelength: 64.,
                vertical: false,
                edges: FilterEdges::Wrap,
                center: [0.; 2],
            }),
            Self::BrightnessContrast => Operation::Filter(Filter::BrightnessContrast {
                brightness: 0.,
                contrast: 0.,
            }),
            Self::Exposure => Operation::Filter(Filter::Exposure { stops: 0. }),
            Self::HueSaturation => Operation::Filter(Filter::HueSaturation {
                hue: 0.,
                saturation: 0.,
                lightness: 0.,
            }),
            Self::Vibrance => Operation::Filter(Filter::Vibrance { amount: 0. }),
            Self::BlackWhite => Operation::Filter(Filter::BlackWhite { strength: 1. }),
            Self::Threshold => Operation::Filter(Filter::Threshold { level: 128 }),
            Self::Posterize => Operation::Filter(Filter::Posterize { levels: 6 }),
            Self::BoxBlur => Operation::Filter(Filter::BoxBlur { radius: 2 }),
            Self::Median => Operation::Filter(Filter::Median { radius: 1 }),
            Self::InnerShadow => Operation::Effect(Effect::InnerShadow {
                color: [0; 3],
                opacity: 0.65,
                angle: 120.,
                distance: 6.,
                spread: 0.,
                size: 6.,
            }),
            Self::InnerGlow => Operation::Effect(Effect::InnerGlow {
                color: [255; 3],
                opacity: 0.6,
                spread: 0.,
                size: 8.,
            }),
            Self::Stroke => Operation::Effect(Effect::Stroke {
                color: [35, 70, 120],
                opacity: 1.,
                width: 4.,
                position: StrokePosition::Outside,
                rounded: true,
            }),
            Self::Levels => Operation::Filter(Filter::Levels {
                black: 0,
                white: 255,
                gamma: 1.,
                output_black: 0,
                output_white: 255,
            }),
            Self::Curves => Operation::Filter(Filter::Curves {
                points: [0, 64, 128, 192, 255],
            }),
            Self::MotionBlur => Operation::Filter(Filter::MotionBlur {
                distance: 12.,
                angle: 0.,
            }),
            Self::HighPass => Operation::Filter(Filter::HighPass { radius: 4. }),
            Self::Mosaic => Operation::Filter(Filter::Mosaic { block_size: 12 }),
            Self::AddNoise => Operation::Filter(Filter::AddNoise {
                amount: 0.1,
                seed: 1,
                monochrome: true,
                distribution: NoiseDistribution::Uniform,
            }),
            Self::Invert => Operation::Filter(Filter::Invert),
            Self::Brightness => Operation::Filter(Filter::Brightness { amount: 0. }),
            Self::GaussianBlur => Operation::Filter(Filter::GaussianBlur { radius: 4. }),
            Self::ColorOverlay => Operation::Effect(Effect::ColorOverlay {
                color: [53, 215, 225],
                opacity: 0.5,
            }),
            Self::DropShadow => Operation::Effect(Effect::DropShadow {
                color: [0; 3],
                opacity: 0.7,
                angle: 120.,
                distance: 12.,
                spread: 0.,
                size: 8.,
            }),
            Self::OuterGlow => Operation::Effect(Effect::OuterGlow {
                color: [53, 215, 225],
                opacity: 0.7,
                spread: 0.,
                size: 12.,
            }),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Filter(Filter),
    Effect(Effect),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Number(f32),
    Color([u8; 3]),
    Boolean(bool),
    Choice(usize),
}
impl Operation {
    /// Registry identifier for these typed parameters.
    pub fn kind(&self) -> Kind {
        if let Self::Effect(Effect::Configured { effect, .. }) = self {
            return Self::Effect((**effect).clone()).kind();
        }
        match self {
            Self::Effect(Effect::Configured { .. }) => {
                unreachable!("unwrapped effect configuration")
            }

            Self::Filter(Filter::FieldBlur { .. }) => Kind::FieldBlur,
            Self::Filter(Filter::IrisBlur { .. }) => Kind::IrisBlur,
            Self::Filter(Filter::TiltShift { .. }) => Kind::TiltShift,
            Self::Filter(Filter::PathBlur { .. }) => Kind::PathBlur,
            Self::Filter(Filter::SpinBlur { .. }) => Kind::SpinBlur,
            Self::Filter(Filter::Flame { .. }) => Kind::Flame,
            Self::Filter(Filter::PictureFrame { .. }) => Kind::PictureFrame,
            Self::Filter(Filter::Tree { .. }) => Kind::Tree,
            Self::Filter(Filter::NtscColors { .. }) => Kind::NtscColors,
            Self::Filter(Filter::Deinterlace { .. }) => Kind::Deinterlace,
            Self::Filter(Filter::HsbHsl { .. }) => Kind::HsbHsl,
            Self::Filter(Filter::ColorLookup { .. }) => Kind::ColorLookup,
            Self::Filter(Filter::Displace { .. }) => Kind::Displace,
            Self::Filter(Filter::Shear { .. }) => Kind::Shear,
            Self::Filter(Filter::Custom { .. }) => Kind::Custom,
            Self::Filter(Filter::Extrude { .. }) => Kind::Extrude,
            Self::Filter(Filter::OilPaint { .. }) => Kind::OilPaint,
            Self::Filter(Filter::Tiles { .. }) => Kind::Tiles,
            Self::Filter(Filter::TraceContour { .. }) => Kind::TraceContour,
            Self::Filter(Filter::Wind { .. }) => Kind::Wind,
            Self::Filter(Filter::LensBlur { .. }) => Kind::LensBlur,
            Self::Filter(Filter::LensFlare { .. }) => Kind::LensFlare,
            Self::Filter(Filter::LightingEffects { .. }) => Kind::LightingEffects,
            Self::Filter(Filter::DiffuseGlow { .. }) => Kind::DiffuseGlow,
            Self::Filter(Filter::Glass { .. }) => Kind::Glass,
            Self::Filter(Filter::OceanRipple { .. }) => Kind::OceanRipple,
            Self::Filter(Filter::Texturizer { .. }) => Kind::Texturizer,
            Self::Filter(Filter::ShapeBlur { .. }) => Kind::ShapeBlur,
            Self::Filter(Filter::SmartBlur { .. }) => Kind::SmartBlur,
            Self::Filter(Filter::SharpenEdges) => Kind::SharpenEdges,
            Self::Filter(Filter::SmartSharpen { .. }) => Kind::SmartSharpen,
            Self::Filter(Filter::Fresco { .. }) => Kind::Fresco,
            Self::Filter(Filter::NeonGlow { .. }) => Kind::NeonGlow,
            Self::Filter(Filter::PaletteKnife { .. }) => Kind::PaletteKnife,
            Self::Filter(Filter::PlasticWrap { .. }) => Kind::PlasticWrap,
            Self::Filter(Filter::RoughPastels { .. }) => Kind::RoughPastels,
            Self::Filter(Filter::Sponge { .. }) => Kind::Sponge,
            Self::Filter(Filter::Underpainting { .. }) => Kind::Underpainting,
            Self::Filter(Filter::Watercolor { .. }) => Kind::Watercolor,
            Self::Filter(Filter::Chrome { .. }) => Kind::Chrome,
            Self::Filter(Filter::ConteCrayon { .. }) => Kind::ConteCrayon,
            Self::Filter(Filter::Plaster { .. }) => Kind::Plaster,
            Self::Filter(Filter::Reticulation { .. }) => Kind::Reticulation,
            Self::Filter(Filter::TornEdges { .. }) => Kind::TornEdges,
            Self::Filter(Filter::WaterPaper { .. }) => Kind::WaterPaper,

            Self::Filter(Filter::SprayedStrokes { .. }) => Kind::SprayedStrokes,
            Self::Filter(Filter::SumiE { .. }) => Kind::SumiE,
            Self::Filter(Filter::BasRelief { .. }) => Kind::BasRelief,
            Self::Filter(Filter::ChalkCharcoal { .. }) => Kind::ChalkCharcoal,
            Self::Filter(Filter::Charcoal { .. }) => Kind::Charcoal,
            Self::Filter(Filter::GraphicPen { .. }) => Kind::GraphicPen,
            Self::Filter(Filter::NotePaper { .. }) => Kind::NotePaper,
            Self::Filter(Filter::Stamp { .. }) => Kind::Stamp,

            Self::Filter(Filter::ColoredPencil { .. }) => Kind::ColoredPencil,
            Self::Filter(Filter::DryBrush { .. }) => Kind::DryBrush,
            Self::Filter(Filter::PaintDaubs { .. }) => Kind::PaintDaubs,
            Self::Filter(Filter::SmudgeStick { .. }) => Kind::SmudgeStick,
            Self::Filter(Filter::AngledStrokes { .. }) => Kind::AngledStrokes,
            Self::Filter(Filter::Crosshatch { .. }) => Kind::Crosshatch,
            Self::Filter(Filter::DarkStrokes { .. }) => Kind::DarkStrokes,
            Self::Filter(Filter::Spatter { .. }) => Kind::Spatter,

            Self::Filter(Filter::ColorBalance { .. }) => Kind::ColorBalance,
            Self::Filter(Filter::PhotoFilter { .. }) => Kind::PhotoFilter,
            Self::Filter(Filter::ChannelMixer { .. }) => Kind::ChannelMixer,
            Self::Filter(Filter::GradientMap { .. }) => Kind::GradientMap,
            Self::Filter(Filter::SelectiveColor { .. }) => Kind::SelectiveColor,
            Self::Filter(Filter::ShadowsHighlights { .. }) => Kind::ShadowsHighlights,
            Self::Filter(Filter::Blur) => Kind::Blur,
            Self::Filter(Filter::BlurMore) => Kind::BlurMore,

            Self::Filter(Filter::Facet) => Kind::Facet,
            Self::Filter(Filter::Diffuse { .. }) => Kind::Diffuse,
            Self::Filter(Filter::MosaicTiles { .. }) => Kind::MosaicTiles,
            Self::Filter(Filter::StainedGlass { .. }) => Kind::StainedGlass,
            Self::Filter(Filter::Pointillize { .. }) => Kind::Pointillize,
            Self::Filter(Filter::Mezzotint { .. }) => Kind::Mezzotint,
            Self::Filter(Filter::Crystallize { .. }) => Kind::Crystallize,
            Self::Filter(Filter::ColorHalftone { .. }) => Kind::ColorHalftone,
            Self::Filter(Filter::Craquelure { .. }) => Kind::Craquelure,
            Self::Filter(Filter::Patchwork { .. }) => Kind::Patchwork,
            Self::Filter(Filter::HalftonePattern { .. }) => Kind::HalftonePattern,
            Self::Filter(Filter::Photocopy { .. }) => Kind::Photocopy,
            Self::Filter(Filter::InkOutlines { .. }) => Kind::InkOutlines,
            Self::Filter(Filter::AccentedEdges { .. }) => Kind::AccentedEdges,
            Self::Filter(Filter::FilmGrain { .. }) => Kind::FilmGrain,
            Self::Filter(Filter::Cutout { .. }) => Kind::Cutout,
            Self::Filter(Filter::Despeckle) => Kind::Despeckle,
            Self::Filter(Filter::DustScratches { .. }) => Kind::DustScratches,
            Self::Filter(Filter::ReduceNoise { .. }) => Kind::ReduceNoise,
            Self::Filter(Filter::PolarCoordinates { .. }) => Kind::PolarCoordinates,
            Self::Filter(Filter::ZigZag { .. }) => Kind::ZigZag,
            Self::Filter(Filter::Fibers { .. }) => Kind::Fibers,
            Self::Filter(Filter::GlowingEdges { .. }) => Kind::GlowingEdges,
            Self::Filter(Filter::Pinch { .. }) => Kind::Pinch,
            Self::Filter(Filter::Spherize { .. }) => Kind::Spherize,
            Self::Filter(Filter::Ripple { .. }) => Kind::Ripple,
            Self::Filter(Filter::Sharpen) => Kind::Sharpen,
            Self::Filter(Filter::SharpenMore) => Kind::SharpenMore,
            Self::Filter(Filter::SurfaceBlur { .. }) => Kind::SurfaceBlur,
            Self::Filter(Filter::RadialBlur { .. }) => Kind::RadialBlur,
            Self::Filter(Filter::Clouds { .. }) => Kind::Clouds,
            Self::Filter(Filter::DifferenceClouds { .. }) => Kind::DifferenceClouds,
            Self::Filter(Filter::PosterEdges { .. }) => Kind::PosterEdges,
            Self::Filter(Filter::Grain { .. }) => Kind::Grain,
            Self::Filter(Filter::Fragment { .. }) => Kind::Fragment,
            Self::Filter(Filter::Average) => Kind::Average,
            Self::Filter(Filter::FindEdges) => Kind::FindEdges,
            Self::Filter(Filter::Solarize) => Kind::Solarize,
            Self::Filter(Filter::Emboss { .. }) => Kind::Emboss,
            Self::Filter(Filter::UnsharpMask { .. }) => Kind::UnsharpMask,
            Self::Filter(Filter::Minimum { .. }) => Kind::Minimum,
            Self::Filter(Filter::Maximum { .. }) => Kind::Maximum,
            Self::Filter(Filter::Offset { .. }) => Kind::Offset,
            Self::Filter(Filter::Twirl { .. }) => Kind::Twirl,
            Self::Filter(Filter::Wave { .. }) => Kind::Wave,
            Self::Filter(Filter::BrightnessContrast { .. }) => Kind::BrightnessContrast,
            Self::Filter(Filter::Exposure { .. }) => Kind::Exposure,
            Self::Filter(Filter::HueSaturation { .. }) => Kind::HueSaturation,
            Self::Filter(Filter::Vibrance { .. }) => Kind::Vibrance,
            Self::Filter(Filter::BlackWhite { .. }) => Kind::BlackWhite,
            Self::Filter(Filter::Threshold { .. }) => Kind::Threshold,
            Self::Filter(Filter::Posterize { .. }) => Kind::Posterize,
            Self::Filter(Filter::BoxBlur { .. }) => Kind::BoxBlur,
            Self::Filter(Filter::Median { .. }) => Kind::Median,
            Self::Effect(Effect::BevelEmboss { .. }) => Kind::BevelEmboss,
            Self::Effect(Effect::Satin { .. }) => Kind::Satin,
            Self::Effect(Effect::GradientOverlay { .. }) => Kind::GradientOverlay,
            Self::Effect(Effect::PatternOverlay { .. }) => Kind::PatternOverlay,
            Self::Effect(Effect::InnerShadow { .. }) => Kind::InnerShadow,
            Self::Effect(Effect::InnerGlow { .. }) => Kind::InnerGlow,
            Self::Effect(Effect::Stroke { .. }) => Kind::Stroke,
            Self::Filter(Filter::Levels { .. }) => Kind::Levels,
            Self::Filter(Filter::Curves { .. }) => Kind::Curves,
            Self::Filter(Filter::MotionBlur { .. }) => Kind::MotionBlur,
            Self::Filter(Filter::HighPass { .. }) => Kind::HighPass,
            Self::Filter(Filter::Mosaic { .. }) => Kind::Mosaic,
            Self::Filter(Filter::AddNoise { .. }) => Kind::AddNoise,
            Self::Filter(Filter::Invert) => Kind::Invert,
            Self::Filter(Filter::Brightness { .. }) => Kind::Brightness,
            Self::Filter(Filter::GaussianBlur { .. }) => Kind::GaussianBlur,
            Self::Effect(Effect::ColorOverlay { .. }) => Kind::ColorOverlay,
            Self::Effect(Effect::DropShadow { .. }) => Kind::DropShadow,
            Self::Effect(Effect::OuterGlow { .. }) => Kind::OuterGlow,
        }
    }
    /// Read a registered parameter by its stable key.
    pub fn value(&self, key: &str) -> Option<Value> {
        if let Self::Effect(e) = self {
            if self
                .kind()
                .descriptor()
                .parameters
                .iter()
                .any(|p| p.key == key)
                && let Some(value) = style_options::value(e, key)
            {
                return Some(value);
            }
            if let Effect::Configured { effect, .. } = e {
                return Self::Effect((**effect).clone()).value(key);
            }
        }
        if let Self::Effect(e) = self
            && let Some(v) = styles::value(e, key)
        {
            return Some(v);
        }

        if let Self::Filter(f) = self
            && let Some(value) = adjustments::value(f, key)
        {
            return Some(value);
        }
        if let Self::Filter(f) = self
            && let Some(c) = f.center()
        {
            match key {
                "center_x" => return Some(Value::Number(c[0])),
                "center_y" => return Some(Value::Number(c[1])),
                _ => {}
            }
        }

        if let Self::Filter(f) = self
            && let Some(v) = gallery::value(f, key)
        {
            return Some(v);
        }
        match (self, key) {
            (Self::Filter(Filter::IrisBlur { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::IrisBlur { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::IrisBlur { height: v, .. }), "height") => Some(Value::Number(*v)),
            (Self::Filter(Filter::IrisBlur { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::IrisBlur { feather: v, .. }), "feather") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::TiltShift { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::TiltShift { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::TiltShift { transition: v, .. }), "transition") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::TiltShift { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::PathBlur { distance: v, .. }), "distance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PathBlur { taper: v, .. }), "taper") => Some(Value::Number(*v)),
            (Self::Filter(Filter::PathBlur { centered: v, .. }), "centered") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::SpinBlur { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::SpinBlur { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::SpinBlur { height: v, .. }), "height") => Some(Value::Number(*v)),
            (Self::Filter(Filter::SpinBlur { feather: v, .. }), "feather") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SpinBlur { rotation: v, .. }), "rotation") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Flame { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Flame { height: v, .. }), "height") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Flame { turbulence: v, .. }), "turbulence") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Flame { detail: v, .. }), "detail") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Flame { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Flame { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Flame { opacity: v, .. }), "opacity") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Flame { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::PictureFrame { width: v, .. }), "width") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PictureFrame { margin: v, .. }), "margin") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PictureFrame { bevel: v, .. }), "bevel") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PictureFrame { style: v, .. }), "style") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::PictureFrame { color: v, .. }), "color") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::PictureFrame { opacity: v, .. }), "opacity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Tree { height: v, .. }), "height") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Tree { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Tree { branching: v, .. }), "branching") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Tree { foliage: v, .. }), "foliage") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Tree { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Tree { form: v, .. }), "form") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Tree { leaf_color: v, .. }), "leaf_color") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::Tree { bark_color: v, .. }), "bark_color") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::Tree { opacity: v, .. }), "opacity") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Tree { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::NtscColors { setup: v, .. }), "setup") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Deinterlace { field: v, .. }), "field") => {
                Some(Value::Choice(*v as usize))
            }
            (
                Self::Filter(Filter::Deinterlace {
                    interpolation: v, ..
                }),
                "interpolation",
            ) => Some(Value::Choice(*v as usize)),
            (Self::Filter(Filter::HsbHsl { input: v, .. }), "input") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::HsbHsl { output: v, .. }), "output") => {
                Some(Value::Choice(*v as usize))
            }

            (Self::Filter(Filter::ColorLookup { amount: v, .. }), "amount") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Displace { horizontal: v, .. }), "horizontal") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Displace { vertical: v, .. }), "vertical") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Displace { fit: v, .. }), "fit") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Displace { edges: v, .. }), "edges") => {
                Some(Value::Choice(*v as usize))
            }
            (
                Self::Filter(Filter::Shear {
                    interpolation: v, ..
                }),
                "interpolation",
            ) => Some(Value::Choice(*v as usize)),
            (Self::Filter(Filter::Shear { edges: v, .. }), "edges") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Shear { curve, .. }), "curve0") => Some(Value::Number(curve[0])),
            (Self::Filter(Filter::Shear { curve, .. }), "curve1") => Some(Value::Number(curve[1])),
            (Self::Filter(Filter::Shear { curve, .. }), "curve2") => Some(Value::Number(curve[2])),
            (Self::Filter(Filter::Shear { curve, .. }), "curve3") => Some(Value::Number(curve[3])),
            (Self::Filter(Filter::Shear { curve, .. }), "curve4") => Some(Value::Number(curve[4])),
            (Self::Filter(Filter::Custom { divisor: v, .. }), "divisor") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Custom { bias: v, .. }), "bias") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Custom { alpha: v, .. }), "alpha") => Some(Value::Boolean(*v)),
            (Self::Filter(Filter::Custom { edges: v, .. }), "edges") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k0") => Some(Value::Number(kernel[0])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k1") => Some(Value::Number(kernel[1])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k2") => Some(Value::Number(kernel[2])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k3") => Some(Value::Number(kernel[3])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k4") => Some(Value::Number(kernel[4])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k5") => Some(Value::Number(kernel[5])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k6") => Some(Value::Number(kernel[6])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k7") => Some(Value::Number(kernel[7])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k8") => Some(Value::Number(kernel[8])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k9") => Some(Value::Number(kernel[9])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k10") => Some(Value::Number(kernel[10])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k11") => Some(Value::Number(kernel[11])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k12") => Some(Value::Number(kernel[12])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k13") => Some(Value::Number(kernel[13])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k14") => Some(Value::Number(kernel[14])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k15") => Some(Value::Number(kernel[15])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k16") => Some(Value::Number(kernel[16])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k17") => Some(Value::Number(kernel[17])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k18") => Some(Value::Number(kernel[18])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k19") => Some(Value::Number(kernel[19])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k20") => Some(Value::Number(kernel[20])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k21") => Some(Value::Number(kernel[21])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k22") => Some(Value::Number(kernel[22])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k23") => Some(Value::Number(kernel[23])),
            (Self::Filter(Filter::Custom { kernel, .. }), "k24") => Some(Value::Number(kernel[24])),
            (Self::Filter(Filter::Extrude { cell_size: v, .. }), "cell_size") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Extrude { depth: v, .. }), "depth") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Extrude { shape: v, .. }), "shape") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Extrude { random: v, .. }), "random") => Some(Value::Boolean(*v)),
            (Self::Filter(Filter::Extrude { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::OilPaint { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::OilPaint { stylization: v, .. }), "stylization") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::OilPaint { cleanliness: v, .. }), "cleanliness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::OilPaint { scale: v, .. }), "scale") => Some(Value::Number(*v)),
            (Self::Filter(Filter::OilPaint { bristles: v, .. }), "bristles") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::OilPaint { lighting: v, .. }), "lighting") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::OilPaint { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::OilPaint { shine: v, .. }), "shine") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Tiles { tile_size: v, .. }), "tile_size") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Tiles { offset: v, .. }), "offset") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Tiles { fill: v, .. }), "fill") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Tiles { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Tiles { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Tiles { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::TraceContour { level: v, .. }), "level") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::TraceContour { upper: v, .. }), "upper") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Wind { length: v, .. }), "length") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Wind { method: v, .. }), "method") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Wind { from_right: v, .. }), "from_right") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Wind { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::LensBlur { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::LensBlur { shape: v, .. }), "shape") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::LensBlur { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (
                Self::Filter(Filter::LensBlur {
                    depth_source: v, ..
                }),
                "depth_source",
            ) => Some(Value::Choice(*v as usize)),
            (Self::Filter(Filter::LensBlur { focus: v, .. }), "focus") => Some(Value::Number(*v)),
            (Self::Filter(Filter::LensBlur { invert: v, .. }), "invert") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::LensBlur { highlights: v, .. }), "highlights") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LensBlur { threshold: v, .. }), "threshold") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LensBlur { noise: v, .. }), "noise") => Some(Value::Number(*v)),
            (Self::Filter(Filter::LensBlur { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::LensFlare { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LensFlare { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LensFlare { style: v, .. }), "style") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::LensFlare { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Filter(Filter::LightingEffects { kind: v, .. }), "kind") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::LightingEffects { intensity: v, .. }), "intensity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { ambient: v, .. }), "ambient") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { angle: v, .. }), "angle") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { elevation: v, .. }), "elevation") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { gloss: v, .. }), "gloss") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { metallic: v, .. }), "metallic") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::LightingEffects { color: v, .. }), "color") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::DiffuseGlow { amount: v, .. }), "amount") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DiffuseGlow { clear: v, .. }), "clear") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DiffuseGlow { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DiffuseGlow { grain: v, .. }), "grain") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DiffuseGlow { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::DiffuseGlow { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Glass { distortion: v, .. }), "distortion") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Glass { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Glass { scale: v, .. }), "scale") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Glass { texture: v, .. }), "texture") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Glass { invert: v, .. }), "invert") => Some(Value::Boolean(*v)),
            (Self::Filter(Filter::Glass { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Glass { edges: v, .. }), "edges") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::OceanRipple { amplitude: v, .. }), "amplitude") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::OceanRipple { wavelength: v, .. }), "wavelength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::OceanRipple { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::OceanRipple { edges: v, .. }), "edges") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Texturizer { texture: v, .. }), "texture") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Texturizer { scale: v, .. }), "scale") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Texturizer { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Texturizer { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Texturizer { invert: v, .. }), "invert") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Texturizer { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::ShapeBlur { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ShapeBlur { shape: v, .. }), "shape") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::ShapeBlur { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::SmartBlur { radius: v, .. }), "radius") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::SmartBlur { threshold: v, .. }), "threshold") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SmartBlur { mode: v, .. }), "mode") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::SmartSharpen { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SmartSharpen { amount: v, .. }), "amount") => {
                Some(Value::Number(*v))
            }
            (
                Self::Filter(Filter::SmartSharpen {
                    reduce_noise: v, ..
                }),
                "reduce_noise",
            ) => Some(Value::Number(*v)),
            (Self::Filter(Filter::SmartSharpen { iterations: v, .. }), "iterations") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Fresco { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Fresco { detail: v, .. }), "detail") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Fresco { texture: v, .. }), "texture") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Fresco { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::NeonGlow { size: v, .. }), "size") => Some(Value::Number(*v)),
            (Self::Filter(Filter::NeonGlow { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::NeonGlow { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Filter(Filter::NeonGlow { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::NeonGlow { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::PaletteKnife { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PaletteKnife { detail: v, .. }), "detail") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PaletteKnife { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PlasticWrap { strength: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PlasticWrap { detail: v, .. }), "detail") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PlasticWrap { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RoughPastels { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RoughPastels { detail: v, .. }), "detail") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RoughPastels { texture: v, .. }), "texture") => {
                Some(Value::Choice(
                    SurfaceTexture::ALL
                        .iter()
                        .position(|t| t == v)
                        .expect("texture"),
                ))
            }
            (Self::Filter(Filter::RoughPastels { scale: v, .. }), "scale") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RoughPastels { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RoughPastels { angle: v, .. }), "angle") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RoughPastels { invert: v, .. }), "invert") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::RoughPastels { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Sponge { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Sponge { definition: v, .. }), "definition") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Sponge { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Sponge { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Underpainting { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Underpainting { coverage: v, .. }), "coverage") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Underpainting { texture: v, .. }), "texture") => {
                Some(Value::Choice(
                    SurfaceTexture::ALL
                        .iter()
                        .position(|t| t == v)
                        .expect("texture"),
                ))
            }
            (Self::Filter(Filter::Underpainting { scale: v, .. }), "scale") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Underpainting { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Underpainting { angle: v, .. }), "angle") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Underpainting { invert: v, .. }), "invert") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Underpainting { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Watercolor { detail: v, .. }), "detail") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Watercolor { shadows: v, .. }), "shadows") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Watercolor { texture: v, .. }), "texture") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Watercolor { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Chrome { detail: v, .. }), "detail") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Chrome { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ConteCrayon { foreground: v, .. }), "foreground") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ConteCrayon { background: v, .. }), "background") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ConteCrayon { texture: v, .. }), "texture") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::ConteCrayon { scale: v, .. }), "scale") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ConteCrayon { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ConteCrayon { angle: v, .. }), "angle") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ConteCrayon { invert: v, .. }), "invert") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::ConteCrayon { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::ConteCrayon { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::ConteCrayon { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Plaster { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Plaster { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Plaster { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Plaster { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Plaster { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Reticulation { density: v, .. }), "density") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Reticulation { foreground: v, .. }), "foreground") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Reticulation { background: v, .. }), "background") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Reticulation { grain_size: v, .. }), "grain_size") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Reticulation { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Reticulation { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Reticulation { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::TornEdges { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::TornEdges { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::TornEdges { contrast: v, .. }), "contrast") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::TornEdges { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::TornEdges { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::TornEdges { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::WaterPaper { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::WaterPaper { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::WaterPaper { contrast: v, .. }), "contrast") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::WaterPaper { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }

            (Self::Filter(Filter::SprayedStrokes { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SprayedStrokes { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SprayedStrokes { direction: v, .. }), "direction") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::SprayedStrokes { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::SumiE { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::SumiE { pressure: v, .. }), "pressure") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SumiE { contrast: v, .. }), "contrast") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::BasRelief { detail: v, .. }), "detail") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::BasRelief { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::BasRelief { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::BasRelief { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::BasRelief { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::ChalkCharcoal { charcoal: v, .. }), "charcoal") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ChalkCharcoal { chalk: v, .. }), "chalk") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ChalkCharcoal { pressure: v, .. }), "pressure") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ChalkCharcoal { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::ChalkCharcoal { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::ChalkCharcoal { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Charcoal { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Charcoal { detail: v, .. }), "detail") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Charcoal { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Charcoal { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Charcoal { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::GraphicPen { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::GraphicPen { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::GraphicPen { direction: v, .. }), "direction") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::GraphicPen { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::GraphicPen { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::NotePaper { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::NotePaper { grain: v, .. }), "grain") => Some(Value::Number(*v)),
            (Self::Filter(Filter::NotePaper { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::NotePaper { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::NotePaper { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::NotePaper { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Stamp { balance: v, .. }), "balance") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Stamp { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Stamp { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Stamp { high: v, .. }), "high") => Some(Value::Color(*v)),

            (Self::Filter(Filter::ColoredPencil { width: v, .. }), "width") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColoredPencil { pressure: v, .. }), "pressure") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColoredPencil { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColoredPencil { paper: v, .. }), "paper") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::ColoredPencil { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::DryBrush { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::DryBrush { detail: v, .. }), "detail") => Some(Value::Number(*v)),
            (Self::Filter(Filter::DryBrush { texture: v, .. }), "texture") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DryBrush { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::PaintDaubs { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PaintDaubs { sharpness: v, .. }), "sharpness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PaintDaubs { style: v, .. }), "style") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::SmudgeStick { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SmudgeStick { highlight: v, .. }), "highlight") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::SmudgeStick { intensity: v, .. }), "intensity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::AngledStrokes { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::AngledStrokes { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::AngledStrokes { sharpness: v, .. }), "sharpness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Crosshatch { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Crosshatch { sharpness: v, .. }), "sharpness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Crosshatch { strength: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DarkStrokes { balance: v, .. }), "balance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DarkStrokes { black: v, .. }), "black") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DarkStrokes { white: v, .. }), "white") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Spatter { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Spatter { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Spatter { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }

            (Self::Filter(Filter::ColorHalftone { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColorHalftone { cyan: v, .. }), "cyan") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColorHalftone { magenta: v, .. }), "magenta") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColorHalftone { yellow: v, .. }), "yellow") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ColorHalftone { black: v, .. }), "black") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Crystallize { size: v, .. }), "cell_size") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Crystallize { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Mezzotint { pattern: v, .. }), "pattern") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Mezzotint { scale: v, .. }), "pattern_scale") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Mezzotint { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Mezzotint { monochrome: v, .. }), "monochrome") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Pointillize { size: v, .. }), "cell_size") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Pointillize { dot: v, .. }), "dot") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Pointillize { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Pointillize { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::StainedGlass { size: v, .. }), "cell_size") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::StainedGlass { width: v, .. }), "width") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::StainedGlass { light: v, .. }), "light") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::StainedGlass { color: v, .. }), "color") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::StainedGlass { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::MosaicTiles { size: v, .. }), "tile_size") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::MosaicTiles { grout: v, .. }), "grout") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::MosaicTiles { lighten: v, .. }), "lighten") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Diffuse { radius: v, .. }), "radius") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Diffuse { mode: v, .. }), "mode") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Diffuse { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Cutout { levels: v, .. }), "levels") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Cutout { simplicity: v, .. }), "simplicity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Cutout { fidelity: v, .. }), "fidelity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::FilmGrain { grain: v, .. }), "grain") => Some(Value::Number(*v)),
            (Self::Filter(Filter::FilmGrain { highlight: v, .. }), "highlight") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::FilmGrain { intensity: v, .. }), "intensity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::FilmGrain { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::AccentedEdges { width: v, .. }), "width") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::AccentedEdges { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::AccentedEdges { smoothness: v, .. }), "smoothness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::InkOutlines { length: v, .. }), "length") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::InkOutlines { dark: v, .. }), "dark") => Some(Value::Number(*v)),
            (Self::Filter(Filter::InkOutlines { light: v, .. }), "light") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Photocopy { detail: v, .. }), "detail") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Photocopy { darkness: v, .. }), "darkness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Photocopy { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Photocopy { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::HalftonePattern { size: v, .. }), "size") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::HalftonePattern { contrast: v, .. }), "contrast") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::HalftonePattern { pattern: v, .. }), "pattern") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::HalftonePattern { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::HalftonePattern { high: v, .. }), "high") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::Patchwork { square: v, .. }), "square") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::Patchwork { relief: v, .. }), "relief") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Craquelure { spacing: v, .. }), "spacing") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Craquelure { depth: v, .. }), "depth") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Craquelure { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Craquelure { seed: v, .. }), "seed") => {
                Some(Value::Number(f32::from(*v)))
            }
            (Self::Filter(Filter::PolarCoordinates { to_polar, .. }), "to_polar") => {
                Some(Value::Choice(usize::from(!to_polar)))
            }
            (Self::Filter(Filter::ZigZag { amount, .. }), "amount") => Some(Value::Number(*amount)),
            (Self::Filter(Filter::ZigZag { radius, .. }), "radius") => Some(Value::Number(*radius)),
            (Self::Filter(Filter::ZigZag { ridges, .. }), "ridges") => {
                Some(Value::Number(f32::from(*ridges)))
            }
            (Self::Filter(Filter::ZigZag { style, .. }), "style") => {
                Some(Value::Choice(*style as usize))
            }
            (Self::Filter(Filter::Fibers { seed, .. }), "seed") => {
                Some(Value::Number(f32::from(*seed)))
            }
            (Self::Filter(Filter::Fibers { variance, .. }), "variance") => {
                Some(Value::Number(*variance))
            }
            (Self::Filter(Filter::Fibers { strength, .. }), "strength") => {
                Some(Value::Number(*strength))
            }
            (Self::Filter(Filter::Fibers { low, .. }), "low") => Some(Value::Color(*low)),
            (Self::Filter(Filter::Fibers { high, .. }), "high") => Some(Value::Color(*high)),
            (Self::Filter(Filter::GlowingEdges { width, .. }), "width") => {
                Some(Value::Number(*width))
            }
            (Self::Filter(Filter::GlowingEdges { brightness, .. }), "brightness") => {
                Some(Value::Number(*brightness))
            }
            (Self::Filter(Filter::GlowingEdges { smoothness, .. }), "smoothness") => {
                Some(Value::Number(*smoothness))
            }

            (Self::Filter(Filter::DustScratches { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::DustScratches { threshold: v, .. }), "threshold") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::ReduceNoise { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::ReduceNoise { strength: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (
                Self::Filter(Filter::ReduceNoise {
                    preserve_details: v,
                    ..
                }),
                "preserve_details",
            ) => Some(Value::Number(*v)),
            (Self::Filter(Filter::ReduceNoise { color_noise: v, .. }), "color_noise") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::ReduceNoise { sharpen: v, .. }), "sharpen") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Pinch { amount: v, .. }), "strength") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Pinch { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Spherize { amount: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Spherize { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Ripple { amplitude: v, .. }), "amplitude") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Ripple { wavelength: v, .. }), "wavelength") => {
                Some(Value::Number(*v))
            }

            (Self::Filter(Filter::Fragment { distance }), "distance") => {
                Some(Value::Number(*distance))
            }
            (Self::Filter(Filter::SurfaceBlur { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::SurfaceBlur { threshold: v, .. }), "threshold") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::RadialBlur { amount: v, .. }), "amount") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::RadialBlur { samples: v, .. }), "samples") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::RadialBlur { zoom: v, .. }), "zoom") => {
                Some(Value::Choice(usize::from(*v)))
            }
            (Self::Filter(Filter::PosterEdges { thickness: v, .. }), "thickness") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::PosterEdges { intensity: v, .. }), "intensity") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::PosterEdges { levels: v, .. }), "levels") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Grain { amount: v, .. }), "strength") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Grain { contrast: v, .. }), "contrast") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Grain { size: v, .. }), "grain_size") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Grain { seed: v, .. }), "seed") => Some(Value::Number(*v as f32)),
            (Self::Filter(Filter::Grain { monochrome: v, .. }), "monochrome") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Grain { style: v, .. }), "style") => Some(Value::Choice(
                GrainStyle::ALL
                    .iter()
                    .position(|e| e == v)
                    .expect("grain type"),
            )),
            (Self::Filter(Filter::Clouds { seed: v, .. }), "seed") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Clouds { scale: v, .. }), "scale") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Clouds { low: v, .. }), "low") => Some(Value::Color(*v)),
            (Self::Filter(Filter::Clouds { high: v, .. }), "high") => Some(Value::Color(*v)),
            (Self::Filter(Filter::DifferenceClouds { seed: v, .. }), "seed") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::DifferenceClouds { scale: v, .. }), "scale") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::DifferenceClouds { low: v, .. }), "low") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::DifferenceClouds { high: v, .. }), "high") => {
                Some(Value::Color(*v))
            }
            (Self::Filter(Filter::Emboss { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Emboss { height: v, .. }), "height") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Emboss { amount: v, .. }), "strength") => Some(Value::Number(*v)),
            (Self::Filter(Filter::UnsharpMask { radius: v, .. }), "radius") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::UnsharpMask { amount: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::UnsharpMask { threshold: v, .. }), "threshold") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Minimum { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Maximum { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Offset { x: v, .. }), "x") => Some(Value::Number(*v as f32)),
            (Self::Filter(Filter::Offset { y: v, .. }), "y") => Some(Value::Number(*v as f32)),
            (Self::Filter(Filter::Offset { edges: v, .. }), "edges") => Some(Value::Choice(
                FilterEdges::ALL
                    .iter()
                    .position(|e| e == v)
                    .expect("edge choice"),
            )),
            (Self::Filter(Filter::Twirl { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Twirl { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Wave { amplitude: v, .. }), "amplitude") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Wave { wavelength: v, .. }), "wavelength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Wave { vertical: v, .. }), "vertical") => {
                Some(Value::Boolean(*v))
            }
            (Self::Filter(Filter::Wave { edges: v, .. }), "edges") => Some(Value::Choice(
                FilterEdges::ALL
                    .iter()
                    .position(|e| e == v)
                    .expect("edge choice"),
            )),
            (Self::Filter(Filter::Levels { black: v, .. }), "black") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Levels { white: v, .. }), "white") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Levels { gamma: v, .. }), "gamma") => Some(Value::Number(*v)),
            (
                Self::Filter(Filter::Levels {
                    output_black: v, ..
                }),
                "output_black",
            ) => Some(Value::Number(*v as f32)),
            (
                Self::Filter(Filter::Levels {
                    output_white: v, ..
                }),
                "output_white",
            ) => Some(Value::Number(*v as f32)),
            (Self::Filter(Filter::Curves { points: v, .. }), "point0") => {
                Some(Value::Number(v[0] as f32))
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point1") => {
                Some(Value::Number(v[1] as f32))
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point2") => {
                Some(Value::Number(v[2] as f32))
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point3") => {
                Some(Value::Number(v[3] as f32))
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point4") => {
                Some(Value::Number(v[4] as f32))
            }
            (Self::Filter(Filter::MotionBlur { distance: v, .. }), "distance") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::MotionBlur { angle: v, .. }), "angle") => Some(Value::Number(*v)),
            (Self::Filter(Filter::HighPass { radius: v, .. }), "radius") => Some(Value::Number(*v)),
            (Self::Filter(Filter::Mosaic { block_size: v, .. }), "block_size") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::AddNoise { amount: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::AddNoise { seed: v, .. }), "seed") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::AddNoise { monochrome: v, .. }), "monochrome") => {
                Some(Value::Boolean(*v))
            }
            (
                Self::Filter(Filter::AddNoise {
                    distribution: v, ..
                }),
                "distribution",
            ) => Some(Value::Choice(*v as usize)),
            (Self::Effect(Effect::Stroke { rounded, .. }), "rounded") => {
                Some(Value::Boolean(*rounded))
            }
            (
                Self::Effect(Effect::OuterGlow { size, .. } | Effect::InnerGlow { size, .. }),
                "radius",
            ) => Some(Value::Number(*size)),
            (Self::Filter(Filter::BrightnessContrast { brightness: v, .. }), "brightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::BrightnessContrast { contrast: v, .. }), "contrast") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Exposure { stops: v, .. }), "stops") => Some(Value::Number(*v)),
            (Self::Filter(Filter::HueSaturation { hue: v, .. }), "hue") => Some(Value::Number(*v)),
            (Self::Filter(Filter::HueSaturation { saturation: v, .. }), "saturation") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::HueSaturation { lightness: v, .. }), "lightness") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Vibrance { amount: v, .. }), "amount") => Some(Value::Number(*v)),
            (Self::Filter(Filter::BlackWhite { strength: v, .. }), "strength") => {
                Some(Value::Number(*v))
            }
            (Self::Filter(Filter::Threshold { level: v, .. }), "level") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Posterize { levels: v, .. }), "levels") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::BoxBlur { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Filter(Filter::Median { radius: v, .. }), "radius") => {
                Some(Value::Number(*v as f32))
            }
            (Self::Effect(Effect::InnerShadow { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Effect(Effect::InnerShadow { opacity: v, .. }), "opacity") => {
                Some(Value::Number(*v))
            }
            (Self::Effect(Effect::InnerShadow { angle: v, .. }), "angle") => {
                Some(Value::Number(*v))
            }
            (Self::Effect(Effect::InnerShadow { distance: v, .. }), "distance") => {
                Some(Value::Number(*v))
            }
            (Self::Effect(Effect::InnerShadow { spread: v, .. }), "spread") => {
                Some(Value::Number(*v))
            }
            (Self::Effect(Effect::InnerShadow { size: v, .. }), "size") => Some(Value::Number(*v)),
            (Self::Effect(Effect::InnerGlow { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Effect(Effect::InnerGlow { opacity: v, .. }), "opacity") => {
                Some(Value::Number(*v))
            }
            (Self::Effect(Effect::InnerGlow { spread: v, .. }), "spread") => {
                Some(Value::Number(*v))
            }
            (Self::Effect(Effect::InnerGlow { size: v, .. }), "size") => Some(Value::Number(*v)),
            (Self::Effect(Effect::Stroke { color: v, .. }), "color") => Some(Value::Color(*v)),
            (Self::Effect(Effect::Stroke { opacity: v, .. }), "opacity") => Some(Value::Number(*v)),
            (Self::Effect(Effect::Stroke { width: v, .. }), "width") => Some(Value::Number(*v)),
            (Self::Effect(Effect::Stroke { position: v, .. }), "position") => {
                Some(Value::Choice(*v as usize))
            }
            (Self::Filter(Filter::Brightness { amount }), "amount") => Some(Value::Number(*amount)),
            (Self::Filter(Filter::GaussianBlur { radius }), "radius") => {
                Some(Value::Number(*radius))
            }
            (
                Self::Effect(
                    Effect::ColorOverlay { color, .. }
                    | Effect::DropShadow { color, .. }
                    | Effect::OuterGlow { color, .. },
                ),
                "color",
            ) => Some(Value::Color(*color)),
            (
                Self::Effect(
                    Effect::ColorOverlay { opacity, .. }
                    | Effect::DropShadow { opacity, .. }
                    | Effect::OuterGlow { opacity, .. },
                ),
                "opacity",
            ) => Some(Value::Number(*opacity)),
            (
                Self::Effect(Effect::DropShadow { spread, .. } | Effect::OuterGlow { spread, .. }),
                "spread",
            ) => Some(Value::Number(*spread)),
            (
                Self::Effect(Effect::DropShadow { size, .. } | Effect::OuterGlow { size, .. }),
                "size",
            ) => Some(Value::Number(*size)),
            (Self::Effect(Effect::DropShadow { angle, .. }), "angle") => {
                Some(Value::Number(*angle))
            }
            (Self::Effect(Effect::DropShadow { distance, .. }), "distance") => {
                Some(Value::Number(*distance))
            }
            _ => None,
        }
    }
    /// Validate before mutation; a rejected value leaves the operation unchanged.
    pub fn set_value(&mut self, key: &str, value: Value) -> Result<(), String> {
        let key = if key == "size" && matches!(self.kind(), Kind::OuterGlow | Kind::InnerGlow) {
            "radius"
        } else {
            key
        };
        if self.kind() == Kind::Custom
            && key == "divisor"
            && matches!(value,Value::Number(n) if n.abs()<0.000001)
        {
            return Err("Scale must be nonzero".into());
        }
        let parameter = self
            .kind()
            .descriptor()
            .parameters
            .iter()
            .find(|p| p.key == key)
            .ok_or("Unknown processing parameter")?;
        match (parameter.kind, value) {
            (ParameterType::Color, Value::Color(_)) => {}
            (ParameterType::Boolean, Value::Boolean(_)) => {}
            (ParameterType::Choice(options), Value::Choice(n)) if n < options.len() => {}
            (ParameterType::Number { min, max, .. }, Value::Number(n))
                if n.is_finite() && (min..=max).contains(&n) => {}
            _ => return Err("Invalid processing parameter value".into()),
        }
        if let (Self::Filter(Filter::Levels { black, white, .. }), Value::Number(n)) =
            (&*self, value)
            && ((key == "black" && n >= f32::from(*white))
                || (key == "white" && n <= f32::from(*black)))
        {
            return Err("Input black must be below input white".into());
        }

        if let Self::Filter(f) = self
            && let Some(c) = f.center_mut()
            && let Value::Number(n) = value
        {
            match key {
                "center_x" => {
                    c[0] = n;
                    return Ok(());
                }
                "center_y" => {
                    c[1] = n;
                    return Ok(());
                }
                _ => {}
            }
        }

        if let Self::Effect(e) = self {
            if style_options::set(e, key, value) {
                return Ok(());
            }
            if let Effect::Configured { effect, .. } = e {
                let mut base = Self::Effect((**effect).clone());
                base.set_value(key, value)?;
                let Self::Effect(updated) = base else {
                    unreachable!()
                };
                **effect = updated;
                return Ok(());
            }
        }
        if let Self::Effect(e) = self
            && styles::set(e, key, value)
        {
            return Ok(());
        }
        if let Self::Filter(f) = self
            && adjustments::set(f, key, value)
        {
            return Ok(());
        }
        if let Self::Filter(f) = self
            && gallery::set(f, key, value)?
        {
            return Ok(());
        }

        match (self, key, value) {
            (Self::Filter(Filter::IrisBlur { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::IrisBlur { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::IrisBlur { height: v, .. }), "height", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::IrisBlur { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::IrisBlur { feather: v, .. }), "feather", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::TiltShift { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::TiltShift { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::TiltShift { transition: v, .. }),
                "transition",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::TiltShift { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PathBlur { distance: v, .. }), "distance", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PathBlur { taper: v, .. }), "taper", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PathBlur { centered: v, .. }), "centered", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SpinBlur { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SpinBlur { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SpinBlur { height: v, .. }), "height", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SpinBlur { feather: v, .. }), "feather", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SpinBlur { rotation: v, .. }), "rotation", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { height: v, .. }), "height", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { turbulence: v, .. }), "turbulence", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Flame { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { opacity: v, .. }), "opacity", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Flame { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PictureFrame { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PictureFrame { margin: v, .. }), "margin", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PictureFrame { bevel: v, .. }), "bevel", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PictureFrame { style: v, .. }), "style", Value::Choice(n)) => {
                *v = FrameStyle::ALL[n];
            }
            (Self::Filter(Filter::PictureFrame { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::PictureFrame { opacity: v, .. }),
                "opacity",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { height: v, .. }), "height", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { branching: v, .. }), "branching", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Tree { foliage: v, .. }), "foliage", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Tree { form: v, .. }), "form", Value::Choice(n)) => {
                *v = TreeForm::ALL[n];
            }
            (Self::Filter(Filter::Tree { leaf_color: v, .. }), "leaf_color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { bark_color: v, .. }), "bark_color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { opacity: v, .. }), "opacity", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tree { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NtscColors { setup: v, .. }), "setup", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Deinterlace { field: v, .. }), "field", Value::Choice(n)) => {
                *v = VideoField::ALL[n];
            }
            (
                Self::Filter(Filter::Deinterlace {
                    interpolation: v, ..
                }),
                "interpolation",
                Value::Choice(n),
            ) => {
                *v = FieldInterpolation::ALL[n];
            }
            (Self::Filter(Filter::HsbHsl { input: v, .. }), "input", Value::Choice(n)) => {
                *v = ChannelSpace::ALL[n];
            }
            (Self::Filter(Filter::HsbHsl { output: v, .. }), "output", Value::Choice(n)) => {
                *v = ChannelSpace::ALL[n];
            }

            (Self::Filter(Filter::ColorLookup { amount: v, .. }), "amount", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Displace { horizontal: v, .. }),
                "horizontal",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Displace { vertical: v, .. }), "vertical", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Displace { fit: v, .. }), "fit", Value::Choice(n)) => {
                *v = MapFit::ALL[n];
            }
            (Self::Filter(Filter::Displace { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (
                Self::Filter(Filter::Shear {
                    interpolation: v, ..
                }),
                "interpolation",
                Value::Choice(n),
            ) => {
                *v = ShearInterpolation::ALL[n];
            }
            (Self::Filter(Filter::Shear { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (Self::Filter(Filter::Shear { curve, .. }), "curve0", Value::Number(n)) => {
                curve[0] = n;
            }
            (Self::Filter(Filter::Shear { curve, .. }), "curve1", Value::Number(n)) => {
                curve[1] = n;
            }
            (Self::Filter(Filter::Shear { curve, .. }), "curve2", Value::Number(n)) => {
                curve[2] = n;
            }
            (Self::Filter(Filter::Shear { curve, .. }), "curve3", Value::Number(n)) => {
                curve[3] = n;
            }
            (Self::Filter(Filter::Shear { curve, .. }), "curve4", Value::Number(n)) => {
                curve[4] = n;
            }
            (Self::Filter(Filter::Custom { divisor: v, .. }), "divisor", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Custom { bias: v, .. }), "bias", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Custom { alpha: v, .. }), "alpha", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Custom { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k0", Value::Number(n)) => {
                kernel[0] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k1", Value::Number(n)) => {
                kernel[1] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k2", Value::Number(n)) => {
                kernel[2] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k3", Value::Number(n)) => {
                kernel[3] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k4", Value::Number(n)) => {
                kernel[4] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k5", Value::Number(n)) => {
                kernel[5] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k6", Value::Number(n)) => {
                kernel[6] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k7", Value::Number(n)) => {
                kernel[7] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k8", Value::Number(n)) => {
                kernel[8] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k9", Value::Number(n)) => {
                kernel[9] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k10", Value::Number(n)) => {
                kernel[10] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k11", Value::Number(n)) => {
                kernel[11] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k12", Value::Number(n)) => {
                kernel[12] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k13", Value::Number(n)) => {
                kernel[13] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k14", Value::Number(n)) => {
                kernel[14] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k15", Value::Number(n)) => {
                kernel[15] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k16", Value::Number(n)) => {
                kernel[16] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k17", Value::Number(n)) => {
                kernel[17] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k18", Value::Number(n)) => {
                kernel[18] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k19", Value::Number(n)) => {
                kernel[19] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k20", Value::Number(n)) => {
                kernel[20] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k21", Value::Number(n)) => {
                kernel[21] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k22", Value::Number(n)) => {
                kernel[22] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k23", Value::Number(n)) => {
                kernel[23] = n;
            }
            (Self::Filter(Filter::Custom { kernel, .. }), "k24", Value::Number(n)) => {
                kernel[24] = n;
            }

            (Self::Filter(Filter::Extrude { cell_size: v, .. }), "cell_size", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Extrude { depth: v, .. }), "depth", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Extrude { shape: v, .. }), "shape", Value::Choice(n)) => {
                *v = ExtrudeShape::ALL[n];
            }
            (Self::Filter(Filter::Extrude { random: v, .. }), "random", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Extrude { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::OilPaint { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::OilPaint { stylization: v, .. }),
                "stylization",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::OilPaint { cleanliness: v, .. }),
                "cleanliness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::OilPaint { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::OilPaint { bristles: v, .. }), "bristles", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::OilPaint { lighting: v, .. }), "lighting", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::OilPaint { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::OilPaint { shine: v, .. }), "shine", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tiles { tile_size: v, .. }), "tile_size", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Tiles { offset: v, .. }), "offset", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tiles { fill: v, .. }), "fill", Value::Choice(n)) => {
                *v = TileFill::ALL[n];
            }
            (Self::Filter(Filter::Tiles { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Tiles { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Tiles { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::TraceContour { level: v, .. }), "level", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::TraceContour { upper: v, .. }), "upper", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wind { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wind { method: v, .. }), "method", Value::Choice(n)) => {
                *v = WindMethod::ALL[n];
            }
            (Self::Filter(Filter::Wind { from_right: v, .. }), "from_right", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wind { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::LensBlur { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::LensBlur { shape: v, .. }), "shape", Value::Choice(n)) => {
                *v = ApertureShape::ALL[n];
            }
            (Self::Filter(Filter::LensBlur { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LensBlur {
                    depth_source: v, ..
                }),
                "depth_source",
                Value::Choice(n),
            ) => {
                *v = DepthSource::ALL[n];
            }
            (Self::Filter(Filter::LensBlur { focus: v, .. }), "focus", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::LensBlur { invert: v, .. }), "invert", Value::Boolean(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LensBlur { highlights: v, .. }),
                "highlights",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LensBlur { threshold: v, .. }),
                "threshold",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::LensBlur { noise: v, .. }), "noise", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::LensBlur { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (
                Self::Filter(Filter::LensFlare { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::LensFlare { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::LensFlare { style: v, .. }), "style", Value::Choice(n)) => {
                *v = FlareStyle::ALL[n];
            }
            (Self::Filter(Filter::LensFlare { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::LightingEffects { kind: v, .. }), "kind", Value::Choice(n)) => {
                *v = LightKind::ALL[n];
            }
            (
                Self::Filter(Filter::LightingEffects { intensity: v, .. }),
                "intensity",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LightingEffects { ambient: v, .. }),
                "ambient",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LightingEffects { radius: v, .. }),
                "radius",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::LightingEffects { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LightingEffects { elevation: v, .. }),
                "elevation",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::LightingEffects { gloss: v, .. }), "gloss", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LightingEffects { metallic: v, .. }),
                "metallic",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::LightingEffects { relief: v, .. }),
                "relief",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::LightingEffects { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }

            (Self::Filter(Filter::DiffuseGlow { amount: v, .. }), "amount", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DiffuseGlow { clear: v, .. }), "clear", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DiffuseGlow { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DiffuseGlow { grain: v, .. }), "grain", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DiffuseGlow { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::DiffuseGlow { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Glass { distortion: v, .. }), "distortion", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Glass { smoothness: v, .. }), "smoothness", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Glass { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Glass { texture: v, .. }), "texture", Value::Choice(n)) => {
                *v = GlassSurface::ALL[n];
            }
            (Self::Filter(Filter::Glass { invert: v, .. }), "invert", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Glass { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Glass { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (
                Self::Filter(Filter::OceanRipple { amplitude: v, .. }),
                "amplitude",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::OceanRipple { wavelength: v, .. }),
                "wavelength",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::OceanRipple { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::OceanRipple { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (Self::Filter(Filter::Texturizer { texture: v, .. }), "texture", Value::Choice(n)) => {
                *v = SurfaceTexture::ALL[n];
            }
            (Self::Filter(Filter::Texturizer { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Texturizer { relief: v, .. }), "relief", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Texturizer { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Texturizer { invert: v, .. }), "invert", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Texturizer { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::ShapeBlur { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ShapeBlur { shape: v, .. }), "shape", Value::Choice(n)) => {
                *v = ApertureShape::ALL[n];
            }
            (Self::Filter(Filter::ShapeBlur { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SmartBlur { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::SmartBlur { threshold: v, .. }),
                "threshold",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::SmartBlur { mode: v, .. }), "mode", Value::Choice(n)) => {
                *v = SmartBlurMode::ALL[n];
            }
            (Self::Filter(Filter::SmartSharpen { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SmartSharpen { amount: v, .. }), "amount", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::SmartSharpen {
                    reduce_noise: v, ..
                }),
                "reduce_noise",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::SmartSharpen { iterations: v, .. }),
                "iterations",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Value must be an integer".into());
                }
                *v = n as u8;
            }

            (Self::Filter(Filter::Fresco { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Fresco { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Fresco { texture: v, .. }), "texture", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Fresco { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Seed must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::NeonGlow { size: v, .. }), "size", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::NeonGlow { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::NeonGlow { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NeonGlow { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NeonGlow { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PaletteKnife { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::PaletteKnife { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::PaletteKnife { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::PlasticWrap { strength: v, .. }),
                "strength",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::PlasticWrap { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::PlasticWrap { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::RoughPastels { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::RoughPastels { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::RoughPastels { texture: v, .. }),
                "texture",
                Value::Choice(n),
            ) => {
                *v = SurfaceTexture::ALL[n];
            }
            (Self::Filter(Filter::RoughPastels { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::RoughPastels { relief: v, .. }), "relief", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::RoughPastels { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::RoughPastels { invert: v, .. }), "invert", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::RoughPastels { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Seed must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Sponge { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Sponge { definition: v, .. }),
                "definition",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Sponge { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Sponge { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Seed must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Underpainting { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Underpainting { coverage: v, .. }),
                "coverage",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Underpainting { texture: v, .. }),
                "texture",
                Value::Choice(n),
            ) => {
                *v = SurfaceTexture::ALL[n];
            }
            (Self::Filter(Filter::Underpainting { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Underpainting { relief: v, .. }), "relief", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Underpainting { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Underpainting { invert: v, .. }),
                "invert",
                Value::Boolean(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Underpainting { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Seed must be an integer".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Watercolor { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Watercolor { shadows: v, .. }), "shadows", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Watercolor { texture: v, .. }), "texture", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Watercolor { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Seed must be an integer".into());
                }
                *v = n as u16;
            }

            (Self::Filter(Filter::Chrome { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Chrome { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ConteCrayon { foreground: v, .. }),
                "foreground",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ConteCrayon { background: v, .. }),
                "background",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::ConteCrayon { texture: v, .. }), "texture", Value::Choice(n)) => {
                *v = SurfaceTexture::ALL[n];
            }
            (Self::Filter(Filter::ConteCrayon { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ConteCrayon { relief: v, .. }), "relief", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ConteCrayon { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ConteCrayon { invert: v, .. }), "invert", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ConteCrayon { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::ConteCrayon { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ConteCrayon { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Plaster { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Plaster { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Plaster { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Plaster { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Plaster { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Reticulation { density: v, .. }),
                "density",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Reticulation { foreground: v, .. }),
                "foreground",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Reticulation { background: v, .. }),
                "background",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Reticulation { grain_size: v, .. }),
                "grain_size",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Reticulation { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Reticulation { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Reticulation { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::TornEdges { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::TornEdges { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::TornEdges { contrast: v, .. }), "contrast", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::TornEdges { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::TornEdges { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::TornEdges { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::WaterPaper { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::WaterPaper { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::WaterPaper { contrast: v, .. }),
                "contrast",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::WaterPaper { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }

            (
                Self::Filter(Filter::SprayedStrokes { length: v, .. }),
                "length",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::SprayedStrokes { radius: v, .. }),
                "radius",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::SprayedStrokes { direction: v, .. }),
                "direction",
                Value::Choice(n),
            ) => {
                *v = StrokeDirection::ALL[n];
            }
            (Self::Filter(Filter::SprayedStrokes { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::SumiE { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SumiE { pressure: v, .. }), "pressure", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::SumiE { contrast: v, .. }), "contrast", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::BasRelief { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::BasRelief { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::BasRelief { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::BasRelief { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::BasRelief { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ChalkCharcoal { charcoal: v, .. }),
                "charcoal",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::ChalkCharcoal { chalk: v, .. }), "chalk", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ChalkCharcoal { pressure: v, .. }),
                "pressure",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::ChalkCharcoal { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::ChalkCharcoal { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ChalkCharcoal { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Charcoal { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Charcoal { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Charcoal { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Charcoal { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Charcoal { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::GraphicPen { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::GraphicPen { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::GraphicPen { direction: v, .. }),
                "direction",
                Value::Choice(n),
            ) => {
                *v = StrokeDirection::ALL[n];
            }
            (Self::Filter(Filter::GraphicPen { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::GraphicPen { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NotePaper { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NotePaper { grain: v, .. }), "grain", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NotePaper { relief: v, .. }), "relief", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NotePaper { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::NotePaper { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::NotePaper { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Stamp { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Stamp { smoothness: v, .. }), "smoothness", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Stamp { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Stamp { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }

            (Self::Filter(Filter::ColoredPencil { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ColoredPencil { pressure: v, .. }),
                "pressure",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ColoredPencil { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::ColoredPencil { paper: v, .. }), "paper", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ColoredPencil { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::DryBrush { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DryBrush { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DryBrush { texture: v, .. }), "texture", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DryBrush { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::PaintDaubs { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::PaintDaubs { sharpness: v, .. }),
                "sharpness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::PaintDaubs { style: v, .. }), "style", Value::Choice(n)) => {
                *v = DaubStyle::ALL[n];
            }
            (Self::Filter(Filter::SmudgeStick { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::SmudgeStick { highlight: v, .. }),
                "highlight",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::SmudgeStick { intensity: v, .. }),
                "intensity",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::AngledStrokes { balance: v, .. }),
                "balance",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::AngledStrokes { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::AngledStrokes { sharpness: v, .. }),
                "sharpness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Crosshatch { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Crosshatch { sharpness: v, .. }),
                "sharpness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Crosshatch { strength: v, .. }),
                "strength",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::DarkStrokes { balance: v, .. }), "balance", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DarkStrokes { black: v, .. }), "black", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DarkStrokes { white: v, .. }), "white", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Spatter { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Spatter { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Spatter { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Expected an integer seed".into());
                }
                *v = n as u16;
            }

            (Self::Filter(Filter::ColorHalftone { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ColorHalftone { cyan: v, .. }), "cyan", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ColorHalftone { magenta: v, .. }),
                "magenta",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::ColorHalftone { yellow: v, .. }), "yellow", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::ColorHalftone { black: v, .. }), "black", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Crystallize { size: v, .. }), "cell_size", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral cell_size required".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Crystallize { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Mezzotint { pattern: v, .. }), "pattern", Value::Choice(n)) => {
                *v = MezzotintStyle::ALL[n];
            }
            (
                Self::Filter(Filter::Mezzotint { scale: v, .. }),
                "pattern_scale",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Mezzotint { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (
                Self::Filter(Filter::Mezzotint { monochrome: v, .. }),
                "monochrome",
                Value::Boolean(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Pointillize { size: v, .. }), "cell_size", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral cell_size required".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Pointillize { dot: v, .. }), "dot", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Pointillize { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Pointillize { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::StainedGlass { size: v, .. }), "cell_size", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral cell_size required".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::StainedGlass { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::StainedGlass { light: v, .. }), "light", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::StainedGlass { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::StainedGlass { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::MosaicTiles { size: v, .. }), "tile_size", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral tile_size required".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::MosaicTiles { grout: v, .. }), "grout", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::MosaicTiles { lighten: v, .. }),
                "lighten",
                Value::Boolean(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Diffuse { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral radius required".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Diffuse { mode: v, .. }), "mode", Value::Choice(n)) => {
                *v = DiffuseMode::ALL[n];
            }
            (Self::Filter(Filter::Diffuse { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Cutout { levels: v, .. }), "levels", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral levels required".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::Cutout { simplicity: v, .. }),
                "simplicity",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Cutout { fidelity: v, .. }), "fidelity", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::FilmGrain { grain: v, .. }), "grain", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::FilmGrain { highlight: v, .. }),
                "highlight",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::FilmGrain { intensity: v, .. }),
                "intensity",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::FilmGrain { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::AccentedEdges { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::AccentedEdges { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::AccentedEdges { smoothness: v, .. }),
                "smoothness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::InkOutlines { length: v, .. }), "length", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::InkOutlines { dark: v, .. }), "dark", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::InkOutlines { light: v, .. }), "light", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Photocopy { detail: v, .. }), "detail", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Photocopy { darkness: v, .. }), "darkness", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Photocopy { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Photocopy { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::HalftonePattern { size: v, .. }), "size", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::HalftonePattern { contrast: v, .. }),
                "contrast",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::HalftonePattern { pattern: v, .. }),
                "pattern",
                Value::Choice(n),
            ) => {
                *v = HalftoneStyle::ALL[n];
            }
            (Self::Filter(Filter::HalftonePattern { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::HalftonePattern { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Patchwork { square: v, .. }), "square", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral square required".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Patchwork { relief: v, .. }), "relief", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Craquelure { spacing: v, .. }), "spacing", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Craquelure { depth: v, .. }), "depth", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Craquelure { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Craquelure { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Integral seed required".into());
                }
                *v = n as u16;
            }
            (
                Self::Filter(Filter::PolarCoordinates { to_polar, .. }),
                "to_polar",
                Value::Choice(n),
            ) => *to_polar = n == 0,
            (Self::Filter(Filter::ZigZag { amount, .. }), "amount", Value::Number(n)) => {
                *amount = n
            }
            (Self::Filter(Filter::ZigZag { radius, .. }), "radius", Value::Number(n)) => {
                *radius = n
            }
            (Self::Filter(Filter::ZigZag { ridges, .. }), "ridges", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Ridges must be integral".into());
                }
                *ridges = n as u8;
            }
            (Self::Filter(Filter::ZigZag { style, .. }), "style", Value::Choice(n)) => {
                *style = ZigZagStyle::ALL[n]
            }
            (Self::Filter(Filter::Fibers { seed, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Seed must be integral".into());
                }
                *seed = n as u16;
            }
            (Self::Filter(Filter::Fibers { variance, .. }), "variance", Value::Number(n)) => {
                *variance = n
            }
            (Self::Filter(Filter::Fibers { strength, .. }), "strength", Value::Number(n)) => {
                *strength = n
            }
            (Self::Filter(Filter::Fibers { low, .. }), "low", Value::Color(n)) => *low = n,
            (Self::Filter(Filter::Fibers { high, .. }), "high", Value::Color(n)) => *high = n,
            (Self::Filter(Filter::GlowingEdges { width, .. }), "width", Value::Number(n)) => {
                *width = n
            }
            (
                Self::Filter(Filter::GlowingEdges { brightness, .. }),
                "brightness",
                Value::Number(n),
            ) => *brightness = n,
            (
                Self::Filter(Filter::GlowingEdges { smoothness, .. }),
                "smoothness",
                Value::Number(n),
            ) => *smoothness = n,

            (Self::Filter(Filter::DustScratches { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::DustScratches { threshold: v, .. }),
                "threshold",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::ReduceNoise { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::ReduceNoise { strength: v, .. }),
                "strength",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ReduceNoise {
                    preserve_details: v,
                    ..
                }),
                "preserve_details",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::ReduceNoise { color_noise: v, .. }),
                "color_noise",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::ReduceNoise { sharpen: v, .. }), "sharpen", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Pinch { amount: v, .. }), "strength", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Pinch { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Spherize { amount: v, .. }), "strength", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Spherize { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Ripple { amplitude: v, .. }), "amplitude", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Ripple { wavelength: v, .. }),
                "wavelength",
                Value::Number(n),
            ) => {
                *v = n;
            }

            (Self::Filter(Filter::Fragment { distance }), "distance", Value::Number(n)) => {
                *distance = n
            }
            (Self::Filter(Filter::SurfaceBlur { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::SurfaceBlur { threshold: v, .. }),
                "threshold",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::RadialBlur { amount: v, .. }), "amount", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::RadialBlur { samples: v, .. }), "samples", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::RadialBlur { zoom: v, .. }), "zoom", Value::Choice(n)) => {
                *v = n == 1;
            }
            (
                Self::Filter(Filter::PosterEdges { thickness: v, .. }),
                "thickness",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::PosterEdges { intensity: v, .. }),
                "intensity",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::PosterEdges { levels: v, .. }), "levels", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Grain { amount: v, .. }), "strength", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Grain { contrast: v, .. }), "contrast", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Grain { size: v, .. }), "grain_size", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Grain { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u16;
            }
            (
                Self::Filter(Filter::Grain { monochrome: v, .. }),
                "monochrome",
                Value::Boolean(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Grain { style: v, .. }), "style", Value::Choice(n)) => {
                *v = GrainStyle::ALL[n];
            }
            (Self::Filter(Filter::Clouds { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u16;
            }
            (Self::Filter(Filter::Clouds { scale: v, .. }), "scale", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Clouds { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Clouds { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DifferenceClouds { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u16;
            }
            (
                Self::Filter(Filter::DifferenceClouds { scale: v, .. }),
                "scale",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::DifferenceClouds { low: v, .. }), "low", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::DifferenceClouds { high: v, .. }), "high", Value::Color(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Emboss { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Emboss { height: v, .. }), "height", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Emboss { amount: v, .. }), "strength", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::UnsharpMask { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::UnsharpMask { amount: v, .. }), "strength", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::UnsharpMask { threshold: v, .. }),
                "threshold",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Minimum { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Maximum { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Offset { x: v, .. }), "x", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as i32;
            }
            (Self::Filter(Filter::Offset { y: v, .. }), "y", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as i32;
            }
            (Self::Filter(Filter::Offset { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (Self::Filter(Filter::Twirl { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Twirl { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wave { amplitude: v, .. }), "amplitude", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wave { wavelength: v, .. }), "wavelength", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wave { vertical: v, .. }), "vertical", Value::Boolean(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::Wave { edges: v, .. }), "edges", Value::Choice(n)) => {
                *v = FilterEdges::ALL[n];
            }
            (Self::Filter(Filter::Levels { black: v, .. }), "black", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Levels { white: v, .. }), "white", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Levels { gamma: v, .. }), "gamma", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Levels {
                    output_black: v, ..
                }),
                "output_black",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (
                Self::Filter(Filter::Levels {
                    output_white: v, ..
                }),
                "output_white",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point0", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                v[0] = n as u8;
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point1", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                v[1] = n as u8;
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point2", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                v[2] = n as u8;
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point3", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                v[3] = n as u8;
            }
            (Self::Filter(Filter::Curves { points: v, .. }), "point4", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                v[4] = n as u8;
            }
            (
                Self::Filter(Filter::MotionBlur { distance: v, .. }),
                "distance",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::MotionBlur { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::HighPass { radius: v, .. }), "radius", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::Mosaic { block_size: v, .. }),
                "block_size",
                Value::Number(n),
            ) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::AddNoise { amount: v, .. }), "strength", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::AddNoise { seed: v, .. }), "seed", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u16;
            }
            (
                Self::Filter(Filter::AddNoise { monochrome: v, .. }),
                "monochrome",
                Value::Boolean(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::AddNoise {
                    distribution: v, ..
                }),
                "distribution",
                Value::Choice(n),
            ) => {
                *v = NoiseDistribution::ALL[n];
            }
            (Self::Effect(Effect::Stroke { rounded, .. }), "rounded", Value::Boolean(n)) => {
                *rounded = n
            }
            (
                Self::Effect(Effect::OuterGlow { size, .. } | Effect::InnerGlow { size, .. }),
                "radius",
                Value::Number(n),
            ) => *size = n,
            (
                Self::Filter(Filter::BrightnessContrast { brightness: v, .. }),
                "brightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::BrightnessContrast { contrast: v, .. }),
                "contrast",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Exposure { stops: v, .. }), "stops", Value::Number(n)) => {
                *v = n;
            }
            (Self::Filter(Filter::HueSaturation { hue: v, .. }), "hue", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::HueSaturation { saturation: v, .. }),
                "saturation",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (
                Self::Filter(Filter::HueSaturation { lightness: v, .. }),
                "lightness",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Vibrance { amount: v, .. }), "amount", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Filter(Filter::BlackWhite { strength: v, .. }),
                "strength",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Filter(Filter::Threshold { level: v, .. }), "level", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Posterize { levels: v, .. }), "levels", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::BoxBlur { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Filter(Filter::Median { radius: v, .. }), "radius", Value::Number(n)) => {
                if n.fract() != 0. {
                    return Err("Parameter requires a whole number".into());
                }
                *v = n as u8;
            }
            (Self::Effect(Effect::InnerShadow { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerShadow { opacity: v, .. }), "opacity", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerShadow { angle: v, .. }), "angle", Value::Number(n)) => {
                *v = n;
            }
            (
                Self::Effect(Effect::InnerShadow { distance: v, .. }),
                "distance",
                Value::Number(n),
            ) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerShadow { spread: v, .. }), "spread", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerShadow { size: v, .. }), "size", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerGlow { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerGlow { opacity: v, .. }), "opacity", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerGlow { spread: v, .. }), "spread", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::InnerGlow { size: v, .. }), "size", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::Stroke { color: v, .. }), "color", Value::Color(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::Stroke { opacity: v, .. }), "opacity", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::Stroke { width: v, .. }), "width", Value::Number(n)) => {
                *v = n;
            }
            (Self::Effect(Effect::Stroke { position: v, .. }), "position", Value::Choice(n)) => {
                *v = StrokePosition::ALL[n];
            }
            (Self::Filter(Filter::Brightness { amount }), "amount", Value::Number(n)) => {
                *amount = n
            }
            (Self::Filter(Filter::GaussianBlur { radius }), "radius", Value::Number(n)) => {
                *radius = n
            }
            (
                Self::Effect(
                    Effect::ColorOverlay { color, .. }
                    | Effect::DropShadow { color, .. }
                    | Effect::OuterGlow { color, .. },
                ),
                "color",
                Value::Color(c),
            ) => *color = c,
            (
                Self::Effect(
                    Effect::ColorOverlay { opacity, .. }
                    | Effect::DropShadow { opacity, .. }
                    | Effect::OuterGlow { opacity, .. },
                ),
                "opacity",
                Value::Number(n),
            ) => *opacity = n,
            (
                Self::Effect(Effect::DropShadow { spread, .. } | Effect::OuterGlow { spread, .. }),
                "spread",
                Value::Number(n),
            ) => *spread = n,
            (
                Self::Effect(Effect::DropShadow { size, .. } | Effect::OuterGlow { size, .. }),
                "size",
                Value::Number(n),
            ) => *size = n,
            (Self::Effect(Effect::DropShadow { angle, .. }), "angle", Value::Number(n)) => {
                *angle = n
            }
            (Self::Effect(Effect::DropShadow { distance, .. }), "distance", Value::Number(n)) => {
                *distance = n
            }
            _ => return Err("Parameter does not belong to this operation".into()),
        }
        Ok(())
    }
    /// Check every serialized parameter against the same domain as the controls.
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Effect(e) = self {
            style_options::validate(e)?;
        }
        if let Self::Filter(Filter::Levels { black, white, .. }) = self
            && black >= white
        {
            return Err("Input black must be below input white".into());
        }
        if let Self::Filter(f) = self {
            editable::validate_assets(f)?;
            gallery::validate(f)?;
        }
        let mut copy = self.clone();
        for p in self.kind().descriptor().parameters {
            copy.set_value(p.key, self.value(p.key).expect("registered parameter"))?;
        }
        Ok(())
    }
    /// Scale image-space parameters for a reduced-resolution preview.
    pub fn scaled(&self, scale: f32) -> Self {
        if let Self::Effect(Effect::Configured { effect, options }) = self {
            let Self::Effect(effect) = Self::Effect((**effect).clone()).scaled(scale) else {
                unreachable!()
            };
            let mut options = options.clone();
            for v in options
                .material_offset
                .iter_mut()
                .chain(options.texture_offset.iter_mut())
            {
                *v *= scale;
            }
            return Self::Effect(Effect::Configured {
                effect: Box::new(effect),
                options,
            });
        }
        if let Self::Effect(e) = self
            && let Some(e) = styles::scaled(e, scale)
        {
            return Self::Effect(e);
        }
        let mut result = self.clone();

        if let Self::Filter(Filter::FieldBlur { pins, .. }) = &mut result {
            for p in pins {
                p.radius *= scale;
            }
            return result;
        }
        for key in [
            "transition",
            "margin",
            "bevel",
            "horizontal",
            "vertical",
            "depth",
            "distortion",
            "shadow_radius",
            "highlight_radius",
            "radius",
            "size",
            "distance",
            "width",
            "block_size",
            "scale",
            "grain_size",
            "thickness",
            "height",
            "amplitude",
            "wavelength",
            "smoothness",
            "simplicity",
            "length",
            "square",
            "spacing",
            "cell_size",
            "tile_size",
            "grout",
            "pattern_scale",
            "x",
            "y",
        ] {
            // Neon size selects a tonal band, rather than an image-space extent.
            if (key == "size" && self.kind() == Kind::NeonGlow)
                || (key == "smoothness" && self.kind() == Kind::Glass)
            {
                continue;
            }
            if let Some(Value::Number(n)) = self.value(key) {
                result
                    .set_value(
                        key,
                        Value::Number(
                            if (matches!(
                                self,
                                Self::Filter(Filter::Extrude { .. } | Filter::Tiles { .. })
                            ) && matches!(key, "cell_size" | "tile_size"))
                                || matches!(
                                    self,
                                    Self::Filter(
                                        Filter::SmartBlur { .. }
                                            | Filter::Crystallize { .. }
                                            | Filter::Pointillize { .. }
                                            | Filter::StainedGlass { .. }
                                            | Filter::MosaicTiles { .. }
                                            | Filter::Diffuse { .. }
                                            | Filter::Patchwork { .. }
                                            | Filter::DustScratches { .. }
                                            | Filter::ReduceNoise { .. }
                                            | Filter::SurfaceBlur { .. }
                                            | Filter::PosterEdges { .. }
                                            | Filter::Minimum { .. }
                                            | Filter::Maximum { .. }
                                            | Filter::Emboss { .. }
                                            | Filter::Offset { .. }
                                            | Filter::BoxBlur { .. }
                                            | Filter::Median { .. }
                                            | Filter::Mosaic { .. }
                                    )
                                )
                            {
                                (n * scale).round().max(
                                    if matches!(key, "block_size" | "height" | "square") {
                                        1.
                                    } else if key == "cell_size" {
                                        if self.kind() == Kind::Extrude { 4. } else { 2. }
                                    } else if key == "tile_size" {
                                        if self.kind() == Kind::Tiles { 2. } else { 1. }
                                    } else if matches!(key, "x" | "y") {
                                        -16384.
                                    } else {
                                        0.
                                    },
                                )
                            } else {
                                (n * scale).max(
                                    if key == "scale" && self.kind() == Kind::OilPaint {
                                        0.5
                                    } else if matches!(key, "wavelength" | "scale")
                                        || (key == "size"
                                            && matches!(
                                                self,
                                                Self::Filter(Filter::HalftonePattern { .. })
                                            ))
                                    {
                                        1.
                                    } else if key == "pattern_scale" {
                                        0.05
                                    } else if key == "radius"
                                        && matches!(
                                            self,
                                            Self::Filter(Filter::ColorHalftone { .. })
                                        )
                                    {
                                        0.5
                                    } else if key == "spacing"
                                        || (key == "length"
                                            && matches!(
                                                self,
                                                Self::Filter(Filter::Crosshatch { .. })
                                            ))
                                    {
                                        2.
                                    } else if key == "length"
                                        && matches!(
                                            self,
                                            Self::Filter(
                                                Filter::SprayedStrokes { .. } | Filter::Wind { .. }
                                            )
                                        )
                                    {
                                        0.
                                    } else if matches!(key, "length" | "grain_size")
                                        || (key == "width"
                                            && matches!(
                                                self,
                                                Self::Filter(
                                                    Filter::Charcoal { .. }
                                                        | Filter::ColoredPencil { .. }
                                                        | Filter::GlowingEdges { .. }
                                                        | Filter::AccentedEdges { .. }
                                                )
                                            ))
                                    {
                                        0.5
                                    } else {
                                        f32::NEG_INFINITY
                                    },
                                )
                            },
                        ),
                    )
                    .expect("downscaled parameter");
            }
        }
        // Most "amount" parameters are dimensionless. ZigZag's displacement
        // is in pixels, so it must shrink together with its support radius.
        if matches!(self, Self::Filter(Filter::ZigZag { .. }))
            && let Some(Value::Number(amount)) = self.value("amount")
        {
            result
                .set_value("amount", Value::Number(amount * scale))
                .expect("downscaled displacement");
        }
        result
    }
}

/// Styles have transparent padding beyond the source; preflight it at validation
/// rather than letting a render-worker allocation error become a panic.
pub fn effect_padding(effect: &Effect) -> Option<u32> {
    if let Effect::Configured { effect, .. } = effect {
        return effect_padding(effect);
    }
    if let Some(pad) = styles::padding(effect) {
        return pad;
    }
    let (spread, size) = match effect {
        Effect::Configured { .. } => unreachable!("unwrapped effect configuration"),
        Effect::ColorOverlay { .. } => return None,
        Effect::BevelEmboss { .. }
        | Effect::Satin { .. }
        | Effect::GradientOverlay { .. }
        | Effect::PatternOverlay { .. } => unreachable!("handled style"),
        Effect::Stroke { width, .. } => return Some(width.ceil() as u32 + 2),
        Effect::DropShadow { spread, size, .. }
        | Effect::OuterGlow { spread, size, .. }
        | Effect::InnerShadow { spread, size, .. }
        | Effect::InnerGlow { spread, size, .. } => (*spread, *size),
    };
    Some(gaussian::support(size * (1. - spread)) + (size * spread).ceil() as u32 + 2)
}

/// Reduced document for parameter previews and thumbnails. All spatial settings
/// scale with the pixels; the original document and its source assets are intact.
pub fn preview_document(
    document: &crate::document::Document,
    max_side: u32,
) -> Result<crate::document::Document, String> {
    if max_side == 0 {
        return Err("Preview size must be positive".into());
    }
    let mut doc = document.clone();
    let scale = (max_side as f32 / doc.width.max(doc.height) as f32).min(1.);
    if scale < 1. {
        let w = (doc.width as f32 * scale).round().max(1.) as u32;
        let h = (doc.height as f32 * scale).round().max(1.) as u32;
        crate::transform::resize(&mut doc, w, h, crate::transform::Resample::Bilinear)?;
        for layer in &mut doc.layers {
            for f in &mut layer.filters {
                let Operation::Filter(filter) = Operation::Filter(f.filter.clone()).scaled(scale)
                else {
                    unreachable!()
                };
                f.filter = filter;
            }
            for e in &mut layer.effects {
                let Operation::Effect(effect) = Operation::Effect(e.effect.clone()).scaled(scale)
                else {
                    unreachable!()
                };
                e.effect = effect;
            }
        }
    }
    Ok(doc)
}
