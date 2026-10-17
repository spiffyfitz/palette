use palette_core::{
    composite,
    document::{BlendMode, Document, Effect, EffectInstance, Filter, FilterInstance},
    editing::{self, PixelRect},
    io,
    processing::{self, Kind, Operation, Value},
    raster::{Coverage, Raster},
};
use std::sync::Arc;
fn filter(doc: &mut Document, f: Filter) -> u64 {
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: f,
        mask: None,
    });
    id
}
fn effect(doc: &mut Document, e: Effect) -> u64 {
    let id = doc.allocate_id();
    doc.active_mut().effects.push(EffectInstance {
        id,
        enabled: true,
        effect: e,
    });
    id
}
fn shadow() -> Effect {
    Effect::DropShadow {
        color: [0; 3],
        opacity: 0.5,
        angle: 180.,
        distance: 3.,
        spread: 0.,
        size: 0.,
    }
}

#[test]
fn small_gaussian_matches_independent_kernel_and_ignores_hidden_transparent_color() {
    let mut source = Raster::new(37, 3, [0, 0, 255, 0]).unwrap();
    for y in 0..3 {
        source.set(18, y, [255, 0, 0, 255]);
    }
    let before = source.clone();
    let radius = 0.8;
    let blurred = processing::gaussian_blur(&source, radius).unwrap();
    let weights: Vec<_> = (-3..=3)
        .map(|x| (-0.5 * (f64::from(x) / 0.8).powi(2)).exp())
        .collect();
    let sum: f64 = weights.iter().sum();
    for (i, w) in weights.iter().enumerate() {
        let p = blurred.get(15 + i as u32, 1);
        assert!(p[3].abs_diff((255. * w / sum).round() as u8) <= 1);
        if p[3] > 0 {
            assert_eq!(
                &p[..3],
                &[255, 0, 0],
                "hidden blue must not contaminate a red edge"
            );
        }
    }
    assert_eq!(source, before);
    for radius in [0.1, 0.8, 2., 7., 64.] {
        let constant = Raster::new(9, 7, [70, 80, 90, 120]).unwrap();
        assert_eq!(
            processing::gaussian_blur(&constant, radius).unwrap().rgba(),
            constant.rgba()
        );
    }
    assert_eq!(processing::gaussian_blur(&source, 0.).unwrap(), source);
}

#[test]
fn tiled_blur_is_symmetric_across_tile_boundaries_and_cancellable() {
    let mut source = Raster::new(540, 9, [0; 4]).unwrap();
    for y in 0..9 {
        source.set(255, y, [230, 90, 20, 200]);
    }
    for radius in [0.8, 2., 7.5, 32.] {
        let blurred = processing::gaussian_blur(&source, radius).unwrap();
        for d in 0..100 {
            assert_eq!(
                blurred.get(255 - d, 4),
                blurred.get(255 + d, 4),
                "radius {radius}, d {d}"
            );
        }
    }
    let calls = std::cell::Cell::new(0);
    let cancel = || {
        calls.set(calls.get() + 1);
        calls.get() > 8
    };
    assert!(
        processing::gaussian_blur_cancel(&source, 12., &cancel)
            .unwrap_err()
            .contains("cancelled")
    );
    assert_eq!(source.get(255, 4), [230, 90, 20, 200]);
}

#[test]
fn shadow_uses_masked_alpha_extends_outside_source_and_updates_after_source_edits() {
    let mut doc = Document::from_raster(Raster::new(8, 8, [0; 4]).unwrap());
    doc.width = 20;
    doc.height = 16;
    doc.active_mut().offset = [4, 2];
    doc.active_mut().source.set(2, 3, [240, 30, 20, 255]);
    effect(&mut doc, shadow());
    doc.validate().unwrap();
    let before = doc.active().source.clone();
    let rendered = composite::render(&doc);
    assert_eq!(rendered.get(6, 5), [240, 30, 20, 255]);
    assert_eq!(rendered.get(9, 5), [0, 0, 0, 128]);
    doc.active_mut().opacity = 0.5;
    let half = composite::render(&doc);
    assert_eq!(half.get(6, 5)[3], 128);
    assert_eq!(
        half.get(9, 5)[3],
        64,
        "layer opacity applies once to foreground and shadow"
    );
    doc.active_mut().source.set(7, 3, [240, 30, 20, 255]);
    assert!(
        composite::render(&doc).get(14, 5)[3] > 0,
        "shadow survives beyond source bounds"
    );
    doc.active_mut().mask = Some(Coverage {
        width: 8,
        height: 8,
        values: Arc::new(vec![0; 64]),
        enabled: true,
    });
    assert!(composite::render(&doc).rgba().iter().all(|v| *v == 0));
    assert_eq!(before.get(2, 3), doc.active().source.get(2, 3));
}

#[test]
fn glow_filter_order_filter_masks_and_native_and_paint_copies_remain_editable() {
    let mut doc = Document::from_raster(Raster::new(12, 10, [0; 4]).unwrap());
    doc.width = 30;
    doc.height = 24;
    doc.active_mut().offset = [8, 6];
    for y in 2..8 {
        for x in 0..8 {
            doc.active_mut().source.set(x, y, [230, 50, 20, 255]);
        }
    }
    filter(&mut doc, Filter::GaussianBlur { radius: 0.8 });
    effect(
        &mut doc,
        Effect::OuterGlow {
            color: [40, 210, 240],
            opacity: 0.8,
            spread: 0.3,
            size: 2.,
        },
    );
    effect(&mut doc, shadow());
    doc.validate().unwrap();
    let source = doc.active().source.clone();
    let rendered = composite::render(&doc);
    assert!(
        rendered.get(7, 11)[3] > 0,
        "glow extends outside original local bounds"
    );
    let copy = editing::copy(&doc).unwrap();
    assert_eq!(
        copy, rendered,
        "clipboard appearance includes external styles"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live.palette");
    io::save_project(&path, &doc).unwrap();
    assert!(io::open(&path).unwrap() == doc);
    let compatible = io::paint::compatible_copy(&doc).unwrap();
    assert_eq!(composite::render(&compatible), rendered);
    io::paint::save_paint(&dir.path().join("baked.paint"), &compatible).unwrap();
    assert_eq!(
        composite::render(&io::open(&dir.path().join("baked.paint")).unwrap()),
        rendered
    );
    assert_eq!(doc.active().source, source);
    assert!(!doc.active().filters.is_empty() && !doc.active().effects.is_empty());

    let mut stripes = Document::new(15, 3).unwrap();
    for y in 0..3 {
        for x in 0..15 {
            stripes.active_mut().source.set(
                x,
                y,
                if x.is_multiple_of(2) {
                    [0, 0, 0, 255]
                } else {
                    [255; 4]
                },
            );
        }
    }
    filter(&mut stripes, Filter::Brightness { amount: 0.8 });
    filter(&mut stripes, Filter::GaussianBlur { radius: 1. });
    let first = composite::render(&stripes).get(7, 1);
    stripes.active_mut().filters.swap(0, 1);
    let second = composite::render(&stripes).get(7, 1);
    assert!(second[0] > first[0] + 10);
    stripes.active_mut().filters[0].mask = Some(Coverage {
        width: 15,
        height: 3,
        values: Arc::new(vec![0; 45]),
        enabled: true,
    });
    let masked = composite::render(&stripes);
    stripes.active_mut().filters[0].enabled = false;
    assert_eq!(composite::render(&stripes), masked);
}

#[test]
fn cached_and_region_composites_match_with_offsets_modes_masks_and_styles() {
    let mut doc = Document::new(28, 22).unwrap();
    doc.add_layer();
    doc.active_mut().source = Raster::new(15, 12, [80, 160, 230, 150]).unwrap();
    doc.active_mut().offset = [-2, 4];
    filter(&mut doc, Filter::GaussianBlur { radius: 2.5 });
    effect(&mut doc, shadow());
    let mut renderer = composite::Renderer::default();
    for mode in BlendMode::ALL {
        doc.active_mut().blend = mode;
        let full = renderer.render(&doc).unwrap();
        let evals = renderer.evaluations;
        assert_eq!(renderer.render(&doc).unwrap(), full);
        assert_eq!(renderer.evaluations, evals);
        assert!(renderer.cache_hits > 0);
        let region = PixelRect {
            x: 3,
            y: 2,
            width: 19,
            height: 17,
        };
        let patch = composite::render_region(&doc, region).unwrap();
        for y in 0..region.height {
            for x in 0..region.width {
                assert_eq!(patch.get(x, y), full.get(x + region.x, y + region.y));
            }
        }
    }
    let evals = renderer.evaluations;
    doc.layers[0].source.set(0, 0, [1, 2, 3, 255]);
    renderer.render(&doc).unwrap();
    assert_eq!(renderer.evaluations, evals);
    doc.active_mut().source.set(0, 0, [255, 0, 0, 255]);
    renderer.render(&doc).unwrap();
    assert_eq!(renderer.evaluations, evals + 1);
    let frame = renderer.render_frame(&doc).unwrap();
    assert!(frame.pixels.rgba() == renderer.render(&doc).unwrap().rgba());
    assert_eq!(frame.thumbnails.len(), 1);
    assert_eq!(frame.thumbnails[0].0, doc.active_layer);
    assert_eq!(frame.thumbnails[0].1.width(), 48);
    let bytes = processing::prepare_layer(doc.active()).unwrap().byte_len();
    let coverage = Coverage {
        width: 15,
        height: 12,
        enabled: true,
        values: Arc::new(vec![255; 180]),
    };
    doc.active_mut().mask = Some(coverage.clone());
    doc.active_mut().filters[0].mask = Some(coverage);
    assert_eq!(
        processing::prepare_layer(doc.active()).unwrap().byte_len(),
        bytes + 360
    );
}

#[test]
fn registry_parameters_validate_and_bad_spatial_values_cannot_overwrite_a_project() {
    for descriptor in processing::REGISTRY {
        let mut op = descriptor.kind.default_operation();
        op.validate().unwrap();
        assert_eq!(op.kind(), descriptor.kind);
        for p in descriptor.parameters {
            if let Some(Value::Number(_)) = op.value(p.key) {
                let before = op.clone();
                assert!(op.set_value(p.key, Value::Number(f32::NAN)).is_err());
                assert_eq!(op, before);
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("safe.palette");
    let mut doc = Document::new(5, 5).unwrap();
    io::save_project(&path, &doc).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    filter(
        &mut doc,
        Filter::GaussianBlur {
            radius: f32::INFINITY,
        },
    );
    assert!(io::save_project(&path, &doc).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(matches!(
        Kind::GaussianBlur.default_operation(),
        Operation::Filter(_)
    ));
    let mut large = Document::new(4000, 4000).unwrap();
    for _ in 0..9 {
        effect(&mut large, shadow());
    }
    assert!(large.validate().unwrap_err().contains("512 MiB"));
}
