use palette_core::{
    brush::Brush,
    composite::{self, over},
    document::{BlendMode, Document, Filter, FilterInstance},
    history::History,
    io,
    raster::{Coverage, Raster},
};
use std::{fs, sync::Arc};

#[test]
fn straight_alpha_edges_and_transparent_rgb() {
    assert_eq!(
        over([0, 0, 255, 255], [255, 0, 0, 128], 1.0, BlendMode::Normal),
        [128, 0, 127, 255]
    );
    assert_eq!(
        over([0, 255, 0, 0], [255, 0, 0, 128], 1.0, BlendMode::Multiply),
        [255, 0, 0, 128]
    );
    assert_eq!(over([0; 4], [255, 0, 0, 0], 1.0, BlendMode::Screen), [0; 4]);
    assert_eq!(
        over(
            [128, 128, 128, 255],
            [128, 128, 128, 255],
            1.0,
            BlendMode::Multiply
        ),
        [64, 64, 64, 255]
    );
    assert_eq!(
        over(
            [128, 128, 128, 255],
            [128, 128, 128, 255],
            1.0,
            BlendMode::Screen
        ),
        [192, 192, 192, 255]
    );
}

#[test]
fn masks_and_filters_preserve_source_and_order() {
    let mut doc = Document::from_raster(Raster::new(1, 1, [40, 80, 120, 255]).unwrap());
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    doc.active_mut().mask = Some(Coverage {
        width: 1,
        height: 1,
        values: Arc::new(vec![128]),
        enabled: true,
    });
    assert_eq!(composite::render(&doc).get(0, 0), [215, 175, 135, 128]);
    assert_eq!(doc.active().source.get(0, 0), [40, 80, 120, 255]);
    doc.active_mut().filters[0].enabled = false;
    assert_eq!(composite::render(&doc).get(0, 0), [40, 80, 120, 128]);
    doc.active_mut().offset = [i32::MAX, i32::MIN];
    assert_eq!(composite::render(&doc).get(0, 0), [0; 4]);
}

#[test]
fn continuous_soft_brush_and_destination_out_eraser() {
    let mut raster = Raster::new(128, 32, [0; 4]).unwrap();
    let mut brush = Brush {
        diameter: 12.0,
        opacity: 0.5,
        hardness: 0.0,
        color: [255, 0, 0, 255],
        erase: false,
    };
    brush.segment(&mut raster, [4.5, 16.5], [120.5, 16.5]);
    for x in 5..120 {
        assert_eq!(raster.get(x, 16)[3], 128, "gap at {x}");
    }
    assert!(raster.get(60, 20)[3] < raster.get(60, 16)[3]);
    let source = raster.clone();
    brush.erase = true;
    brush.segment(&mut raster, [50.5, 16.5], [70.5, 16.5]);
    assert_eq!(raster.get(60, 16), [255, 0, 0, 64]);
    assert_eq!(source.get(60, 16), [255, 0, 0, 128]);
}

#[test]
fn history_restores_pixels_layer_properties_and_clean_revision() {
    let mut doc = Document::new(2, 2).unwrap();
    let mut history = History::default();
    history.begin("Layer", &doc);
    doc.add_layer();
    let saved = history.revision;
    history.begin("Paint", &doc);
    doc.active_mut().source.set(0, 0, [255, 0, 0, 200]);
    history.undo(&mut doc);
    assert_eq!(history.revision, saved);
    assert_eq!(doc.active().source.get(0, 0), [0; 4]);
    history.redo(&mut doc);
    assert_eq!(doc.active().source.get(0, 0), [255, 0, 0, 200]);
    history.undo(&mut doc);
    history.begin("Rename", &doc);
    doc.active_mut().name = "New branch".into();
    assert!(history.redo_label().is_none());
    assert_ne!(history.revision, saved);
}

#[test]
fn native_project_roundtrip_retains_editable_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layers.palette");
    let mut doc = Document::new(3, 2).unwrap();
    doc.add_layer();
    doc.active_mut().source.set(1, 1, [255, 0, 0, 128]);
    doc.active_mut().opacity = 0.6;
    doc.active_mut().blend = BlendMode::Multiply;
    doc.active_mut().mask = Some(Coverage {
        width: 3,
        height: 2,
        values: Arc::new(vec![255, 128, 0, 255, 255, 255]),
        enabled: true,
    });
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Brightness { amount: 0.2 },
        mask: None,
    });
    let id = doc.allocate_id();
    doc.active_mut()
        .effects
        .push(palette_core::document::EffectInstance {
            id,
            enabled: true,
            effect: palette_core::document::Effect::ColorOverlay {
                color: [10, 20, 30],
                opacity: 0.3,
            },
        });
    doc.active_mut().offset = [-1, 1];
    doc.active_mut().locked = true;
    io::save_project(&path, &doc).unwrap();
    let reopened = io::open(&path).unwrap();
    assert_eq!(doc, reopened);
    assert_eq!(composite::render(&doc), composite::render(&reopened));
    // Existing targets are replaced atomically, including on Windows.
    doc.active_mut().name = "Renamed".into();
    io::save_project(&path, &doc).unwrap();
    assert_eq!(io::open(&path).unwrap(), doc);
}

#[test]
fn raster_codecs_roundtrip_dimensions_and_lossless_alpha() {
    let dir = tempfile::tempdir().unwrap();
    let doc = Document::from_raster(
        Raster::from_rgba(
            2,
            2,
            &[
                255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 50, 50, 50, 255,
            ],
        )
        .unwrap(),
    );
    for ext in ["png", "bmp", "gif", "tiff", "webp", "jpg"] {
        let path = dir.path().join(format!("test.{ext}"));
        io::export(&path, &doc).unwrap();
        let loaded = io::open(&path).unwrap();
        assert_eq!((loaded.width, loaded.height), (2, 2), "{ext}");
        if ["png", "tiff", "webp"].contains(&ext) {
            assert_eq!(
                loaded.active().source.rgba(),
                doc.active().source.rgba(),
                "{ext}"
            );
        }
    }
}

#[test]
fn bad_project_or_failed_export_cannot_overwrite_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.palette");
    fs::write(&path, b"PALETTE\0\x63\0\0\0").unwrap();
    assert!(io::open(&path).unwrap_err().to_string().contains("version"));
    fs::write(&path, b"existing contents").unwrap();
    assert!(io::export(&path, &Document::new(1, 1).unwrap()).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"existing contents");
    assert!(matches!(
        io::open(&dir.path().join("missing.paint")),
        Err(io::FileError::Io(_))
    ));
    let impostor = dir.path().join("sample.paint");
    fs::write(&impostor, b"PALETTE\0\x01\0\0\0").unwrap();
    assert!(
        io::open(&impostor).is_err(),
        "renaming a native project is not Paint interoperability"
    );
}

#[test]
fn deserialized_invariants_are_checked() {
    let doc = Document::new(1, 1).unwrap();
    let mut value = serde_json::to_value(&doc).unwrap();
    value["active_layer"] = 999.into();
    let invalid: Document = serde_json::from_value(value).unwrap();
    assert!(invalid.validate().is_err());
    let mut value = serde_json::to_value(&doc).unwrap();
    value["layers"][0]["source"]["tiles"][0] = serde_json::json!([1, 2, 3]);
    let invalid: Document = serde_json::from_value(value).unwrap();
    assert!(invalid.validate().is_err());
}
