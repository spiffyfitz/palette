use palette_core::{
    composite,
    document::{Document, Effect, EffectInstance, Filter, FilterInstance, StrokePosition},
    editing::PixelRect,
    io,
    processing::{self, Kind, Operation, Value},
    raster::{Coverage, Raster},
};
use std::{cell::Cell, sync::Arc};

fn add(doc: &mut Document, op: Operation) {
    let id = doc.allocate_id();
    match op {
        Operation::Filter(filter) => doc.active_mut().filters.push(FilterInstance {
            id,
            enabled: true,
            filter,
            mask: None,
        }),
        Operation::Effect(effect) => doc.active_mut().effects.push(EffectInstance {
            id,
            enabled: true,
            effect,
        }),
    }
}
fn point(filter: Filter, pixel: [u8; 4]) -> [u8; 4] {
    let mut doc = Document::from_raster(Raster::new(1, 1, pixel).unwrap());
    add(&mut doc, Operation::Filter(filter));
    doc.validate().unwrap();
    composite::layer_pixel(doc.active(), 0, 0)
}
fn shape() -> Document {
    let mut doc = Document::from_raster(Raster::new(20, 20, [0; 4]).unwrap());
    for y in 6..14 {
        for x in 6..14 {
            doc.active_mut().source.set(x, y, [180, 180, 180, 255]);
        }
    }
    doc
}

#[test]
fn adjustments_match_analytic_values_preserve_alpha_and_have_exact_neutral_settings() {
    for pixel in [[0, 0, 0, 0], [14, 95, 245, 23], [255, 20, 0, 128], [255; 4]] {
        for f in [
            Filter::BrightnessContrast {
                brightness: 0.,
                contrast: 0.,
            },
            Filter::Exposure { stops: 0. },
            Filter::HueSaturation {
                hue: 0.,
                saturation: 0.,
                lightness: 0.,
            },
            Filter::Vibrance { amount: 0. },
            Filter::BlackWhite { strength: 0. },
        ] {
            assert_eq!(point(f, pixel), pixel);
        }
    }
    // Linear-light 0.5 doubles to white at +1 EV; encoded sRGB is ~188.
    assert_eq!(
        point(Filter::Exposure { stops: 1. }, [188, 0, 0, 77]),
        [255, 0, 0, 77]
    );
    assert_eq!(
        point(
            Filter::HueSaturation {
                hue: 120.,
                saturation: 0.,
                lightness: 0.
            },
            [255, 0, 0, 33]
        ),
        [0, 255, 0, 33]
    );
    assert_eq!(
        point(
            Filter::HueSaturation {
                hue: 0.,
                saturation: -1.,
                lightness: 0.
            },
            [255, 0, 0, 33]
        ),
        [128, 128, 128, 33]
    );
    assert_eq!(
        point(Filter::BlackWhite { strength: 1. }, [255, 0, 0, 17]),
        [54, 54, 54, 17]
    );
    assert_eq!(
        point(Filter::Threshold { level: 128 }, [127, 127, 127, 99]),
        [0, 0, 0, 99]
    );
    assert_eq!(
        point(Filter::Threshold { level: 128 }, [128, 128, 128, 99]),
        [255, 255, 255, 99]
    );
    assert_eq!(
        point(Filter::Posterize { levels: 3 }, [50, 130, 230, 44]),
        [0, 128, 255, 44]
    );
    let vibrant = point(Filter::Vibrance { amount: 1. }, [160, 120, 120, 77]);
    assert!(vibrant[0] > 160 && vibrant[1] < 120 && vibrant[3] == 77);
    assert_eq!(
        point(
            Filter::BrightnessContrast {
                brightness: 0.1,
                contrast: 0.
            },
            [100, 100, 100, 20]
        ),
        [126, 126, 126, 20]
    );
}

#[test]
fn box_and_median_match_independent_neighborhood_references_and_cancel_without_partial_outputs() {
    let mut source = Raster::new(520, 3, [0; 4]).unwrap();
    for y in 0..3 {
        for x in 0..520 {
            source.set(
                x,
                y,
                [
                    (x % 251) as u8,
                    (x * 3 % 255) as u8,
                    200,
                    if x % 3 == 0 { 0 } else { 180 },
                ],
            );
        }
    }
    let boxed = processing::box_blur_cancel(&source, 2, &|| false).unwrap();
    let median = processing::median_cancel(&source, 1, &|| false).unwrap();
    for x in [0, 1, 254, 255, 256, 257, 519] {
        for y in 0..3 {
            let mut sums = [0f64; 4];
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let p = source.get(
                        (x as i64 + dx).clamp(0, 519) as u32,
                        (y as i64 + dy).clamp(0, 2) as u32,
                    );
                    for c in 0..3 {
                        sums[c] += f64::from(p[c]) * f64::from(p[3]);
                    }
                    sums[3] += f64::from(p[3]);
                }
            }
            let actual = boxed.get(x, y);
            for c in 0..3 {
                assert!(actual[c].abs_diff((sums[c] / sums[3]).round() as u8) <= 1);
            }
            assert!(actual[3].abs_diff((sums[3] / 25.).round() as u8) <= 1);
            let mut channels: [Vec<u32>; 4] = std::array::from_fn(|_| vec![]);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let p = source.get(
                        (x as i64 + dx).clamp(0, 519) as u32,
                        (y as i64 + dy).clamp(0, 2) as u32,
                    );
                    for c in 0..3 {
                        channels[c].push(u32::from(p[c]) * u32::from(p[3]));
                    }
                    channels[3].push(u32::from(p[3]) * 255);
                }
            }
            for values in &mut channels {
                values.sort_unstable();
            }
            let a = channels[3][4];
            let expected = if a == 0 {
                [0; 4]
            } else {
                [
                    (0..3)
                        .map(|c| ((channels[c][4] * 255 + a / 2) / a).min(255) as u8)
                        .collect::<Vec<_>>(),
                    vec![((a + 127) / 255) as u8],
                ]
                .concat()
                .try_into()
                .unwrap()
            };
            assert_eq!(median.get(x, y), expected);
        }
    }
    for filter in [processing::box_blur_cancel, processing::median_cancel] {
        assert!(filter(&source, 0, &|| false).unwrap() == source);
        let count = Cell::new(0);
        assert!(
            filter(&source, 2, &|| {
                count.set(count.get() + 1);
                count.get() > 1
            })
            .is_err()
        );
        let constant = Raster::new(4, 3, [80, 130, 200, 150]).unwrap();
        assert!(filter(&constant, 2, &|| false).unwrap().rgba() == constant.rgba());
    }
}

#[test]
fn inner_styles_preserve_coverage_and_strokes_obey_position_subpixel_width_and_source_edits() {
    for (kind, expected) in [(Kind::InnerShadow, false), (Kind::InnerGlow, true)] {
        let mut doc = shape();
        let mut op = kind.default_operation();
        op.set_value("size", Value::Number(0.)).unwrap();
        op.set_value("spread", Value::Number(0.)).unwrap();
        if kind == Kind::InnerShadow {
            op.set_value("distance", Value::Number(2.)).unwrap();
            op.set_value("angle", Value::Number(180.)).unwrap();
        } else {
            op.set_value("size", Value::Number(1.)).unwrap();
        }
        add(&mut doc, op);
        let rendered = composite::render(&doc);
        for y in 0..20 {
            for x in 0..20 {
                assert_eq!(rendered.get(x, y)[3], doc.active().source.get(x, y)[3]);
            }
        }
        assert_eq!(rendered.get(10, 10), [180, 180, 180, 255]);
        assert_eq!(rendered.get(6, 10)[0] > 180, expected);
        assert_ne!(rendered.get(6, 10)[0], 180);
    }
    for position in StrokePosition::ALL {
        let mut doc = shape();
        let source = doc.active().source.clone();
        add(
            &mut doc,
            Operation::Effect(Effect::Stroke {
                color: [255, 0, 0],
                opacity: 1.,
                width: if position == StrokePosition::Center {
                    4.
                } else {
                    2.
                },
                position,
            }),
        );
        let rendered = composite::render(&doc);
        assert_eq!(
            rendered.get(4, 10)[3] > 0,
            position != StrokePosition::Inside
        );
        assert_eq!(
            rendered.get(6, 10),
            if position == StrokePosition::Outside {
                [180, 180, 180, 255]
            } else {
                [255, 0, 0, 255]
            }
        );
        assert_eq!(rendered.get(10, 10), [180, 180, 180, 255]);
        assert!(doc.active().source == source);
        doc.active_mut().source.set(10, 10, [0; 4]);
        assert!(composite::render(&doc).rgba() != rendered.rgba());
    }
    let mut doc = shape();
    add(
        &mut doc,
        Operation::Effect(Effect::Stroke {
            color: [255, 0, 0],
            opacity: 1.,
            width: 0.5,
            position: StrokePosition::Outside,
        }),
    );
    assert_eq!(composite::render(&doc).get(5, 10), [255, 0, 0, 128]);
    doc.active_mut().opacity = 0.5;
    assert_eq!(composite::render(&doc).get(5, 10)[3], 64);
}

#[test]
fn every_catalog_entry_roundtrips_sources_masks_settings_and_baked_paint_appearance() {
    let dir = tempfile::tempdir().unwrap();
    for descriptor in processing::REGISTRY {
        let mut doc = shape();
        doc.width = 28;
        doc.height = 24;
        doc.active_mut().offset = [-2, 3];
        let source = doc.active().source.clone();
        let mut op = descriptor.kind.default_operation();
        // Exercise nondefault values, rather than only round-tripping neutral
        // settings whose fields could accidentally be exchanged undetected.
        for parameter in descriptor.parameters {
            let value = match parameter.kind {
                processing::ParameterType::Number { min, max, step } => {
                    let n = min + (max - min) * 0.37;
                    Value::Number(if step >= 1. { n.round() } else { n })
                }
                processing::ParameterType::Color => Value::Color([51, 117, 203]),
                processing::ParameterType::Choice(_) => Value::Choice(2),
            };
            op.set_value(parameter.key, value).unwrap();
        }
        add(&mut doc, op);
        doc.active_mut().mask = Some(Coverage {
            width: 20,
            height: 20,
            values: Arc::new(
                (0..400)
                    .map(|i| if i % 20 < 12 { 255 } else { 128 })
                    .collect(),
            ),
            enabled: true,
        });
        if let Some(f) = doc.active_mut().filters.first_mut() {
            f.mask = Some(Coverage {
                width: 20,
                height: 20,
                values: Arc::new(vec![160; 400]),
                enabled: true,
            });
        }
        doc.validate().unwrap();
        let rendered = composite::render(&doc);
        let region = PixelRect {
            x: 2,
            y: 2,
            width: 19,
            height: 18,
        };
        let patch = composite::render_region(&doc, region).unwrap();
        for y in 0..18 {
            for x in 0..19 {
                assert_eq!(patch.get(x, y), rendered.get(x + 2, y + 2));
            }
        }
        let path = dir.path().join("catalog.palette");
        io::save_project(&path, &doc).unwrap();
        assert!(io::open(&path).unwrap() == doc, "{:?}", descriptor.kind);
        let compatible = io::paint::compatible_copy(&doc).unwrap();
        let path = dir.path().join("catalog.paint");
        io::paint::save_paint(&path, &compatible).unwrap();
        assert!(
            composite::render(&io::open(&path).unwrap()).rgba() == rendered.rgba(),
            "{:?}",
            descriptor.kind
        );
        assert!(doc.active().source == source);
    }
}

#[test]
fn typed_catalog_parameters_reject_bad_integer_choice_and_nonfinite_values_before_saving() {
    for (kind, key, value) in [
        (Kind::Posterize, "levels", Value::Number(3.5)),
        (Kind::Median, "radius", Value::Number(6.)),
        (Kind::Stroke, "position", Value::Choice(3)),
        (Kind::Exposure, "stops", Value::Number(f32::NAN)),
    ] {
        let mut op = kind.default_operation();
        let before = op.clone();
        assert!(op.set_value(key, value).is_err());
        assert!(op == before);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("safe.palette");
    let mut doc = shape();
    io::save_project(&path, &doc).unwrap();
    let saved = std::fs::read(&path).unwrap();
    add(&mut doc, Operation::Filter(Filter::Posterize { levels: 1 }));
    assert!(io::save_project(&path, &doc).is_err());
    assert_eq!(std::fs::read(path).unwrap(), saved);
    // Center strokes retain two planes; allocation preflight counts both.
    let mut large = Document::new(4000, 4000).unwrap();
    for _ in 0..4 {
        add(
            &mut large,
            Operation::Effect(Effect::Stroke {
                color: [0; 3],
                opacity: 1.,
                width: 64.,
                position: StrokePosition::Center,
            }),
        );
    }
    assert!(large.validate().unwrap_err().contains("512 MiB"));
    large.active_mut().effects[0].enabled = false;
    large.validate().unwrap();
}
