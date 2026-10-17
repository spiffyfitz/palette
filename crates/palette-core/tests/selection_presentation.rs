use palette_core::{
    document::Document,
    editing::{self, PixelRect},
    history::History,
    io,
    selection::{self, Combine, Wand},
    transform::SelectionTransform,
};

#[test]
fn feather_preserves_original_and_radius_changes_inversion_history_and_native_save() {
    let mut doc = Document::new(64, 48).unwrap();
    let raw = editing::rectangle(
        64,
        48,
        PixelRect {
            x: 20,
            y: 15,
            width: 16,
            height: 12,
        },
    );
    let source = doc.active().source.clone();
    selection::apply_region(&mut doc, &raw, Combine::Replace, 4.).unwrap();
    assert_eq!(doc.selection_base.as_ref(), Some(&raw));
    let mut history = History::default();
    history.begin("Adjust feather", &doc);
    selection::feather_selection(&mut doc, 2.).unwrap();
    assert_eq!(
        doc.selection.as_ref(),
        Some(&selection::feather(&raw, 2.).unwrap())
    );
    history.finish(&doc);
    let soft = doc.clone();
    history.begin("Invert", &doc);
    selection::invert(&mut doc);
    history.finish(&doc);
    assert_eq!(
        editing::contours(doc.selection_base.as_ref().unwrap()).len(),
        2
    );
    history.undo(&mut doc);
    assert!(doc == soft);
    history.redo(&mut doc);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("feather.palette");
    io::save_project(&path, &doc).unwrap();
    assert!(io::open(&path).unwrap() == doc);
    selection::invert(&mut doc);
    selection::feather_selection(&mut doc, 0.).unwrap();
    assert_eq!(doc.selection.as_ref(), Some(&raw));
    assert!(doc.selection_base.is_none());
    assert_eq!(doc.active().source, source);
}

#[test]
fn feather_extent_is_separate_from_original_and_both_follow_transforms() {
    let mut doc = Document::new(64, 48).unwrap();
    let raw = editing::rectangle(
        64,
        48,
        PixelRect {
            x: 20,
            y: 15,
            width: 16,
            height: 12,
        },
    );
    selection::apply_region(&mut doc, &raw, Combine::Replace, 3.).unwrap();
    let base = editing::contours(doc.selection_base.as_ref().unwrap());
    let extent = editing::contours_at(doc.selection.as_ref().unwrap(), 1);
    let left = |contours: &Vec<Vec<[f32; 2]>>| {
        contours
            .iter()
            .flatten()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
    };
    assert_eq!(left(&base), 20.);
    assert!(left(&extent) < 20.);
    SelectionTransform::Move([3, -2]).apply(&mut doc).unwrap();
    assert_eq!(
        editing::bounds(doc.selection_base.as_ref().unwrap()),
        Some(PixelRect {
            x: 23,
            y: 13,
            width: 16,
            height: 12
        })
    );
    palette_core::transform::canvas_resize(&mut doc, 80, 60).unwrap();
    palette_core::transform::orthogonal(&mut doc, palette_core::transform::Orthogonal::Clockwise)
        .unwrap();
    doc.validate().unwrap();
    assert_eq!(
        [
            doc.selection_base.as_ref().unwrap().width,
            doc.selection_base.as_ref().unwrap().height
        ],
        [60, 80]
    );
    let region = editing::rectangle(
        60,
        80,
        PixelRect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        },
    );
    selection::apply_region(&mut doc, &region, Combine::Replace, 0.).unwrap();
    assert!(doc.selection_base.is_none());
    let bad = editing::rectangle(
        4,
        4,
        PixelRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
    );
    let before = doc.clone();
    assert!(selection::apply_region(&mut doc, &bad, Combine::Replace, 2.).is_err());
    assert!(doc == before);
}

#[test]
fn wand_tolerance_increases_color_range_in_both_sampling_modes() {
    let mut doc = Document::new(256, 2).unwrap();
    for x in 0..256 {
        for y in 0..2 {
            doc.active_mut()
                .source
                .set(x, y, [x as u8, x as u8, x as u8, 255]);
        }
    }
    for contiguous in [true, false] {
        for antialias in [true, false] {
            let mut previous = vec![0; 512];
            for tolerance in [0., 1., 5., 10., 25., 50., 75., 100.] {
                let mask = selection::wand(
                    &doc,
                    [0, 0],
                    Wand {
                        tolerance,
                        contiguous,
                        antialias,
                        sample_merged: false,
                    },
                )
                .unwrap();
                assert!(
                    previous.iter().zip(mask.values.iter()).all(|(a, b)| a <= b),
                    "{tolerance}"
                );
                if tolerance == 100. {
                    assert!(mask.values.iter().all(|v| *v == 255));
                }
                previous = mask.values.as_ref().clone();
            }
        }
    }
}

#[test]
fn version_three_migrates_without_original_contour_and_invalid_base_cannot_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.palette");
    let doc = Document::new(12, 8).unwrap();
    io::save_project(&path, &doc).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    // With no retained original, v3 has the same descriptor/asset layout.
    bytes[8..12].copy_from_slice(&3u32.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let mut migrated = io::open(&path).unwrap();
    assert!(migrated == doc);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    migrated.selection_base = Some(editing::rectangle(
        12,
        8,
        PixelRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
    ));
    assert!(io::save_project(&path, &migrated).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    migrated.selection = Some(editing::rectangle(
        12,
        8,
        PixelRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
    ));
    migrated.selection_base = Some(editing::rectangle(
        4,
        4,
        PixelRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        },
    ));
    assert!(io::save_project(&path, &migrated).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn fractional_feather_radii_match_independent_integrated_gaussian_references() {
    use palette_core::raster::Coverage;
    use std::sync::Arc;
    let mut bytes = vec![0; 41];
    bytes[20] = 255;
    let mask = Coverage {
        width: 41,
        height: 1,
        values: Arc::new(bytes),
        enabled: true,
    };
    // Independent Python math.erf evaluation of the integrated Gaussian weights;
    // each row lists the center and four neighboring samples at positive offsets.
    for (radius, expected) in [
        (0.1, [235, 10, 0, 0, 0]),
        (0.2, [214, 20, 0, 0, 0]),
        (0.3, [194, 31, 0, 0, 0]),
        (0.4, [174, 40, 0, 0, 0]),
        (0.7, [125, 59, 6, 0, 0]),
        (1.3, [75, 57, 25, 7, 1]),
        (2.7, [37, 35, 28, 20, 13]),
        (4.9, [21, 20, 19, 17, 15]),
    ] {
        let soft = selection::feather(&mask, radius).unwrap();
        for (offset, reference) in expected.into_iter().enumerate() {
            assert!(
                soft.values[20 + offset].abs_diff(reference) <= 1,
                "radius={radius}, offset={offset}"
            );
            assert_eq!(soft.values[20 - offset], soft.values[20 + offset]);
        }
        for (w, h) in [(1, 1), (1, 9), (9, 1), (7, 5)] {
            let constant = Coverage {
                width: w,
                height: h,
                values: Arc::new(vec![73; (w * h) as usize]),
                enabled: false,
            };
            assert_eq!(selection::feather(&constant, radius).unwrap(), constant);
        }
    }
    assert_eq!(mask.values[20], 255);
    assert_eq!(mask.values.iter().filter(|v| **v > 0).count(), 1);
}
