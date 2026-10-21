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
                rounded: false,
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
            rounded: false,
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
                processing::ParameterType::Boolean => Value::Boolean(true),
                processing::ParameterType::Choice(options) => Value::Choice(options.len() - 1),
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
        (Kind::Minimum, "radius", Value::Number(1.5)),
        (Kind::Offset, "x", Value::Number(0.5)),
        (Kind::Wave, "wavelength", Value::Number(0.)),
        (Kind::Twirl, "angle", Value::Number(f32::NAN)),
        (Kind::Offset, "edges", Value::Choice(3)),
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
                rounded: false,
            }),
        );
    }
    assert!(large.validate().unwrap_err().contains("512 MiB"));
    large.active_mut().effects[0].enabled = false;
    large.validate().unwrap();
}

#[test]
fn levels_and_curves_match_independent_analytic_values_and_reject_crossed_input_limits() {
    for n in 0..=255 {
        let p = [n, 255 - n, n / 2, 73];
        assert_eq!(
            point(
                Filter::Curves {
                    points: [0, 64, 128, 192, 255]
                },
                p
            ),
            p
        );
        assert_eq!(
            point(
                Filter::Levels {
                    black: 0,
                    white: 255,
                    gamma: 1.,
                    output_black: 0,
                    output_white: 255
                },
                p
            ),
            p
        );
    }
    assert_eq!(
        point(
            Filter::Levels {
                black: 20,
                white: 220,
                gamma: 2.,
                output_black: 10,
                output_white: 210
            },
            [20, 70, 220, 81]
        ),
        [10, 110, 210, 81]
    );
    // A straight descending curve interpolates exactly, including the unequal last interval.
    for n in 0..=255 {
        assert_eq!(
            point(
                Filter::Curves {
                    points: [255, 191, 127, 63, 0]
                },
                [n, n, n, 99]
            ),
            [255 - n, 255 - n, 255 - n, 99]
        );
    }
    let mut op = Kind::Levels.default_operation();
    op.set_value("black", Value::Number(200.)).unwrap();
    let before = op.clone();
    assert!(op.set_value("white", Value::Number(200.)).is_err());
    assert!(op == before);
    let mut doc = shape();
    add(
        &mut doc,
        Operation::Filter(Filter::Levels {
            black: 90,
            white: 80,
            gamma: 1.,
            output_black: 0,
            output_white: 255,
        }),
    );
    assert!(doc.validate().is_err());
}

#[test]
fn motion_mosaic_and_high_pass_obey_alpha_edges_reference_values_and_cancellation() {
    let mut source = Raster::new(3, 1, [0; 4]).unwrap();
    source.set(0, 0, [255, 0, 0, 255]);
    source.set(1, 0, [0, 255, 0, 0]); // Hidden green must not bleed into either average.
    source.set(2, 0, [0, 0, 255, 255]);
    for angle in [0., 180.] {
        let result = processing::motion_blur_cancel(&source, 2., angle, &|| false).unwrap();
        assert_eq!(result.get(1, 0), [128, 0, 128, 170]);
        assert_eq!(result.get(0, 0), [255, 0, 0, 170]);
        assert_eq!(result.get(2, 0), [0, 0, 255, 170]);
    }
    assert!(processing::motion_blur_cancel(&source, 0., 0., &|| false).unwrap() == source);
    let mosaic = processing::mosaic_cancel(&source, 2, &|| false).unwrap();
    assert_eq!(mosaic.get(0, 0), [255, 0, 0, 128]);
    assert_eq!(mosaic.get(1, 0), [255, 0, 0, 128]);
    assert_eq!(mosaic.get(2, 0), [0, 0, 255, 255]); // Partial last cell has its actual area.
    let constant = Raster::new(5, 4, [80, 120, 160, 77]).unwrap();
    for radius in [0., 0.7, 4.] {
        let high = processing::high_pass_cancel(&constant, radius, &|| false).unwrap();
        assert!(
            high.rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p == &[128, 128, 128, 77])
        );
    }
    let high = processing::high_pass_cancel(&source, 0., &|| false).unwrap();
    assert_eq!(high.get(1, 0), [0; 4]);
    assert!(processing::motion_blur_cancel(&source, 3., 42., &|| true).is_err());
    assert!(processing::mosaic_cancel(&source, 2, &|| true).is_err());
    assert!(processing::high_pass_cancel(&source, 2., &|| true).is_err());
}

#[test]
fn seeded_noise_is_coordinate_stable_across_point_spatial_and_region_paths() {
    use palette_core::document::NoiseDistribution;
    for distribution in NoiseDistribution::ALL {
        for monochrome in [true, false] {
            let mut doc = Document::from_raster(Raster::new(270, 9, [128, 128, 128, 255]).unwrap());
            add(
                &mut doc,
                Operation::Filter(Filter::AddNoise {
                    amount: 0.2,
                    seed: 123,
                    monochrome,
                    distribution,
                }),
            );
            let output = composite::render(&doc);
            let sum: u64 = output
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| {
                    assert_eq!(p[3], 255);
                    if monochrome {
                        assert!(p[0] == p[1] && p[1] == p[2]);
                    }
                    u64::from(p[0])
                })
                .sum();
            assert!((sum as f64 / (270. * 9.) - 128.).abs() < 3.);
            assert!(output.rgba().as_chunks::<4>().0.iter().any(|p| p[0] != 128));
            if !monochrome {
                assert!(
                    output
                        .rgba()
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|p| p[0] != p[1])
                );
            }
            for x in [0, 1, 127, 128, 255, 256, 269] {
                assert_eq!(composite::layer_pixel(doc.active(), x, 3), output.get(x, 3));
            }
            let id = doc.allocate_id();
            doc.active_mut().filters.insert(
                0,
                FilterInstance {
                    id,
                    enabled: true,
                    filter: Filter::BoxBlur { radius: 1 },
                    mask: None,
                },
            );
            assert!(composite::render(&doc).rgba() == output.rgba());
            let region = composite::render_region(
                &doc,
                PixelRect {
                    x: 125,
                    y: 2,
                    width: 135,
                    height: 4,
                },
            )
            .unwrap();
            for y in 0..4 {
                for x in 0..135 {
                    assert_eq!(region.get(x, y), output.get(x + 125, y + 2));
                }
            }
            if let Filter::AddNoise { seed, .. } = &mut doc.active_mut().filters[1].filter {
                *seed = 124;
            }
            assert!(composite::render(&doc).rgba() != output.rgba());
        }
    }
    assert_eq!(
        point(
            Filter::AddNoise {
                amount: 0.,
                seed: 0,
                monochrome: false,
                distribution: NoiseDistribution::Gaussian
            },
            [21, 83, 190, 17]
        ),
        [21, 83, 190, 17]
    );
}

#[test]
fn rounded_stroke_contours_differ_from_square_and_old_v4_projects_keep_square_corners() {
    let mut doc = shape();
    let mut op = Kind::Stroke.default_operation();
    op.set_value("width", Value::Number(3.)).unwrap();
    op.set_value("opacity", Value::Number(1.)).unwrap();
    add(&mut doc, op);
    let source = doc.active().source.clone();
    let round = composite::render(&doc);
    assert_eq!(round.get(3, 3)[3], 0); // Distance sqrt(18) is outside radius 3 plus its AA fringe.
    assert_eq!(round.get(3, 6)[3], 255);
    if let Effect::Stroke { rounded, .. } = &mut doc.active_mut().effects[0].effect {
        *rounded = false;
    }
    let square = composite::render(&doc);
    assert_eq!(square.get(3, 3)[3], 255);
    assert!(doc.active().source == source);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stroke.palette");
    io::save_project(&path, &doc).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[8..12], &8u32.to_le_bytes());
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut manifest: serde_json::Value = serde_json::from_slice(&bytes[16..16 + len]).unwrap();
    manifest["layers"][0]["effects"][0]["effect"]["Stroke"]
        .as_object_mut()
        .unwrap()
        .remove("rounded");
    let json = serde_json::to_vec(&manifest).unwrap();
    let mut legacy = bytes[..8].to_vec();
    legacy.extend(4u32.to_le_bytes());
    legacy.extend((json.len() as u32).to_le_bytes());
    legacy.extend(json);
    legacy.extend(&bytes[16 + len..]);
    std::fs::write(&path, &legacy).unwrap();
    let migrated = io::open(&path).unwrap();
    assert!(matches!(
        migrated.active().effects[0].effect,
        Effect::Stroke { rounded: false, .. }
    ));
    assert!(composite::render(&migrated).rgba() == square.rgba());
    assert_eq!(std::fs::read(&path).unwrap(), legacy);
}
