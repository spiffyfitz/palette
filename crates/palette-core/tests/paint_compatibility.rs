//! Specimen-derived expectations, plus generated offsets/alpha and hostile input.
use palette_core::{
    composite,
    document::{BlendMode, Document, Filter, FilterInstance},
    io::{self, paint},
    raster::{Coverage, Raster},
};
use std::{fs, io::Write, path::Path, sync::Arc};

const SPECIMEN: &[u8] = include_bytes!("fixtures/microsoft-paint.paint");
fn sample() -> Document {
    io::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/microsoft-paint.paint"))
        .unwrap()
}

#[test]
fn microsoft_specimen_has_two_ordered_transparent_layers_and_white_overlay_backing() {
    let doc = sample();
    assert_eq!([doc.width, doc.height], [819, 594]);
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.background, [255; 4]);
    assert_eq!(
        doc.layers.iter().map(|l| l.offset).collect::<Vec<_>>(),
        [[0, 0], [0, 0]]
    );
    let counts: Vec<_> = doc
        .layers
        .iter()
        .map(|l| {
            let mut counts = [0usize; 3];
            for y in 0..594 {
                for x in 0..819 {
                    counts[match l.source.get(x, y)[3] {
                        0 => 0,
                        255 => 2,
                        _ => 1,
                    }] += 1;
                }
            }
            counts
        })
        .collect();
    assert_eq!(counts, [[338184, 0, 148302], [454052, 1486, 30948]]);
    assert_eq!(doc.layers[0].source.get(200, 150), [237, 28, 36, 255]);
    assert_eq!(doc.layers[0].source.get(300, 217), [237, 28, 36, 255]);
    assert_eq!(doc.layers[1].source.get(300, 217), [0, 0, 0, 255]);
    let pixels = composite::render(&doc);
    assert_eq!(pixels.get(0, 0), [255; 4]);
    assert_eq!(pixels.get(200, 150), [237, 28, 36, 255]);
    assert_eq!(pixels.get(300, 217), [0, 0, 0, 255]);
    // Independent Python/Pillow raw-Deflate/BGRA oracle channel totals. This
    // detects order/alpha/backing mistakes that a self round-trip cannot find.
    let mut sums = [0u64; 4];
    for y in 0..594 {
        for x in 0..819 {
            for (sum, channel) in sums.iter_mut().zip(pixels.get(x, y)) {
                *sum += u64::from(channel);
            }
        }
    }
    assert_eq!(sums, [113866085, 89472810, 90406422, 124053930]);
    assert!(doc.metadata.exif.starts_with(b"II\x2a\0"));
    assert_eq!(doc.metadata.exif.len(), 54);
}

fn assert_content(a: &Document, b: &Document) {
    assert_eq!([a.width, a.height], [b.width, b.height]);
    assert_eq!(a.background, b.background);
    assert_eq!(a.metadata, b.metadata);
    assert_eq!(a.layers.len(), b.layers.len());
    for (a, b) in a.layers.iter().zip(&b.layers) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.offset, b.offset);
        assert_eq!(
            a.source.rgba(),
            b.source.rgba(),
            "logical pixels and hidden RGB"
        );
    }
    assert!(composite::render(a) == composite::render(b));
}

#[test]
fn specimen_edits_round_trip_as_two_layers_and_native_v3_keeps_canvas_backing() {
    let mut doc = sample();
    doc.layers[0].source.set(0, 0, [90, 30, 180, 128]);
    doc.layers[1].source.set(800, 580, [1, 2, 3, 0]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edited.paint");
    paint::save_paint(&path, &doc).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[4..8], b"ftyp");
    assert_eq!(&bytes[8..12], b"mif1");
    assert_content(&doc, &io::open(&path).unwrap());
    let native = dir.path().join("full.palette");
    io::save_project(&native, &doc).unwrap();
    assert_eq!(&fs::read(&native).unwrap()[8..12], &8u32.to_le_bytes());
    assert!(io::open(&native).unwrap() == doc);
    assert_eq!(
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/microsoft-paint.paint")
        )
        .unwrap(),
        SPECIMEN
    );
}

#[test]
fn different_source_dimensions_offsets_hidden_rgb_partial_alpha_and_metadata_survive() {
    for offset in [[-2, 3], [2, -3], [40000, -40000]] {
        let mut doc = Document::from_raster(Raster::new(7, 5, [10, 20, 30, 128]).unwrap());
        doc.background = [30, 40, 50, 60];
        doc.add_layer();
        doc.active_mut().source = Raster::new(2, 3, [90, 80, 70, 127]).unwrap();
        doc.active_mut().source.set(1, 1, [11, 22, 33, 0]);
        doc.active_mut().name = "Foreground ♥".into();
        doc.active_mut().offset = offset;
        doc.metadata.icc_profile = Arc::new(b"opaque ICC profile bytes".to_vec());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("offset.paint");
        paint::save_paint(&path, &doc).unwrap();
        assert_content(&doc, &io::open(&path).unwrap());
    }
}

#[test]
fn declared_zlib_and_uncompressed_packed_sources_decode_without_codec_guessing() {
    let doc = sample();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("codec.paint");
    for compressed in [false, true] {
        let payloads: Vec<_> = doc
            .layers
            .iter()
            .map(|l| {
                let mut bgra = l.source.rgba();
                for p in bgra.as_chunks_mut::<4>().0 {
                    p.swap(0, 2);
                }
                if compressed {
                    let mut z =
                        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
                    z.write_all(&bgra).unwrap();
                    z.finish().unwrap()
                } else {
                    bgra
                }
            })
            .collect();
        let mut bytes = SPECIMEN[..625].to_vec();
        if compressed {
            bytes[297..301].copy_from_slice(b"zlib");
        } else {
            bytes[289..293].copy_from_slice(b"test"); // optional future property replaces cmpC
            bytes[461] = 1;
            bytes[470] = 1; // clear essential flags on the now-unknown property
        }
        let len1 = payloads[0].len() as u32;
        let len2 = payloads[1].len() as u32;
        for (at, value) in [
            (217, len1),
            (229, 625 + len1),
            (233, len2),
            (261, 625 + len1 + len2),
            (617, 8 + len1 + len2 + 58),
        ] {
            bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
        }
        for payload in payloads {
            bytes.extend_from_slice(&payload);
        }
        bytes.extend_from_slice(&SPECIMEN[9796..]);
        fs::write(&path, bytes).unwrap();
        assert_content(&doc, &io::open(&path).unwrap());
    }
}

#[test]
fn unsupported_editable_features_cannot_overwrite_a_paint_file_and_baking_keeps_original() {
    let mut doc = Document::new(5, 4).unwrap();
    doc.add_layer();
    doc.active_mut().source = Raster::new(5, 4, [100, 80, 60, 200]).unwrap();
    doc.active_mut().mask = Some(Coverage {
        width: 5,
        height: 4,
        enabled: true,
        values: Arc::new(vec![128; 20]),
    });
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let original = doc.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.paint");
    fs::write(&path, b"existing file").unwrap();
    assert!(paint::save_paint(&path, &doc).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"existing file");
    let compatible = paint::compatible_copy(&doc).unwrap();
    assert!(paint::limitations(&compatible).is_empty());
    assert_eq!(compatible.layers.len(), 2);
    assert!(composite::render(&compatible) == composite::render(&doc));
    assert!(doc == original);
    paint::save_paint(&path, &compatible).unwrap();
    assert_content(&compatible, &io::open(&path).unwrap());
    // A backdrop-dependent blend merges only its required prefix, retaining
    // independent layers above it. No destructive mutation of the source doc.
    doc.active_mut().blend = BlendMode::Multiply;
    doc.add_layer();
    doc.active_mut().source.set(4, 3, [255, 0, 0, 255]);
    let converted = paint::compatible_copy(&doc).unwrap();
    assert_eq!(converted.layers.len(), 2);
    // Baking the backing into a raster changes unused tile padding. Compare
    // visible RGBA rather than the storage representation of those tiles.
    assert_eq!(
        composite::render(&converted).rgba(),
        composite::render(&doc).rgba()
    );
}

#[test]
fn malformed_boxes_locations_decompression_and_required_properties_fail_safely() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.paint");
    for cut in [0, 4, 8, 31, 32, 40, 188, 553, SPECIMEN.len() - 1] {
        fs::write(&path, &SPECIMEN[..cut]).unwrap();
        assert!(io::open(&path).is_err(), "truncation {cut}");
    }
    // Known byte offsets are deliberately fixture-specific, not a parser.
    for (offset, bytes) in [
        (0, vec![255; 4]),       // Box extends past EOF.
        (213, vec![255; 4]),     // Item1 extent out of bounds.
        (314, vec![255; 4]),     // Excessive image dimensions.
        (461, vec![0xff]),       // Invalid essential property index.
        (373, b"nope".to_vec()), // Unsupported uncC profile.
        (297, b"nope".to_vec()), // Unsupported compression.
        (625, vec![255; 16]),    // Broken raw Deflate.
        (505, vec![0, 3]),       // Derived-image cycle.
    ] {
        let mut bad = SPECIMEN.to_vec();
        bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
        fs::write(&path, bad).unwrap();
        assert!(io::open(&path).is_err(), "mutation {offset}");
    }
    // Treat an existing optional pixi as an unknown future property. It may be
    // skipped only when nonessential; the required bit changes this to failure.
    let mut optional = SPECIMEN.to_vec();
    optional[429..433].copy_from_slice(b"test");
    fs::write(&path, &optional).unwrap();
    assert!(io::open(&path).is_ok());
    optional[466] |= 0x80; // Source1's sixth association (pixi).
    fs::write(&path, optional).unwrap();
    assert!(io::open(&path).is_err());
    for length in [3989u32, 3991] {
        let mut bad = SPECIMEN.to_vec();
        bad[217..221].copy_from_slice(&length.to_be_bytes());
        fs::write(&path, bad).unwrap();
        assert!(
            io::open(&path).is_err(),
            "truncated or trailing Deflate stream"
        );
    }
    let mut too_many = SPECIMEN.to_vec();
    for _ in 0..4096 {
        too_many.extend_from_slice(b"\0\0\0\x08free");
    }
    fs::write(&path, too_many).unwrap();
    assert!(io::open(&path).is_err(), "box budget");
}
