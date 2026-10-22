use palette_core::{
    composite,
    document::{BlendMode, Document, Filter, FilterInstance},
    editing::{self, PixelRect},
    history::History,
    io,
    raster::{Coverage, Raster},
    text::{self, Alignment, TextEngine, TextSpec},
};
use std::sync::{Arc, OnceLock};

fn engine() -> &'static TextEngine {
    static ENGINE: OnceLock<TextEngine> = OnceLock::new();
    ENGINE.get_or_init(|| TextEngine::new(&[epaint_default_fonts::UBUNTU_LIGHT]).unwrap())
}

fn spec() -> TextSpec {
    TextSpec {
        text: "Palette".into(),
        font: 0,
        size: 32.0,
        line_height: 1.2,
        scroll_y: 0.,
        alignment: Alignment::Left,
        width: 200,
        height: 100,
        color: [200, 50, 25, 180],
        opacity: 0.5,
        background: None,
    }
}

fn alpha_bounds(pixels: &Raster) -> PixelRect {
    let values = (0..pixels.height())
        .flat_map(|y| (0..pixels.width()).map(move |x| pixels.get(x, y)[3]))
        .collect();
    editing::bounds(&Coverage {
        width: pixels.width(),
        height: pixels.height(),
        values: Arc::new(values),
        enabled: true,
    })
    .unwrap()
}

#[test]
fn region_compositing_matches_full_output_with_masks_filters_blending_and_offsets() {
    let mut doc = Document::new(130, 90).unwrap();
    doc.active_mut().source = Raster::new(130, 90, [50, 160, 220, 120]).unwrap();
    doc.add_layer();
    let filter_id = doc.allocate_id();
    let layer = doc.active_mut();
    layer.source = Raster::new(80, 100, [240, 70, 90, 150]).unwrap();
    layer.offset = [-10, 25];
    layer.opacity = 0.6;
    layer.blend = BlendMode::Screen;
    layer.filters.push(FilterInstance {
        id: filter_id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    layer.mask = Some(Coverage {
        width: 80,
        height: 100,
        values: Arc::new(vec![130; 8000]),
        enabled: true,
    });
    doc.add_layer();
    doc.active_mut().offset = [i32::MAX, i32::MIN];
    let full = composite::render(&doc);
    for region in [
        PixelRect {
            x: 0,
            y: 20,
            width: 30,
            height: 40,
        },
        PixelRect {
            x: 60,
            y: 60,
            width: 70,
            height: 30,
        },
    ] {
        let part = composite::render_region(&doc, region).unwrap();
        for y in 0..region.height {
            for x in 0..region.width {
                assert_eq!(part.get(x, y), full.get(region.x + x, region.y + y));
            }
        }
    }
    assert!(
        composite::render_region(
            &doc,
            PixelRect {
                x: u32::MAX,
                y: 0,
                width: 2,
                height: 1
            }
        )
        .is_err()
    );
}

#[test]
fn text_edges_keep_straight_color_and_correct_partial_alpha() {
    let pixels = engine().render(&spec()).unwrap();
    let mut feather = false;
    let mut solid = false;
    for y in 0..pixels.height() {
        for x in 0..pixels.width() {
            let pixel = pixels.get(x, y);
            assert!(pixel[3] <= 90);
            if pixel[3] > 0 {
                assert_eq!(&pixel[..3], &[200, 50, 25]);
                feather |= pixel[3] < 90;
                solid |= pixel[3] == 90;
            }
        }
    }
    assert!(feather && solid);
}

#[test]
fn text_wrap_alignment_newlines_and_clipping_are_deterministic() {
    let mut input = spec();
    input.text = "Hi".into();
    let left = alpha_bounds(&engine().render(&input).unwrap());
    input.alignment = Alignment::Right;
    let right = alpha_bounds(&engine().render(&input).unwrap());
    assert!(right.x > left.x + 100);
    input.alignment = Alignment::Center;
    let center = alpha_bounds(&engine().render(&input).unwrap());
    assert!(center.x > left.x && center.x < right.x);
    input.text = "Hi\nHi".into();
    let multiline = alpha_bounds(&engine().render(&input).unwrap());
    assert!(multiline.height > left.height);
    input.text = "Hi Hi Hi Hi".into();
    input.width = 45;
    assert!(alpha_bounds(&engine().render(&input).unwrap()).height > left.height);
    input.height = 12;
    let clipped = engine().render(&input).unwrap();
    assert_eq!(clipped.height(), 12);
    assert_eq!(clipped, engine().render(&input).unwrap());
}

#[test]
fn text_selection_new_layer_history_and_native_roundtrip_preserve_existing_pixels() {
    let mut doc = Document::new(250, 150).unwrap();
    doc.selection = Some(Coverage {
        width: 250,
        height: 150,
        values: Arc::new(vec![128; 250 * 150]),
        enabled: true,
    });
    let before = doc.clone();
    let mut pixels = engine().render(&spec()).unwrap();
    let original = pixels.clone();
    text::constrain(&mut pixels, &doc, [20, 30]);
    for y in 0..pixels.height() {
        for x in 0..pixels.width() {
            assert_eq!(
                pixels.get(x, y)[3],
                (f32::from(original.get(x, y)[3]) * 128.0 / 255.0).round() as u8
            );
        }
    }
    let mut history = History::default();
    history.begin("Text", &doc);
    assert!(text::insert(&mut doc, pixels.clone(), [20, 30], "Text: Palette").unwrap());
    history.finish(&doc);
    assert_eq!(doc.layers[0], before.layers[0]);
    assert_eq!(doc.active().source, pixels);
    let after = doc.clone();
    history.undo(&mut doc);
    assert_eq!(doc, before);
    history.redo(&mut doc);
    assert_eq!(doc, after);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("text.palette");
    io::save_project(&path, &doc).unwrap();
    assert_eq!(io::read_project(&path).unwrap(), doc);
    assert!(
        !text::insert(
            &mut doc,
            engine()
                .render(&TextSpec {
                    text: "   \n".into(),
                    ..spec()
                })
                .unwrap(),
            [0, 0],
            "Empty"
        )
        .unwrap()
    );
    assert_eq!(doc, after);
}

#[test]
fn text_rejects_invalid_parameters_before_allocating() {
    for invalid in [
        TextSpec {
            size: f32::NAN,
            ..spec()
        },
        TextSpec {
            size: 500.0,
            ..spec()
        },
        TextSpec {
            width: u32::MAX,
            ..spec()
        },
        TextSpec {
            height: 0,
            ..spec()
        },
        TextSpec { font: 1, ..spec() },
        TextSpec {
            text: "a".repeat(4097),
            ..spec()
        },
        TextSpec {
            opacity: -0.1,
            ..spec()
        },
        TextSpec {
            line_height: f32::INFINITY,
            ..spec()
        },
    ] {
        assert!(engine().render(&invalid).is_err());
    }
    assert!(TextEngine::new(&[]).is_err());
    assert!(TextEngine::new(&[b"invalid font"]).is_err());
}
