use palette_core::{
    brush::{Brush, PaintStroke},
    composite,
    document::{Document, Filter, FilterInstance},
    editing::{self, PixelRect},
    history::History,
    raster::{Coverage, Raster},
    transform::{self, Orthogonal, Resample},
};
use std::sync::Arc;

fn rect(x: u32, y: u32, width: u32, height: u32) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn brush_stroke_opacity_does_not_depend_on_event_frequency_or_retracing() {
    let source = Raster::new(128, 24, [0; 4]).unwrap();
    let brush = Brush {
        diameter: 8.0,
        opacity: 0.5,
        hardness: 1.0,
        color: [255, 0, 0, 255],
        erase: false,
    };
    let mut sparse = source.clone();
    let mut stroke = PaintStroke::new(&source);
    stroke.segment(brush, &mut sparse, [4.5, 12.5], [120.5, 12.5], None);
    let mut dense = source.clone();
    let mut stroke = PaintStroke::new(&source);
    for x in 5..=120 {
        stroke.segment(
            brush,
            &mut dense,
            [x as f32 - 0.5, 12.5],
            [x as f32 + 0.5, 12.5],
            None,
        );
    }
    stroke.segment(brush, &mut dense, [120.5, 12.5], [4.5, 12.5], None);
    assert_eq!(dense.rgba(), sparse.rgba());
    assert_eq!(dense.get(60, 12), [255, 0, 0, 128]);
    let mut next = PaintStroke::new(&dense);
    next.segment(brush, &mut dense, [20.5, 12.5], [80.5, 12.5], None);
    assert_eq!(dense.get(60, 12)[3], 192);
}

#[test]
fn coverage_clips_brush_and_eraser_in_translated_layer_space() {
    let mut raster = Raster::new(4, 2, [30, 60, 90, 255]).unwrap();
    let mask = Coverage {
        width: 6,
        height: 2,
        values: Arc::new(vec![0, 0, 128, 255, 0, 0, 0, 0, 128, 255, 0, 0]),
        enabled: true,
    };
    let brush = Brush {
        diameter: 8.0,
        opacity: 1.0,
        hardness: 1.0,
        color: [255, 0, 0, 255],
        erase: true,
    };
    brush.segment_selected(&mut raster, [0.0, 1.0], [4.0, 1.0], Some((&mask, [1, 0])));
    assert_eq!(raster.get(0, 0), [30, 60, 90, 255]);
    assert_eq!(raster.get(1, 0), [30, 60, 90, 127]);
    assert_eq!(raster.get(2, 0)[3], 0);
    assert_eq!(raster.get(3, 0), [30, 60, 90, 255]);
}

#[test]
fn flood_respects_contiguity_tolerance_transparency_selection_and_lock() {
    let mut doc = Document::from_raster(Raster::new(7, 3, [0, 0, 0, 0]).unwrap());
    for y in 0..3 {
        doc.active_mut().source.set(3, y, [0, 0, 0, 255]);
    }
    // Hidden RGB differs but belongs to the same transparent region.
    doc.active_mut().source.set(1, 1, [255, 90, 180, 0]);
    assert!(editing::flood_fill(&mut doc, [0, 0], [250, 10, 20, 255], 1.0, 0).unwrap());
    assert_eq!(doc.active().source.get(2, 2), [250, 10, 20, 255]);
    assert_eq!(doc.active().source.get(4, 1), [0; 4]);
    doc.selection = Some(editing::rectangle(7, 3, rect(4, 0, 2, 2)));
    assert!(editing::flood_fill(&mut doc, [4, 0], [0, 0, 255, 255], 1.0, 0).unwrap());
    assert_eq!(doc.active().source.get(5, 1), [0, 0, 255, 255]);
    assert_eq!(doc.active().source.get(6, 1), [0; 4]);
    assert_eq!(doc.active().source.get(4, 2), [0; 4]);
    doc.active_mut().locked = true;
    let before = doc.clone();
    assert!(!editing::flood_fill(&mut doc, [4, 0], [255; 4], 1.0, 255).unwrap());
    assert_eq!(doc, before);
    doc.active_mut().locked = false;
    doc.selection = None;
    doc.active_mut().source.set(5, 0, [4, 4, 250, 255]);
    assert!(editing::flood_fill(&mut doc, [4, 0], [255; 4], 1.0, 5).unwrap());
    assert_eq!(doc.active().source.get(5, 0), [255; 4]);
}

#[test]
fn copy_paste_and_cut_preserve_other_layers_and_live_source_properties() {
    let mut doc = Document::new(4, 3).unwrap();
    doc.add_layer();
    doc.active_mut().source.set(1, 1, [30, 60, 90, 200]);
    doc.selection = Some(editing::rectangle(4, 3, rect(1, 1, 1, 1)));
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let copied = editing::copy(&doc).unwrap();
    assert_eq!(copied.get(0, 0), [225, 195, 165, 200]);
    assert_eq!(doc.active().source.get(1, 1), [30, 60, 90, 200]);
    let mut history = History::default();
    history.begin("Cut", &doc);
    assert!(editing::clear(&mut doc));
    history.finish(&doc);
    assert_eq!(doc.layers[0].source.get(1, 1), [255; 4]);
    assert_eq!(doc.active().source.get(1, 1)[3], 0);
    history.undo(&mut doc);
    assert_eq!(doc.active().source.get(1, 1), [30, 60, 90, 200]);
    editing::paste(&mut doc, copied).unwrap();
    assert_eq!(doc.layers.len(), 3);
    assert_eq!(doc.active().offset, [1, 1]);
    assert_eq!(doc.active().source.get(0, 0), [225, 195, 165, 200]);
    assert_eq!(doc.layers[1].filters.len(), 1);
    doc.validate().unwrap();
}

#[test]
fn moving_selection_expands_translated_layer_without_losing_hidden_pixels() {
    let mut doc = Document::from_raster(Raster::new(2, 2, [0; 4]).unwrap());
    doc.width = 6;
    doc.height = 4;
    doc.active_mut().offset = [-1, 0];
    doc.active_mut().source.set(0, 0, [20, 40, 60, 255]);
    doc.active_mut().source.set(1, 0, [255, 0, 0, 180]);
    doc.selection = Some(editing::rectangle(6, 4, rect(0, 0, 1, 1)));
    assert!(editing::move_selection(&mut doc, [3, 2]).unwrap());
    assert_eq!(composite::render(&doc).get(3, 2), [255, 0, 0, 180]);
    assert_eq!(composite::render(&doc).get(0, 0), [0; 4]);
    assert_eq!(doc.active().offset, [-1, 0]);
    assert_eq!(doc.active().source.get(0, 0), [20, 40, 60, 255]);
    assert_eq!(
        editing::bounds(doc.selection.as_ref().unwrap()),
        Some(rect(3, 2, 1, 1))
    );
    doc.validate().unwrap();
}

#[test]
fn crop_retains_hidden_pixels_and_undo_restores_geometry() {
    let mut doc = Document::new(6, 4).unwrap();
    doc.add_layer();
    doc.active_mut().source.set(0, 0, [255, 0, 0, 255]);
    doc.selection = Some(editing::rectangle(6, 4, rect(2, 1, 3, 2)));
    let before = doc.clone();
    let mut history = History::default();
    history.begin("Crop", &doc);
    assert!(transform::crop(&mut doc).unwrap());
    history.finish(&doc);
    assert_eq!((doc.width, doc.height), (3, 2));
    assert_eq!(doc.active().offset, [-2, -1]);
    assert_eq!(doc.active().source.get(0, 0), [255, 0, 0, 255]);
    history.undo(&mut doc);
    assert_eq!(doc, before);
}

#[test]
fn orthogonal_operations_have_exact_expected_pixel_order() {
    for (operation, expected, size) in [
        (Orthogonal::Clockwise, vec![4, 1, 5, 2, 6, 3], [2, 3]),
        (Orthogonal::CounterClockwise, vec![3, 6, 2, 5, 1, 4], [2, 3]),
        (Orthogonal::HalfTurn, vec![6, 5, 4, 3, 2, 1], [3, 2]),
        (Orthogonal::FlipHorizontal, vec![3, 2, 1, 6, 5, 4], [3, 2]),
        (Orthogonal::FlipVertical, vec![4, 5, 6, 1, 2, 3], [3, 2]),
    ] {
        let mut pixels = Raster::new(3, 2, [0; 4]).unwrap();
        for i in 0..6 {
            pixels.set(i % 3, i / 3, [i as u8 + 1, 0, 0, 255]);
        }
        let mut doc = Document::from_raster(pixels);
        transform::orthogonal(&mut doc, operation).unwrap();
        assert_eq!([doc.width, doc.height], size);
        assert_eq!(
            doc.active()
                .source
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[0])
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn rotation_tracks_layer_positions_and_mask_and_filter_coverage() {
    let mut doc = Document::new(4, 3).unwrap();
    doc.add_layer();
    doc.active_mut().source = Raster::new(2, 1, [100, 20, 30, 255]).unwrap();
    doc.active_mut().offset = [1, 1];
    doc.active_mut().mask = Some(Coverage {
        width: 2,
        height: 1,
        values: Arc::new(vec![255, 128]),
        enabled: true,
    });
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: Some(Coverage {
            width: 2,
            height: 1,
            values: Arc::new(vec![255, 0]),
            enabled: true,
        }),
    });
    let before = composite::render(&doc);
    transform::orthogonal(&mut doc, Orthogonal::Clockwise).unwrap();
    let after = composite::render(&doc);
    for y in 0..3 {
        for x in 0..4 {
            assert_eq!(before.get(x, y), after.get(2 - y, x));
        }
    }
    assert_eq!(doc.active().offset, [1, 1]);
    assert_eq!(
        doc.active().mask.as_ref().unwrap().values.as_ref(),
        &vec![255, 128]
    );
    assert_eq!(doc.active().filters[0].id, id);
    doc.validate().unwrap();
}

#[test]
fn bilinear_resampling_does_not_bleed_transparent_rgb() {
    let input = Raster::from_rgba(2, 1, &[255, 0, 255, 0, 0, 255, 0, 255]).unwrap();
    let resized = transform::resample(&input, 3, 1, Resample::Bilinear).unwrap();
    assert_eq!(resized.get(1, 0), [0, 255, 0, 128]);
    assert_eq!(resized.get(2, 0), [0, 255, 0, 255]);
    let mut doc = Document::from_raster(input);
    doc.selection = Some(editing::rectangle(2, 1, rect(1, 0, 1, 1)));
    transform::resize(&mut doc, 4, 2, Resample::Nearest).unwrap();
    assert_eq!(doc.active().source.get(2, 0), [0, 255, 0, 255]);
    assert_eq!(
        editing::bounds(doc.selection.as_ref().unwrap()),
        Some(rect(2, 0, 2, 2))
    );
    doc.validate().unwrap();
}

#[test]
fn canvas_resize_updates_selection_but_preserves_source_and_rejects_bad_sizes() {
    let mut doc = Document::new(4, 3).unwrap();
    doc.selection = Some(editing::rectangle(4, 3, rect(1, 1, 2, 2)));
    transform::canvas_resize(&mut doc, 2, 2).unwrap();
    assert_eq!(doc.active().source.width(), 4);
    assert_eq!(
        doc.selection.as_ref().unwrap().values.as_ref(),
        &vec![0, 0, 0, 255]
    );
    let before = doc.clone();
    assert!(transform::resize(&mut doc, 0, 100, Resample::Nearest).is_err());
    assert_eq!(doc, before);
    doc.validate().unwrap();
}

#[test]
fn history_accounts_for_unique_tiles_and_keeps_large_canvas_strokes_undoable() {
    let mut doc = Document::new(7680, 4320).unwrap();
    let mut history = History::default();
    history.begin("Small edit on 8K canvas", &doc);
    doc.active_mut().source.set(10, 10, [255, 0, 0, 255]);
    history.finish(&doc);
    assert!(history.undo_label().is_some());
    assert_eq!(history.retained_bytes(&doc), 0); // Solid white backing is still shared by other live tiles.
    history.undo(&mut doc);
    assert_eq!(doc.active().source.get(10, 10), [255; 4]);
    assert!(history.retained_bytes(&doc) <= 128 * 128 * 4);
    history.redo(&mut doc);
    assert_eq!(doc.active().source.get(10, 10), [255, 0, 0, 255]);
}

#[test]
fn inverted_rectangle_outline_keeps_its_inner_boundary() {
    let mut mask = editing::rectangle(5, 5, rect(1, 1, 3, 3));
    for value in Arc::make_mut(&mut mask.values) {
        *value = 255 - *value;
    }
    assert_eq!(editing::bounds(&mask), Some(rect(0, 0, 5, 5)));
    let edges = editing::outline(&mask);
    assert_eq!(edges.len(), 8);
    assert!(edges.contains(&[[1.0, 1.0], [4.0, 1.0]]));
}

#[test]
fn failed_transform_preflight_is_atomic_and_bounded_across_layers() {
    let mut doc = Document::new(10, 10).unwrap();
    doc.add_layer();
    doc.add_layer();
    doc.add_layer();
    doc.add_layer();
    let before = doc.clone();
    // Five 8K output layers fit the per-raster limit but exceed the aggregate
    // processing budget. Rejection must happen before materializing those pixels.
    let error = transform::resize(&mut doc, 7680, 4320, Resample::Nearest).unwrap_err();
    assert!(error.contains("512 MiB"));
    assert_eq!(doc, before);
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: Some(Coverage {
            width: 8192,
            height: 10,
            values: Arc::new(vec![255; 81920]),
            enabled: true,
        }),
    });
    let before = doc.clone();
    assert!(transform::resize(&mut doc, 10000, 1000, Resample::Nearest).is_err());
    assert_eq!(doc, before);
}

#[test]
fn paste_expands_canvas_and_preserves_the_entire_clipboard_image() {
    let mut doc = Document::new(2, 2).unwrap();
    let pixels = Raster::new(5, 3, [40, 80, 120, 128]).unwrap();
    editing::paste(&mut doc, pixels).unwrap();
    assert_eq!((doc.width, doc.height), (5, 3));
    assert_eq!(doc.active().source.get(4, 2), [40, 80, 120, 128]);
    assert_eq!(doc.layers[0].source.get(0, 0), [255; 4]);
    assert_eq!(
        editing::bounds(doc.selection.as_ref().unwrap()),
        Some(rect(0, 0, 5, 3))
    );
}

#[test]
fn edited_selection_and_layer_stack_roundtrip_in_native_project() {
    let mut doc = Document::new(6, 4).unwrap();
    doc.add_layer();
    doc.active_mut().source.set(1, 1, [50, 100, 150, 200]);
    doc.selection = Some(editing::rectangle(6, 4, rect(1, 1, 2, 2)));
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    editing::move_selection(&mut doc, [2, 0]).unwrap();
    transform::orthogonal(&mut doc, Orthogonal::Clockwise).unwrap();
    transform::resize(&mut doc, 8, 12, Resample::Nearest).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("edited.palette");
    palette_core::io::save_project(&path, &doc).unwrap();
    let loaded = palette_core::io::read_project(&path).unwrap();
    assert_eq!(loaded, doc);
    assert_eq!(composite::render(&loaded), composite::render(&doc));
    assert_eq!(
        editing::copy(&loaded).unwrap().rgba(),
        editing::copy(&doc).unwrap().rgba()
    );
}
