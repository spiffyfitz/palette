use flate2::{Compression, write::ZlibEncoder};
use palette_core::{
    document::{Document, Filter, FilterInstance},
    io,
    raster::Raster,
};
use std::{fs, io::Write, sync::Arc};

#[test]
fn removed_neural_filters_are_rejected_without_changing_current_or_legacy_projects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("retired-filter.palette");
    let mut doc = Document::new(4, 4).unwrap();
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: false,
        filter: Filter::Invert,
        mask: None,
    });
    io::save_project(&path, &doc).unwrap();
    let original = fs::read(&path).unwrap();
    let length = u32::from_le_bytes(original[12..16].try_into().unwrap()) as usize;
    let manifest: serde_json::Value = serde_json::from_slice(&original[16..16 + length]).unwrap();
    for name in [
        "SmartPortrait",
        "LandscapeMixer",
        "PhotoRestoration",
        "Harmonization",
        "MakeupTransfer",
        "ColorTransfer",
        "DepthBlur",
        "SuperZoom",
        "SkinSmoothing",
        "Colorize",
        "JpegArtifactsRemoval",
        "StyleTransfer",
    ] {
        for version in [1u32, 8] {
            let mut metadata = if version == 1 {
                serde_json::to_value(&doc).unwrap()
            } else {
                manifest.clone()
            };
            metadata["layers"][0]["filters"][0]["filter"] =
                serde_json::json!({name: {"strength": 1.0}});
            let json = serde_json::to_vec(&metadata).unwrap();
            let mut bytes = b"PALETTE\0".to_vec();
            bytes.extend_from_slice(&version.to_le_bytes());
            if version == 1 {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                encoder.write_all(&json).unwrap();
                bytes.extend(encoder.finish().unwrap());
            } else {
                bytes.extend_from_slice(&(json.len() as u32).to_le_bytes());
                bytes.extend_from_slice(&json);
                bytes.extend_from_slice(&original[16 + length..]);
            }
            fs::write(&path, &bytes).unwrap();
            let error = match io::open(&path) {
                Err(error) => error,
                Ok(_) => panic!("retired filters must fail clearly"),
            };
            assert!(
                error.to_string().contains("Neural Filters"),
                "{name}: {error}"
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }
}

#[test]
fn avif_import_and_export_are_explicitly_unsupported_and_preserve_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.avif");
    fs::write(&path, b"existing file").unwrap();
    let error = io::open(&path).err().unwrap();
    assert!(error.to_string().contains("AVIF support has been removed"));
    assert!(io::export(&path, &Document::new(4, 4).unwrap()).is_err());
    assert_eq!(fs::read(path).unwrap(), b"existing file");
}

#[test]
fn detected_animation_and_page_notices_match_the_imported_pixels_and_survive_misnamed_files() {
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes, notice) in [
        (
            "animated.webp",
            include_bytes!("fixtures/two-frame.webp").as_slice(),
            "first WebP frame",
        ),
        (
            "animated.png",
            include_bytes!("fixtures/default-image-apng.png").as_slice(),
            "PNG default image",
        ),
        (
            "pages.tiff",
            include_bytes!("fixtures/two-page.tiff").as_slice(),
            "first TIFF page",
        ),
    ] {
        let path = dir.path().join(name);
        fs::write(&path, bytes).unwrap();
        let report = io::open_with_report(&path).unwrap();
        assert!(report.notice.unwrap().contains(notice));
        assert_eq!([report.document.width, report.document.height], [4, 4]);
        assert_eq!(report.document.active().source.get(0, 0), [255, 0, 0, 255]);
        let renamed = dir.path().join("misnamed.bmp");
        fs::write(&renamed, bytes).unwrap();
        assert_eq!(
            io::open_with_report(&renamed).unwrap().notice,
            report.notice
        );
    }
    let gif = dir.path().join("animation.dat");
    let mut encoder = image::codecs::gif::GifEncoder::new(fs::File::create(&gif).unwrap());
    for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
        encoder
            .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                4,
                4,
                image::Rgba(color),
            )))
            .unwrap();
    }
    drop(encoder);
    let report = io::open_with_report(&gif).unwrap();
    assert!(report.notice.unwrap().contains("first GIF frame"));
    assert_eq!(report.document.active().source.get(0, 0), [255, 0, 0, 255]);
    for extension in ["png", "webp"] {
        let path = dir.path().join(format!("still.{extension}"));
        io::export(&path, &report.document).unwrap();
        assert!(io::open_with_report(&path).unwrap().notice.is_none());
    }
}

#[test]
fn version_one_migrates_to_current_without_losing_editable_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = Document::from_raster(Raster::new(130, 90, [255; 4]).unwrap());
    doc.add_layer();
    doc.active_mut().source.set(129, 89, [10, 30, 50, 128]);
    doc.active_mut().offset = [-2, 3];
    doc.active_mut().opacity = 0.7;
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let legacy = dir.path().join("legacy.palette");
    let mut metadata = serde_json::to_value(&doc).unwrap();
    metadata.as_object_mut().unwrap().remove("background");
    metadata
        .as_object_mut()
        .unwrap()
        .remove("background_visible");
    metadata.as_object_mut().unwrap().remove("metadata"); // Genuine old schema.
    let mut file = fs::File::create(&legacy).unwrap();
    file.write_all(b"PALETTE\0\x01\0\0\0").unwrap();
    let mut encoder = ZlibEncoder::new(file, Compression::fast());
    serde_json::to_writer(&mut encoder, &metadata).unwrap();
    encoder.finish().unwrap();
    let migrated = io::open(&legacy).unwrap();
    assert!(migrated == doc);
    let current = dir.path().join("current.palette");
    io::save_project(&current, &migrated).unwrap();
    assert_eq!(&fs::read(&current).unwrap()[8..12], &8u32.to_le_bytes());
    assert!(io::open(&current).unwrap() == doc);
    assert!(
        io::open(&legacy).unwrap() == doc,
        "migration never overwrites its source"
    );
}

#[test]
fn binary_assets_share_sparse_tiles_and_preserve_opaque_metadata_and_hidden_rgb() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = Document::new(4096, 4096).unwrap();
    doc.add_layer();
    doc.active_mut().source.set(4095, 4095, [10, 30, 50, 0]);
    doc.metadata.icc_profile = Arc::new(b"opaque ICC bytes".to_vec());
    doc.metadata.exif = Arc::new(b"opaque Exif bytes".to_vec());
    let path = dir.path().join("large-sparse.palette");
    io::save_project(&path, &doc).unwrap();
    assert!(fs::metadata(&path).unwrap().len() < 32 * 1024);
    assert!(io::open(&path).unwrap() == doc);
    let old = fs::read(&path).unwrap();
    doc.metadata.exif = Arc::new(vec![0; 4 * 1024 * 1024 + 1]);
    assert!(io::save_project(&path, &doc).is_err());
    assert_eq!(fs::read(&path).unwrap(), old);
}

#[test]
fn current_project_rejects_truncation_bombs_bad_references_corruption_and_unknown_versions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.palette");
    io::save_project(&path, &Document::new(4, 4).unwrap()).unwrap();
    let bytes = fs::read(&path).unwrap();
    for cut in [0, 8, 12, 16, bytes.len() - 1] {
        fs::write(&path, &bytes[..cut]).unwrap();
        assert!(io::open(&path).is_err());
    }
    for offset in [8, bytes.len() - 1] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0xff;
        fs::write(&path, bad).unwrap();
        assert!(io::open(&path).is_err());
    }
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes[16..16 + length]).unwrap();
    for mutation in 0..3 {
        let mut bad = manifest.clone();
        match mutation {
            0 => bad["asset_lengths"][0] = serde_json::json!(u64::MAX),
            1 => bad["layers"][0]["source"]["tiles"][0] = serde_json::json!(99999),
            _ => bad["layers"][0]["source"]["width"] = serde_json::json!(u32::MAX),
        }
        let json = serde_json::to_vec(&bad).unwrap();
        let mut result = bytes[..12].to_vec();
        result.extend_from_slice(&(json.len() as u32).to_le_bytes());
        result.extend_from_slice(&json);
        result.extend_from_slice(&bytes[16 + length..]);
        fs::write(&path, result).unwrap();
        assert!(io::open(&path).is_err());
    }
}

#[test]
fn version_two_defaults_missing_backing_to_transparent_then_saves_current_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v2.palette");
    let doc = Document::from_raster(Raster::new(5, 4, [255; 4]).unwrap());
    io::save_project(&path, &doc).unwrap();
    let bytes = fs::read(&path).unwrap();
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut manifest: serde_json::Value = serde_json::from_slice(&bytes[16..16 + length]).unwrap();
    manifest.as_object_mut().unwrap().remove("background");
    manifest
        .as_object_mut()
        .unwrap()
        .remove("background_visible");
    let json = serde_json::to_vec(&manifest).unwrap();
    let mut v2 = b"PALETTE\0".to_vec();
    v2.extend_from_slice(&2u32.to_le_bytes());
    v2.extend_from_slice(&(json.len() as u32).to_le_bytes());
    v2.extend_from_slice(&json);
    v2.extend_from_slice(&bytes[16 + length..]);
    fs::write(&path, &v2).unwrap();
    let migrated = io::open(&path).unwrap();
    assert!(migrated == doc);
    assert_eq!(fs::read(&path).unwrap(), v2);
    io::save_project(&path, &migrated).unwrap();
    assert_eq!(&fs::read(&path).unwrap()[8..12], &8u32.to_le_bytes());
}

#[test]
fn raster_import_applies_exif_orientation_and_preserves_original_metadata_in_native_project() {
    use image::ImageEncoder;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oriented.png");
    let exif = vec![
        b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut encoder = image::codecs::png::PngEncoder::new(fs::File::create(&path).unwrap());
    encoder.set_exif_metadata(exif.clone()).unwrap();
    encoder
        .write_image(
            &[255, 0, 0, 255].repeat(6),
            2,
            3,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    let doc = io::open(&path).unwrap();
    assert_eq!([doc.width, doc.height], [3, 2]);
    assert_eq!(*doc.metadata.exif, exif);
    let project = dir.path().join("oriented.palette");
    io::save_project(&project, &doc).unwrap();
    assert!(io::open(&project).unwrap() == doc);
}
