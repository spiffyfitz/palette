//! Implemented processing catalog and shared, UI-independent parameter metadata.
//! The renderer never knows widget types; adding an operation extends this
//! registry and its evaluator rather than introducing a second UI parameter model.
mod extended;
mod gaussian;
mod layer;
mod neighborhood;
mod point;
mod round;
use crate::document::{Effect, Filter, NoiseDistribution, StrokePosition};
pub use extended::{high_pass_cancel, mosaic_cancel, motion_blur_cancel};
pub use gaussian::{gaussian_blur, gaussian_blur_cancel};
pub use layer::{PreparedLayer, has_neighborhood, prepare_layer, prepare_layer_cancel};
pub use neighborhood::{box_blur_cancel, median_cancel};
pub(crate) use point::point_filter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Stable identifiers for the implemented processing catalog.
pub enum Kind {
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
const SIZE: Parameter = number("size", "Size (px)", 0., 64., 0.1);
/// Implemented operations only; unfinished catalog entries are never registered.
pub const REGISTRY: &[Descriptor] = &[
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
        parameters: &[COLOR, OPACITY],
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
        match self {
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
        match self {
            Self::Filter(Filter::BrightnessContrast { .. }) => Kind::BrightnessContrast,
            Self::Filter(Filter::Exposure { .. }) => Kind::Exposure,
            Self::Filter(Filter::HueSaturation { .. }) => Kind::HueSaturation,
            Self::Filter(Filter::Vibrance { .. }) => Kind::Vibrance,
            Self::Filter(Filter::BlackWhite { .. }) => Kind::BlackWhite,
            Self::Filter(Filter::Threshold { .. }) => Kind::Threshold,
            Self::Filter(Filter::Posterize { .. }) => Kind::Posterize,
            Self::Filter(Filter::BoxBlur { .. }) => Kind::BoxBlur,
            Self::Filter(Filter::Median { .. }) => Kind::Median,
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
        match (self, key) {
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
        let key = if key == "size"
            && matches!(
                self,
                Self::Effect(Effect::OuterGlow { .. } | Effect::InnerGlow { .. })
            ) {
            "radius"
        } else {
            key
        };
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
        match (self, key, value) {
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
        if let Self::Filter(Filter::Levels { black, white, .. }) = self
            && black >= white
        {
            return Err("Input black must be below input white".into());
        }
        let mut copy = self.clone();
        for p in self.kind().descriptor().parameters {
            copy.set_value(p.key, self.value(p.key).expect("registered parameter"))?;
        }
        Ok(())
    }
    /// Scale image-space parameters for a reduced-resolution preview.
    pub fn scaled(&self, scale: f32) -> Self {
        let mut result = self.clone();
        for key in ["radius", "size", "distance", "width", "block_size"] {
            if let Some(Value::Number(n)) = self.value(key) {
                result
                    .set_value(
                        key,
                        Value::Number(
                            if matches!(
                                self,
                                Self::Filter(
                                    Filter::BoxBlur { .. }
                                        | Filter::Median { .. }
                                        | Filter::Mosaic { .. }
                                )
                            ) {
                                (n * scale)
                                    .round()
                                    .max(if key == "block_size" { 1. } else { 0. })
                            } else {
                                n * scale
                            },
                        ),
                    )
                    .expect("downscaled parameter");
            }
        }
        result
    }
}

/// Styles have transparent padding beyond the source; preflight it at validation
/// rather than letting a render-worker allocation error become a panic.
pub fn effect_padding(effect: &Effect) -> Option<u32> {
    let (spread, size) = match effect {
        Effect::ColorOverlay { .. } => return None,
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
