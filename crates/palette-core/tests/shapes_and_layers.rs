use palette_core::{
    composite,
    document::{Document, Filter, FilterInstance},
    editing, io,
    raster::{Coverage, Raster},
    shapes::{self, Kind, Style},
};
fn blank() -> Document {
    Document::from_raster(Raster::new(96, 96, [0; 4]).unwrap())
}
fn fill() -> Style {
    Style {
        outline: None,
        fill: Some([240, 80, 20, 255]),
        width: 3.,
        opacity: 1.,
    }
}
const SHAPES: [Kind; 20] = [
    Kind::Rectangle,
    Kind::Ellipse,
    Kind::RoundedRectangle,
    Kind::Triangle,
    Kind::Diamond,
    Kind::Pentagon,
    Kind::Hexagon,
    Kind::Star,
    Kind::RightArrow,
    Kind::DownArrow,
    Kind::LeftArrow,
    Kind::UpArrow,
    Kind::RightTriangle,
    Kind::FourPointStar,
    Kind::SixPointStar,
    Kind::RectangularCallout,
    Kind::OvalCallout,
    Kind::CloudCallout,
    Kind::Heart,
    Kind::Lightning,
];
#[test]
fn all_closed_shapes_have_real_coverage_and_reverse_drag_is_identical() {
    for kind in SHAPES {
        let mut a = blank();
        let mut b = a.clone();
        shapes::paint(&mut a, kind, [12.25, 10.5], [82.25, 80.5], fill()).unwrap();
        shapes::paint(&mut b, kind, [82.25, 80.5], [12.25, 10.5], fill()).unwrap();
        assert_eq!(a, b, "{kind:?}");
        assert!(
            a.active()
                .source
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] == 255),
            "solid interior {kind:?}"
        );
        assert_eq!(a.active().source.get(5, 5), [0; 4]);
        assert!(
            a.active()
                .source
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] > 0 && p[3] < 255),
            "antialias {kind:?}"
        );
        let path = shapes::path(kind, [12.25, 10.5], [82.25, 80.5]);
        assert_eq!(
            shapes::triangles(&path).len(),
            path.points.len() - 2,
            "tessellation {kind:?}"
        );
    }
}
#[test]
fn independent_outline_fill_and_selection_have_correct_alpha() {
    let mut doc = blank();
    let style = Style {
        outline: Some([0, 0, 0, 255]),
        opacity: 0.5,
        ..fill()
    };
    shapes::paint(&mut doc, Kind::Rectangle, [10., 10.], [70., 70.], style).unwrap();
    assert_eq!(doc.active().source.get(40, 40), [240, 80, 20, 128]);
    assert_eq!(doc.active().source.get(10, 40)[3], 192);
    assert_eq!(doc.active().source.get(10, 10)[3], 192); // no opacity buildup at joins
    let mut selected = blank();
    selected.selection = Some(Coverage {
        enabled: true,
        width: 96,
        height: 96,
        values: std::sync::Arc::new(vec![128; 96 * 96]),
    });
    shapes::paint(
        &mut selected,
        Kind::Rectangle,
        [10., 10.],
        [70., 70.],
        Style {
            opacity: 0.5,
            ..fill()
        },
    )
    .unwrap();
    assert_eq!(selected.active().source.get(40, 40)[3], 64);
}
#[test]
fn shape_on_translated_layer_clips_to_canvas_selection_and_respects_lock() {
    let mut doc = blank();
    doc.active_mut().offset = [20, 10];
    doc.selection = Some(editing::rectangle(
        96,
        96,
        editing::PixelRect {
            x: 30,
            y: 30,
            width: 10,
            height: 10,
        },
    ));
    shapes::paint(&mut doc, Kind::Rectangle, [20., 20.], [70., 70.], fill()).unwrap();
    let result = composite::render(&doc);
    assert_eq!(result.get(35, 35), [240, 80, 20, 255]);
    assert_eq!(result.get(25, 25), [0; 4]);
    doc.active_mut().locked = true;
    let before = doc.clone();
    assert!(!shapes::paint(&mut doc, Kind::Star, [0., 0.], [90., 90.], fill()).unwrap());
    assert_eq!(doc, before);
    assert!(shapes::paint(&mut doc, Kind::Star, [f32::NAN, 0.], [90., 90.], fill()).is_err());
    assert_eq!(doc, before);
}
#[test]
fn whole_layer_translation_preserves_processing_and_roundtrips() {
    let mut doc = blank();
    doc.active_mut().source.set(20, 20, [100, 60, 30, 255]);
    doc.active_mut().mask = Some(Coverage {
        enabled: true,
        width: 96,
        height: 96,
        values: std::sync::Arc::new(vec![128; 96 * 96]),
    });
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    let before = doc.active().clone();
    editing::translate_layer(&mut doc, [-5, 12]).unwrap();
    let mut expected = before;
    expected.offset = [-5, 12];
    assert_eq!(doc.active(), &expected);
    assert_eq!(composite::render(&doc).get(15, 32), [155, 195, 225, 128]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("moved.palette");
    io::save_project(&path, &doc).unwrap();
    assert_eq!(io::open(&path).unwrap(), doc);
    doc.active_mut().offset = [i32::MAX, 0];
    let original = doc.clone();
    assert!(editing::translate_layer(&mut doc, [1, 0]).is_err());
    assert_eq!(doc, original);
    doc.active_mut().locked = true;
    assert!(!editing::translate_layer(&mut doc, [-1, 0]).unwrap());
}
#[test]
fn constrained_geometry_and_degenerate_shapes_are_bounded() {
    assert_eq!(
        shapes::constrain(Kind::Rectangle, [10., 10.], [30., 20.]),
        [30., 30.]
    );
    let line = shapes::constrain(Kind::Line, [0., 0.], [30., 1.]);
    assert!(line[1].abs() < 0.001);
    let mut doc = blank();
    for kind in SHAPES {
        shapes::paint(&mut doc, kind, [-50., -50.], [-20., -20.], fill()).unwrap();
        shapes::paint(&mut doc, kind, [30., 30.], [30., 30.], fill()).unwrap();
    }
    assert_eq!(doc.active().source, blank().active().source);
}

#[test]
fn flat_shape_contours_and_raster_strokes_never_escape_the_drag_bounds() {
    for kind in SHAPES {
        for size in [
            [0., 60.],
            [0.000001, 60.],
            [0.1, 60.],
            [60., 0.000001],
            [60., 0.],
            [0., 0.],
        ] {
            let mut doc = blank();
            let from = [18., 18.];
            let to = [from[0] + size[0], from[1] + size[1]];
            let path = shapes::path(kind, from, to);
            assert!(
                path.points.iter().all(|p| p[0].is_finite()
                    && p[1].is_finite()
                    && p[0] >= 17.99
                    && p[0] <= to[0] + 0.01
                    && p[1] >= 17.99
                    && p[1] <= to[1] + 0.01),
                "{kind:?} {size:?}"
            );
            shapes::paint(
                &mut doc,
                kind,
                from,
                to,
                Style {
                    outline: Some([0, 0, 0, 255]),
                    fill: None,
                    width: 4.,
                    opacity: 0.5,
                },
            )
            .unwrap();
            for y in 0..96 {
                for x in 0..96 {
                    if doc.active().source.get(x, y)[3] > 0 {
                        assert!(
                            (x as f32) >= 15.
                                && (x as f32) <= to[0] + 3.
                                && (y as f32) >= 15.
                                && (y as f32) <= to[1] + 3.,
                            "{kind:?} {size:?} at {x},{y}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn selection_contours_close_around_holes_and_diagonally_touching_islands() {
    let mut mask = editing::rectangle(
        12,
        12,
        editing::PixelRect {
            x: 2,
            y: 2,
            width: 6,
            height: 6,
        },
    );
    let rectangle = editing::contours(&mask);
    assert_eq!(rectangle.len(), 1);
    assert_eq!(rectangle[0].len(), 5);
    for value in std::sync::Arc::make_mut(&mut mask.values) {
        *value = 255 - *value;
    }
    let loops = editing::contours(&mask);
    assert_eq!(loops.len(), 2);
    let areas: Vec<f32> = loops
        .iter()
        .map(|p| {
            p.windows(2)
                .map(|s| s[0][0] * s[1][1] - s[1][0] * s[0][1])
                .sum()
        })
        .collect();
    assert!(areas.iter().any(|a| *a > 0.) && areas.iter().any(|a| *a < 0.));
    let mut islands = editing::rectangle(
        12,
        12,
        editing::PixelRect {
            x: 1,
            y: 1,
            width: 3,
            height: 3,
        },
    );
    let bytes = std::sync::Arc::make_mut(&mut islands.values);
    for y in 4..7 {
        for x in 4..7 {
            bytes[y * 12 + x] = 255;
        }
    }
    let loops = editing::contours(&islands);
    assert_eq!(loops.len(), 2);
    for p in loops {
        assert_eq!(p.first(), p.last());
        assert_eq!(p.len(), 5);
    }
}
