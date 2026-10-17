use palette_core::{
    composite,
    document::{BlendMode as B, Document, Filter, FilterInstance},
    editing::{self, PixelRect},
    history::History,
    io,
    raster::{Coverage, Raster},
    selection::{self, Combine, Wand},
};
use std::sync::Arc;
fn mask(w: u32, h: u32, v: Vec<u8>) -> Coverage {
    Coverage {
        width: w,
        height: h,
        values: Arc::new(v),
        enabled: true,
    }
}

#[test]
fn separable_blends_match_independent_opaque_reference_values() {
    let back = [51, 102, 204, 255];
    let front = [204, 51, 102, 255];
    // Analytic normalized RGB inputs (0.2,0.4,0.8)/(0.8,0.2,0.4).
    for (mode, rgb) in [
        (B::Normal, [204, 51, 102]),
        (B::Multiply, [41, 20, 82]),
        (B::Screen, [214, 133, 224]),
        (B::Overlay, [82, 41, 194]),
        (B::Darken, [51, 51, 102]),
        (B::Lighten, [204, 102, 204]),
        (B::ColorDodge, [255, 128, 255]),
        (B::ColorBurn, [0, 0, 128]),
        (B::LinearDodge, [255, 153, 255]),
        (B::LinearBurn, [0, 0, 51]),
        (B::SoftLight, [89, 65, 196]),
        (B::HardLight, [173, 41, 163]),
        (B::Difference, [153, 51, 102]),
        (B::Exclusion, [173, 112, 143]),
        (B::Divide, [64, 255, 255]),
    ] {
        let actual = composite::over(back, front, 1., mode);
        assert!(
            actual[..3].iter().zip(rgb).all(|(a, b)| a.abs_diff(b) <= 1),
            "{mode:?}: {actual:?}"
        );
        assert_eq!(actual[3], 255);
    }
}
#[test]
fn all_modes_handle_zero_alpha_extremes_and_partial_coverage_without_hidden_rgb() {
    for mode in B::ALL {
        assert_eq!(
            composite::over([255, 0, 255, 0], [51, 102, 204, 128], 1., mode),
            [51, 102, 204, 128],
            "{mode:?}"
        );
        assert_eq!(
            composite::over([51, 102, 204, 128], [255, 0, 255, 0], 1., mode),
            [51, 102, 204, 128]
        );
        assert_eq!(
            composite::over([1, 2, 3, 0], [99, 88, 77, 0], 1., mode),
            [0; 4]
        );
        let back = [51, 102, 204, 128];
        let front = [204, 51, 102, 128];
        let mixed = composite::over([51, 102, 204, 255], [204, 51, 102, 255], 1., mode);
        let result = composite::over(back, front, 0.5, mode);
        let ab = 128f64 / 255.;
        let af = 64f64 / 255.;
        let alpha = af + ab * (1. - af);
        assert_eq!(result[3], (alpha * 255.).round() as u8);
        for c in 0..3 {
            let reference = ((1. - af) * ab * f64::from(back[c])
                + (1. - ab) * af * f64::from(front[c])
                + ab * af * f64::from(mixed[c]))
                / alpha;
            assert!((f64::from(result[c]) - reference).abs() <= 1., "{mode:?}");
        }
        // Includes dodge/burn/divide's otherwise singular endpoints.
        for a in [0, 128, 255] {
            for b in [0, 128, 255] {
                assert_eq!(
                    composite::over([a, a, a, 255], [b, b, b, 255], 1., mode)[3],
                    255
                );
            }
        }
    }
    assert_eq!(
        composite::over([0, 0, 0, 255], [255; 4], 1., B::ColorDodge),
        [0, 0, 0, 255]
    );
    assert_eq!(
        composite::over([255; 4], [0, 0, 0, 255], 1., B::ColorBurn),
        [255; 4]
    );
}
#[test]
fn nonseparable_blends_preserve_luminosity_and_use_gamut_clipping() {
    let lum = |p: [u8; 4]| 0.3 * f64::from(p[0]) + 0.59 * f64::from(p[1]) + 0.11 * f64::from(p[2]);
    for b in [[30, 90, 180, 255], [0, 255, 0, 255], [127, 127, 127, 255]] {
        for f in [[255, 0, 0, 255], [10, 60, 250, 255], [255; 4]] {
            for mode in [B::Hue, B::Saturation, B::Color] {
                assert!((lum(composite::over(b, f, 1., mode)) - lum(b)).abs() < 1.);
            }
            assert!((lum(composite::over(b, f, 1., B::Luminosity)) - lum(f)).abs() < 1.);
        }
    }
    assert_eq!(
        composite::over([0, 255, 0, 255], [255, 0, 0, 255], 1., B::Color),
        [255, 106, 106, 255]
    );
    assert_eq!(
        composite::over([0, 255, 0, 255], [255, 0, 0, 255], 1., B::Luminosity),
        [0, 130, 0, 255]
    );
}
#[test]
fn blend_modes_filters_masks_and_backing_survive_native_save_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("all-modes.palette");
    let mut doc = Document::new(12, 10).unwrap();
    let mut history = History::default();
    history.begin("Layer stack", &doc);
    for mode in B::ALL {
        doc.add_layer();
        doc.active_mut().blend = mode;
        doc.active_mut().opacity = 0.4;
        doc.active_mut().source.set(4, 3, [30, 90, 180, 128]);
    }
    doc.selection = Some(editing::rectangle(
        12,
        10,
        PixelRect {
            x: 3,
            y: 2,
            width: 3,
            height: 4,
        },
    ));
    selection::mask_from_selection(&mut doc).unwrap();
    assert!(Arc::ptr_eq(
        &doc.active().mask.as_ref().unwrap().values,
        &doc.selection.as_ref().unwrap().values
    ));
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    doc.background = [220, 210, 200, 255];
    history.finish(&doc);
    io::save_project(&path, &doc).unwrap();
    assert!(io::open(&path).unwrap() == doc);
    let rendered = composite::render(&doc);
    history.undo(&mut doc);
    assert_eq!(doc.layers.len(), 1);
    history.redo(&mut doc);
    assert!(composite::render(&doc) == rendered);
}
#[test]
fn coverage_set_operations_keep_fractional_edges_and_validate_sizes() {
    let a = mask(4, 1, vec![0, 64, 128, 255]);
    let b = mask(4, 1, vec![255, 128, 128, 0]);
    for (mode, expected) in [
        (Combine::Replace, vec![255, 128, 128, 0]),
        (Combine::Add, vec![255, 160, 192, 255]),
        (Combine::Subtract, vec![0, 32, 64, 255]),
        (Combine::Intersect, vec![0, 32, 64, 0]),
    ] {
        assert_eq!(
            *selection::combine(Some(&a), &b, mode).unwrap().values,
            expected
        );
    }
    assert_eq!(
        *selection::combine(None, &b, Combine::Add).unwrap().values,
        *b.values
    );
    assert_eq!(
        *selection::combine(None, &b, Combine::Subtract)
            .unwrap()
            .values,
        vec![0, 127, 127, 255]
    );
    assert!(selection::combine(Some(&a), &mask(1, 1, vec![255]), Combine::Add).is_err());
    assert_eq!(*a.values, vec![0, 64, 128, 255]);
    assert!(Arc::ptr_eq(
        &selection::combine(None, &b, Combine::Replace)
            .unwrap()
            .values,
        &b.values
    ));
    let visual = editing::contours(&mask(4, 1, vec![0, 127, 128, 255]));
    assert_eq!(visual.len(), 1);
    assert_eq!(
        visual[0].iter().map(|p| p[0]).fold(f32::INFINITY, f32::min),
        2.
    );
}
fn exact_wand() -> Wand {
    Wand {
        tolerance: 0.,
        antialias: false,
        ..Default::default()
    }
}
#[test]
fn wand_distinguishes_connected_islands_tolerance_and_transparent_hidden_colors() {
    let mut r = Raster::new(7, 3, [255; 4]).unwrap();
    for x in [0, 1, 5, 6] {
        for y in 0..3 {
            r.set(x, y, [200, 20, 50, 255]);
        }
    }
    r.set(2, 0, [201, 21, 51, 255]);
    let doc = Document::from_raster(r);
    let hard = selection::wand(&doc, [0, 1], exact_wand()).unwrap();
    assert_eq!(hard.values.iter().filter(|v| **v == 255).count(), 6);
    assert_eq!(hard.at(5, 1), 0.);
    let all = selection::wand(
        &doc,
        [0, 1],
        Wand {
            contiguous: false,
            ..exact_wand()
        },
    )
    .unwrap();
    assert_eq!(all.values.iter().filter(|v| **v == 255).count(), 12);
    let more = selection::wand(
        &doc,
        [0, 1],
        Wand {
            tolerance: 2.,
            ..exact_wand()
        },
    )
    .unwrap();
    assert_eq!(more.at(2, 0), 1.);
    assert!(
        more.values
            .iter()
            .zip(hard.values.iter())
            .all(|(a, b)| a >= b)
    );
    let mut doc = Document::from_raster(Raster::new(3, 1, [0; 4]).unwrap());
    doc.active_mut().source.set(1, 0, [255, 0, 255, 0]);
    doc.active_mut().source.set(2, 0, [0, 0, 0, 1]);
    assert_eq!(
        *selection::wand(&doc, [0, 0], exact_wand()).unwrap().values,
        vec![255, 255, 0]
    );
}
#[test]
fn wand_merged_sampling_offsets_antialias_and_locks_do_not_mutate_sources() {
    let mut doc = Document::new(8, 5).unwrap();
    doc.add_layer();
    doc.active_mut().source = Raster::new(2, 2, [255, 0, 0, 255]).unwrap();
    doc.active_mut().offset = [3, 1];
    doc.active_mut().locked = true;
    let before = doc.clone();
    let mask = selection::wand(&doc, [3, 1], exact_wand()).unwrap();
    assert_eq!(mask.values.iter().filter(|v| **v == 255).count(), 4);
    let aa = selection::wand(
        &doc,
        [3, 1],
        Wand {
            antialias: true,
            ..exact_wand()
        },
    )
    .unwrap();
    assert!(aa.values.iter().any(|v| *v > 0 && *v < 255));
    doc.active_mut().visible = false;
    let merged = selection::wand(
        &doc,
        [3, 1],
        Wand {
            sample_merged: true,
            ..exact_wand()
        },
    )
    .unwrap();
    assert!(merged.values.iter().all(|v| *v == 255));
    doc.active_mut().visible = true;
    assert!(doc == before);
    assert!(selection::wand(&doc, [8, 0], exact_wand()).is_err());
    assert!(
        selection::wand(
            &doc,
            [0, 0],
            Wand {
                tolerance: f32::NAN,
                ..exact_wand()
            }
        )
        .is_err()
    );
}
#[test]
fn scanline_wand_matches_an_independent_four_connected_breadth_first_search() {
    use std::collections::VecDeque;
    for seed in 0..20u32 {
        let mut r = Raster::new(17, 11, [0, 0, 0, 255]).unwrap();
        for y in 0..11 {
            for x in 0..17 {
                if (x * 13 + y * 7 + seed * 11) % 5 < 2 {
                    r.set(x, y, [255; 4]);
                }
            }
        }
        let target = r.get(seed % 17, seed % 11);
        let mut queue = VecDeque::from([(seed % 17, seed % 11)]);
        let mut visited = vec![0; 187];
        while let Some((x, y)) = queue.pop_front() {
            if visited[(y * 17 + x) as usize] != 0 || r.get(x, y) != target {
                continue;
            }
            visited[(y * 17 + x) as usize] = 255;
            for (nx, ny) in [
                (i64::from(x) - 1, i64::from(y)),
                (i64::from(x) + 1, i64::from(y)),
                (i64::from(x), i64::from(y) - 1),
                (i64::from(x), i64::from(y) + 1),
            ] {
                if (0..17).contains(&nx) && (0..11).contains(&ny) {
                    queue.push_back((nx as u32, ny as u32));
                }
            }
        }
        assert_eq!(
            *selection::wand(
                &Document::from_raster(r),
                [seed % 17, seed % 11],
                exact_wand()
            )
            .unwrap()
            .values,
            visited
        );
    }
}
#[test]
fn feathering_is_symmetric_bounded_preserves_constant_masks_and_never_changes_input() {
    let base = editing::rectangle(
        41,
        1,
        PixelRect {
            x: 15,
            y: 0,
            width: 11,
            height: 1,
        },
    );
    let soft = selection::feather(&base, 3.).unwrap();
    assert_eq!(
        soft.values.iter().rev().copied().collect::<Vec<_>>(),
        *soft.values
    );
    assert!(soft.values[14] > 0 && soft.values[14] < 128);
    assert!(soft.values[15] > 128 && soft.values[15] < 255);
    assert_eq!(base.values[14], 0);
    assert_eq!(selection::feather(&base, 0.).unwrap(), base);
    for value in [0, 128, 255] {
        for (w, h) in [(1, 1), (1, 7), (9, 1), (4, 5)] {
            let m = mask(w, h, vec![value; (w * h) as usize]);
            assert_eq!(selection::feather(&m, 128.).unwrap(), m);
        }
    }
    assert!(selection::feather(&base, f32::INFINITY).is_err());
    assert!(selection::feather(&base, -1.).is_err());
    let mut disabled = base.clone();
    disabled.enabled = false;
    let softened = selection::feather(&disabled, 3.).unwrap();
    assert!(!softened.enabled);
    assert_eq!(softened.values, soft.values);
    assert_eq!(selection::feather(&disabled, 0.).unwrap(), disabled);
}
#[test]
fn layer_masks_reanchor_selection_and_preserve_source_filter_and_opacity() {
    let mut doc = Document::new(9, 7).unwrap();
    doc.active_mut().offset = [-2, 1];
    doc.active_mut().opacity = 0.7;
    doc.selection = Some(editing::rectangle(
        9,
        7,
        PixelRect {
            x: 1,
            y: 2,
            width: 3,
            height: 2,
        },
    ));
    let source = doc.active().source.clone();
    selection::mask_from_selection(&mut doc).unwrap();
    let m = doc.active().mask.as_ref().unwrap();
    assert_eq!(m.at(3, 1), 1.);
    assert_eq!(m.at(2, 1), 0.);
    assert_eq!(m.values.iter().filter(|v| **v == 255).count(), 6);
    assert!(doc.active().source == source);
    assert_eq!(doc.active().opacity, 0.7);
    assert!(!selection::mask_from_selection(&mut doc).unwrap());
    doc.active_mut().locked = true;
    let before = doc.clone();
    assert!(selection::mask_from_selection(&mut doc).is_err());
    assert!(doc == before);
}
