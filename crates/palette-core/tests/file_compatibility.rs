use flate2::{Compression, write::ZlibEncoder};
use palette_core::{
    document::{Document, Filter, FilterInstance},
    io,
    raster::Raster,
};
use std::{fs, io::Write, sync::Arc};

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
    assert_eq!(&fs::read(&current).unwrap()[8..12], &7u32.to_le_bytes());
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
    assert_eq!(&fs::read(&path).unwrap()[8..12], &7u32.to_le_bytes());
}

#[test]
fn real_avif_encodes_decodes_dimensions_alpha_and_quality_without_overwriting_on_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut source = Raster::new(32, 24, [0; 4]).unwrap();
    for y in 0..24 {
        for x in 0..32 {
            source.set(
                x,
                y,
                [
                    (x * 7) as u8,
                    (y * 9) as u8,
                    90,
                    if x < 8 {
                        0
                    } else if x < 16 {
                        128
                    } else {
                        255
                    },
                ],
            );
        }
    }
    let doc = Document::from_raster(source);
    let path = dir.path().join("image.avif");
    io::export_avif(
        &path,
        &doc,
        io::AvifOptions {
            quality: 90,
            speed: 10,
        },
    )
    .unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[4..8], b"ftyp");
    let decoded = io::open(&path).unwrap();
    assert_eq!([decoded.width, decoded.height], [32, 24]);
    for (x, alpha) in [(4, 0), (12, 128), (24, 255)] {
        assert!(decoded.active().source.get(x, 12)[3].abs_diff(alpha) <= 1);
    }
    let original = doc.active().source.get(24, 12);
    let actual = decoded.active().source.get(24, 12);
    assert!(
        original[..3]
            .iter()
            .zip(&actual[..3])
            .all(|(a, b)| a.abs_diff(*b) < 18)
    );
    io::export_avif(
        &path,
        &doc,
        io::AvifOptions {
            quality: 25,
            speed: 10,
        },
    )
    .unwrap();
    assert_ne!(fs::read(&path).unwrap(), bytes);
    let old = fs::read(&path).unwrap();
    assert!(
        io::export_avif(
            &path,
            &doc,
            io::AvifOptions {
                quality: 0,
                speed: 10
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), old);
    fs::write(&path, &old[..old.len() / 2]).unwrap();
    assert!(io::open(&path).is_err());
}

#[test]
fn avif_opaque_single_pixel_and_ten_bit_sources_are_decoded_by_the_real_codec() {
    let dir = tempfile::tempdir().unwrap();
    for (w, h, depth) in [
        (1, 1, ravif::BitDepth::Eight),
        (17, 9, ravif::BitDepth::Ten),
    ] {
        let pixels = vec![ravif::RGBA8::new(180, 90, 30, 255); w * h];
        let encoded = ravif::Encoder::new()
            .with_quality(95.)
            .with_speed(10)
            .with_bit_depth(depth)
            .with_num_threads(Some(1))
            .encode_rgba(ravif::Img::new(&pixels, w, h))
            .unwrap();
        let path = dir.path().join("opaque.avif");
        fs::write(&path, encoded.avif_file).unwrap();
        let doc = io::open(&path).unwrap();
        assert_eq!([doc.width, doc.height], [w as u32, h as u32]);
        let p = doc.active().source.get(0, 0);
        assert_eq!(p[3], 255);
        assert!(p[0].abs_diff(180) < 10 && p[1].abs_diff(90) < 10 && p[2].abs_diff(30) < 10);
    }
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
