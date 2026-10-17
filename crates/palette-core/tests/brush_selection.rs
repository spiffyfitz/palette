use palette_core::{
    document::{Document, Filter, FilterInstance},
    history::History,
    io,
    raster::Raster,
    selection::{
        self, Combine,
        brush::{Settings, Stroke, from_points},
    },
};

fn settings() -> Settings {
    Settings {
        diameter: 20.,
        hardness: 1.,
        ..Default::default()
    }
}

#[test]
fn edge_aware_strokes_stop_at_barriers_and_tolerance_expands_monotonically() {
    let mut doc = Document::new(80, 30).unwrap();
    for y in 0..30 {
        for x in 0..80 {
            doc.active_mut().source.set(
                x,
                y,
                if (35..40).contains(&x) {
                    [0, 0, 255, 255]
                } else {
                    [220, 10, 10, 255]
                },
            );
        }
    }
    let path = [[10.5, 15.5], [65.5, 15.5]];
    let low = from_points(&doc, &path, settings()).unwrap();
    assert_eq!(low.values[15 * 80 + 25], 255);
    assert_eq!(low.values[15 * 80 + 37], 0);
    assert_eq!(
        low.values[15 * 80 + 55],
        0,
        "cannot jump a contrasting barrier"
    );
    let high = from_points(
        &doc,
        &path,
        Settings {
            tolerance: 100.,
            ..settings()
        },
    )
    .unwrap();
    assert_eq!(high.values[15 * 80 + 55], 255);
    assert_eq!(high.values[15 * 80 + 37], 255);
    let ramp = Document::from_raster(
        Raster::from_rgba(
            80,
            30,
            &(0..30)
                .flat_map(|_| (0..80).flat_map(|x| [x * 3, x * 3, x * 3, 255]))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    );
    let mut previous = vec![0; 2400];
    for tolerance in [0., 1., 4., 12., 35., 100.] {
        let mask = from_points(
            &ramp,
            &path,
            Settings {
                tolerance,
                ..settings()
            },
        )
        .unwrap();
        assert!(previous.iter().zip(mask.values.iter()).all(|(a, b)| a <= b));
        previous = mask.values.as_ref().clone();
    }
}

#[test]
fn soft_capsules_have_no_gaps_and_maximum_strength_is_independent_of_event_rate() {
    let doc = Document::new(100, 40).unwrap();
    let opts = Settings {
        edge_aware: false,
        strength: 0.5,
        hardness: 0.,
        ..settings()
    };
    let straight = from_points(&doc, &[[10.5, 20.5], [85.5, 20.5]], opts).unwrap();
    let many: Vec<_> = (10..=85).map(|x| [x as f32 + 0.5, 20.5]).collect();
    let frequent = from_points(&doc, &many, opts).unwrap();
    assert_eq!(straight, frequent);
    assert!((10..=85).all(|x| straight.values[20 * 100 + x] == 128));
    assert_eq!(straight.values[0], 0);
    assert!(straight.values[25 * 100 + 50] > 0 && straight.values[25 * 100 + 50] < 128);
    let hard = from_points(
        &doc,
        &many,
        Settings {
            hardness: 1.,
            ..opts
        },
    )
    .unwrap();
    assert!(hard.values[25 * 100 + 50] > straight.values[25 * 100 + 50]);
    let mut incremental = Stroke::new(&doc, many[0], opts).unwrap();
    for (i, p) in many.iter().enumerate().skip(1) {
        incremental.extend(*p).unwrap();
        if i % 10 == 0 {
            let _preview = incremental.coverage().unwrap();
        }
    }
    assert_eq!(incremental.coverage().unwrap(), straight);
    let single = from_points(
        &doc,
        &[[10., 10.], [25., 25.]],
        Settings {
            diameter: 1.,
            hardness: 0.,
            ..settings()
        },
    )
    .unwrap();
    assert!(
        (10..=25).all(|n| single.values[n * 100 + n] == 255),
        "one-pixel soft tips must remain visible and connected"
    );
}

#[test]
fn sampling_respects_offsets_processed_layer_pixels_merged_layers_and_hidden_transparent_rgb() {
    let mut doc = Document::new(32, 24).unwrap();
    doc.add_layer();
    doc.active_mut().source = Raster::new(12, 24, [100, 0, 0, 128]).unwrap();
    doc.active_mut().offset = [10, 0];
    doc.active_mut().opacity = 0.5;
    doc.active_mut().locked = true; // Selection is read-only sampling.
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let before = doc.clone();
    let active = from_points(
        &doc,
        &[[15.5, 12.5]],
        Settings {
            diameter: 32.,
            tolerance: 0.,
            ..settings()
        },
    )
    .unwrap();
    assert_eq!(active.values[12 * 32 + 15], 255);
    assert_eq!(active.values[12 * 32 + 5], 0);
    let merged = from_points(
        &doc,
        &[[15.5, 12.5]],
        Settings {
            diameter: 32.,
            tolerance: 100.,
            sample_merged: true,
            ..settings()
        },
    )
    .unwrap();
    assert!(merged.values[12 * 32 + 5] > 0);
    assert!(doc == before);
    let mut layers = Document::new(32, 24).unwrap();
    for y in 0..24 {
        for x in 0..16 {
            layers.active_mut().source.set(x, y, [255, 0, 0, 255]);
        }
    }
    layers.add_layer();
    let opts = Settings {
        diameter: 32.,
        tolerance: 0.,
        ..settings()
    };
    let active = from_points(&layers, &[[10.5, 12.5]], opts).unwrap();
    let merged = from_points(
        &layers,
        &[[10.5, 12.5]],
        Settings {
            sample_merged: true,
            ..opts
        },
    )
    .unwrap();
    assert!(active.values[12 * 32 + 20] > 0);
    assert_eq!(
        merged.values[12 * 32 + 20],
        0,
        "all-visible sampling follows the underlying color edge"
    );
    let mut transparent = Document::from_raster(Raster::new(32, 24, [12, 200, 10, 0]).unwrap());
    transparent
        .active_mut()
        .source
        .set(16, 12, [255, 1, 155, 0]);
    let mask = from_points(
        &transparent,
        &[[15.5, 12.5]],
        Settings {
            tolerance: 0.,
            ..settings()
        },
    )
    .unwrap();
    assert_eq!(
        mask.values[12 * 32 + 16],
        255,
        "hidden RGB cannot create a false edge"
    );
}

#[test]
fn brush_regions_combine_feather_undo_and_save_without_changing_source_pixels() {
    let original = Document::new(64, 48).unwrap();
    let opts = Settings {
        edge_aware: false,
        strength: 0.5,
        ..settings()
    };
    let raw = from_points(&original, &[[20.5, 20.5], [35.5, 20.5]], opts).unwrap();
    for mode in [
        Combine::Replace,
        Combine::Add,
        Combine::Subtract,
        Combine::Intersect,
    ] {
        let mut doc = original.clone();
        doc.selection = Some(palette_core::editing::rectangle(
            64,
            48,
            palette_core::editing::PixelRect {
                x: 5,
                y: 5,
                width: 30,
                height: 30,
            },
        ));
        let before = doc.clone();
        let mut history = History::default();
        history.begin("Brush selection", &doc);
        selection::apply_region(&mut doc, &raw, mode, 2.5).unwrap();
        history.finish(&doc);
        assert!(doc.layers == original.layers);
        assert!(doc.selection_base.is_some());
        let committed = doc.clone();
        history.undo(&mut doc);
        assert!(doc == before);
        history.redo(&mut doc);
        assert!(doc == committed);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("brush.palette");
        io::save_project(&path, &doc).unwrap();
        assert!(io::open(&path).unwrap() == doc);
    }
}

#[test]
fn brush_rejects_invalid_parameters_and_retains_source_and_previous_coverage_on_error() {
    let doc = Document::new(20, 20).unwrap();
    for opts in [
        Settings {
            diameter: f32::NAN,
            ..settings()
        },
        Settings {
            strength: 1.1,
            ..settings()
        },
        Settings {
            tolerance: -1.,
            ..settings()
        },
        Settings {
            hardness: f32::INFINITY,
            ..settings()
        },
    ] {
        assert!(from_points(&doc, &[[10., 10.]], opts).is_err());
    }
    for point in [[-1., 10.], [20., 10.], [f32::NAN, 0.]] {
        assert!(Stroke::new(&doc, point, settings()).is_err());
    }
    let mut stroke = Stroke::new(&doc, [10., 10.], settings()).unwrap();
    let before = stroke.coverage().unwrap();
    assert!(stroke.extend([f32::INFINITY, 10.]).is_err());
    assert_eq!(stroke.coverage().unwrap(), before);
    assert!(from_points(&doc, &vec![[10., 10.]; 16385], settings()).is_err());
}

#[test]
fn bounded_feather_matches_full_canvas_at_fractional_and_large_radii_and_canvas_edges() {
    for (w, h, radii) in [
        (240, 180, vec![0., 0.1, 2.5, 4.9, 5., 12.]),
        (2048, 1280, vec![128.]),
    ] {
        for corner in [false, true] {
            let mut mask = palette_core::editing::rectangle(
                w,
                h,
                palette_core::editing::PixelRect {
                    x: if corner { 0 } else { w / 2 },
                    y: if corner { 0 } else { h / 2 },
                    width: 12,
                    height: 10,
                },
            );
            for (i, v) in std::sync::Arc::make_mut(&mut mask.values)
                .iter_mut()
                .enumerate()
            {
                if *v > 0 {
                    *v = 50 + (i % 206) as u8;
                }
            }
            for radius in &radii {
                assert_eq!(
                    selection::feather_region(&mask, *radius).unwrap(),
                    selection::feather(&mask, *radius).unwrap(),
                    "{w}x{h}, corner={corner}, r={radius}"
                );
            }
            mask.enabled = false;
            assert_eq!(
                selection::feather_region(&mask, 2.5).unwrap(),
                selection::feather(&mask, 2.5).unwrap()
            );
        }
    }
    let zero = palette_core::editing::rectangle(
        80,
        60,
        palette_core::editing::PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
    );
    assert_eq!(
        selection::feather_region(&zero, 3.).unwrap(),
        selection::feather(&zero, 3.).unwrap()
    );
    assert!(selection::feather_region(&zero, f32::NAN).is_err());
}
