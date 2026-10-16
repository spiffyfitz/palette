use palette_core::{
    document::{Document, Filter, FilterInstance},
    editing::{self, PixelRect},
    io,
    raster::{Coverage, Raster},
    shapes::{self, Kind, Style},
    transform::{self, Orthogonal as O},
};
use std::sync::Arc;
fn pattern() -> Document {
    let mut doc = Document::new(14, 12).unwrap();
    doc.add_layer();
    for (i, value) in [10, 20, 30, 40, 50, 60].into_iter().enumerate() {
        doc.active_mut().source.set(
            4 + (i % 2) as u32,
            3 + (i / 2) as u32,
            [value, 0, 0, if value == 20 { 128 } else { 255 }],
        );
    }
    doc.active_mut().source.set(1, 1, [200, 20, 40, 255]);
    doc.selection = Some(editing::rectangle(
        14,
        12,
        PixelRect {
            x: 4,
            y: 3,
            width: 2,
            height: 3,
        },
    ));
    doc
}

#[test]
fn selection_resize_preserves_processing_outside_pixels_and_transparent_colors() {
    use transform::SelectionTransform as T;
    let mut doc = pattern();
    doc.active_mut().opacity = 0.6;
    doc.active_mut().mask = Some(Coverage {
        width: 14,
        height: 12,
        enabled: true,
        values: Arc::new(vec![200; 168]),
    });
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let before = doc.clone();
    T::Resize {
        origin: [7, 4],
        size: [4, 6],
    }
    .apply(&mut doc)
    .unwrap();
    assert_eq!([doc.width, doc.height], [14, 12]);
    assert_eq!(doc.layers[0], before.layers[0]);
    assert_eq!(
        doc.active().source.get(1, 1),
        before.active().source.get(1, 1)
    );
    assert_eq!(doc.active().source.get(4, 3)[3], 0);
    assert_eq!(doc.active().source.get(7, 4), [10, 0, 0, 255]);
    assert_eq!(doc.active().source.get(10, 4), [20, 0, 0, 128]);
    assert_eq!(doc.active().mask, before.active().mask);
    assert_eq!(doc.active().filters, before.active().filters);
    assert_eq!(doc.active().opacity, before.active().opacity);
    assert_eq!(
        editing::bounds(doc.selection.as_ref().unwrap()),
        Some(PixelRect {
            x: 7,
            y: 4,
            width: 4,
            height: 6
        })
    );

    let mut doc = Document::new(12, 4).unwrap();
    doc.add_layer();
    doc.active_mut().source.set(1, 1, [255, 0, 0, 128]);
    doc.active_mut().source.set(2, 1, [0, 255, 0, 0]);
    doc.selection = Some(editing::rectangle(
        12,
        4,
        PixelRect {
            x: 1,
            y: 1,
            width: 2,
            height: 1,
        },
    ));
    T::Resize {
        origin: [4, 1],
        size: [6, 1],
    }
    .apply(&mut doc)
    .unwrap();
    for x in 4..10 {
        let p = doc.active().source.get(x, 1);
        if p[3] > 0 {
            assert_eq!(&p[..3], &[255, 0, 0]);
        }
    }
    assert_eq!(
        doc.selection.as_ref().unwrap().at(9, 1),
        1.,
        "transparent pixels remain selected"
    );
}

#[test]
fn selection_gesture_validation_lock_noops_and_canvas_clipping_are_atomic() {
    use transform::SelectionTransform as T;
    let original = pattern();
    for op in [
        T::Resize {
            origin: [0, 0],
            size: [0, 4],
        },
        T::Resize {
            origin: [0, 0],
            size: [u32::MAX, u32::MAX],
        },
        T::Rotate(f64::INFINITY),
    ] {
        let mut doc = original.clone();
        assert!(op.apply(&mut doc).is_err());
        assert!(doc == original);
    }
    let mut doc = original.clone();
    assert!(
        !T::Resize {
            origin: [4, 3],
            size: [2, 3]
        }
        .apply(&mut doc)
        .unwrap()
    );
    assert!(doc == original);
    doc.active_mut().locked = true;
    let locked = doc.clone();
    assert!(
        !T::Resize {
            origin: [1, 1],
            size: [6, 9]
        }
        .apply(&mut doc)
        .unwrap()
    );
    assert!(doc == locked);
    let mut doc = original;
    T::Move([-5, -4]).apply(&mut doc).unwrap();
    assert_eq!(
        editing::bounds(doc.selection.as_ref().unwrap()),
        Some(PixelRect {
            x: 0,
            y: 0,
            width: 1,
            height: 2
        })
    );
    assert_eq!(doc.active().source.get(0, 0), [40, 0, 0, 255]);
    assert_eq!([doc.width, doc.height], [14, 12]);
}

#[test]
fn overlapping_fractional_moves_do_not_double_attenuate_selected_alpha() {
    let mut doc = Document::from_raster(Raster::new(6, 4, [100, 50, 25, 128]).unwrap());
    doc.selection = Some(Coverage {
        width: 6,
        height: 4,
        enabled: true,
        values: Arc::new(vec![64; 24]),
    });
    editing::move_selection(&mut doc, [1, 0]).unwrap();
    assert_eq!(doc.active().source.get(2, 2), [100, 50, 25, 128]);
    assert_eq!(doc.active().source.get(0, 2)[3], 96);
}
#[test]
fn flips_and_quarter_turns_affect_only_selected_pixels_and_coverage() {
    for (op, rows) in [
        (
            O::FlipHorizontal,
            vec![vec![20, 10], vec![40, 30], vec![60, 50]],
        ),
        (
            O::FlipVertical,
            vec![vec![50, 60], vec![30, 40], vec![10, 20]],
        ),
        (O::Clockwise, vec![vec![50, 30, 10], vec![60, 40, 20]]),
        (
            O::CounterClockwise,
            vec![vec![20, 40, 60], vec![10, 30, 50]],
        ),
        (O::HalfTurn, vec![vec![60, 50], vec![40, 30], vec![20, 10]]),
    ] {
        let mut doc = pattern();
        let original = doc.clone();
        assert!(transform::selected_orthogonal(&mut doc, op).unwrap());
        assert_eq!([doc.width, doc.height], [14, 12]);
        assert_eq!(doc.layers[0], original.layers[0]);
        assert_eq!(doc.active().source.get(1, 1), [200, 20, 40, 255]);
        for (y, row) in rows.iter().enumerate() {
            for (x, value) in row.iter().enumerate() {
                let p = doc.active().source.get(4 + x as u32, 3 + y as u32);
                assert_eq!(p[0], *value);
                assert_eq!(p[3], if *value == 20 { 128 } else { 255 });
            }
        }
        let bounds = editing::bounds(doc.selection.as_ref().unwrap()).unwrap();
        assert_eq!(
            [bounds.x, bounds.y, bounds.width, bounds.height],
            [4, 3, rows[0].len() as u32, rows.len() as u32]
        );
    }
    let mut doc = pattern();
    let original = doc.clone();
    for _ in 0..4 {
        transform::selected_orthogonal(&mut doc, O::Clockwise).unwrap();
    }
    assert_eq!(doc.selection, original.selection);
    for y in 0..12 {
        for x in 0..14 {
            let a = doc.active().source.get(x, y);
            let b = original.active().source.get(x, y);
            if b[3] == 0 {
                assert_eq!(a[3], 0, "{x},{y}");
            } else {
                assert_eq!(a, b, "{x},{y}");
            }
        }
    }
}
#[test]
fn fractional_selection_flip_retains_alpha_and_translated_layers_stay_local() {
    let mut doc = Document::from_raster(Raster::new(8, 6, [100, 50, 30, 128]).unwrap());
    doc.selection = Some(Coverage {
        width: 8,
        height: 6,
        enabled: true,
        values: Arc::new(vec![128; 48]),
    });
    transform::selected_orthogonal(&mut doc, O::FlipHorizontal).unwrap();
    assert_eq!(doc.active().source.get(3, 3), [100, 50, 30, 128]);
    let mut doc = pattern();
    doc.active_mut().offset = [2, 1];
    let old = doc.clone();
    transform::selected_orthogonal(&mut doc, O::FlipVertical).unwrap();
    assert_eq!(doc.layers[0], old.layers[0]);
    assert_eq!([doc.width, doc.height], [14, 12]);
    // Source outside the selected canvas rectangle is unchanged despite expansion.
    assert_eq!(doc.active().source.get(3, 2), [200, 20, 40, 255]);
}
#[test]
fn custom_rotation_is_scoped_alpha_safe_and_atomic() {
    let mut doc = pattern();
    let original = doc.clone();
    transform::rotate(&mut doc, 31.).unwrap();
    assert_eq!([doc.width, doc.height], [14, 12]);
    assert_eq!(doc.layers[0], original.layers[0]);
    assert_eq!(
        doc.active().source.get(1, 1),
        original.active().source.get(1, 1)
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rotated.palette");
    io::save_project(&path, &doc).unwrap();
    assert_eq!(io::open(&path).unwrap(), doc);
    let current = doc.clone();
    assert!(transform::rotate(&mut doc, f64::NAN).is_err());
    assert_eq!(doc, current);
    doc.active_mut().locked = true;
    let current = doc.clone();
    assert!(!transform::rotate(&mut doc, 31.).unwrap());
    assert_eq!(doc, current);
    let mut exact = pattern();
    let mut custom = exact.clone();
    transform::selected_orthogonal(&mut exact, O::Clockwise).unwrap();
    transform::rotate(&mut custom, 90.).unwrap();
    assert_eq!(exact, custom);
}
#[test]
fn custom_whole_image_rotation_retains_layers_masks_and_live_filters() {
    let mut doc = pattern();
    doc.selection = None;
    doc.active_mut().mask = Some(Coverage {
        width: 14,
        height: 12,
        enabled: true,
        values: Arc::new(vec![200; 168]),
    });
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let ids: Vec<_> = doc.layers.iter().map(|l| l.id).collect();
    transform::rotate(&mut doc, 45.).unwrap();
    assert_eq!(doc.layers.iter().map(|l| l.id).collect::<Vec<_>>(), ids);
    assert_eq!([doc.width, doc.height], [19, 19]);
    assert!(doc.active().mask.is_some());
    assert_eq!(doc.active().filters[0].id, id);
    doc.validate().unwrap();
    let mut alpha =
        Document::from_raster(Raster::from_rgba(2, 1, &[255, 0, 0, 128, 0, 255, 0, 0]).unwrap());
    transform::rotate(&mut alpha, 33.).unwrap();
    for p in alpha.active().source.rgba().as_chunks::<4>().0 {
        if p[3] > 0 {
            assert_eq!(&p[..3], &[255, 0, 0]);
        }
    }
}
#[test]
fn committed_shapes_and_paths_have_undo_ready_selections() {
    let mut doc = Document::new(100, 80).unwrap();
    doc.add_layer();
    let style = Style {
        outline: Some([10, 20, 30, 255]),
        fill: None,
        width: 4.,
        opacity: 1.,
    };
    let path = shapes::path(Kind::Triangle, [20., 20.], [60., 60.]);
    assert!(shapes::paint_selected(&mut doc, &path, style).unwrap());
    let rect = editing::bounds(doc.selection.as_ref().unwrap()).unwrap();
    assert!(rect.contains(20, 20) && rect.contains(60, 60));
    let p = shapes::cubic([10., 10.], [80., 60.], [[20., 70.], [60., 0.]]);
    assert_eq!(p.points[0], [10., 10.]);
    assert_eq!(*p.points.last().unwrap(), [80., 60.]);
    assert!(!p.closed);
}
