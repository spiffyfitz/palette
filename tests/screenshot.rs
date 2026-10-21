//! Desktop screenshot regression. Requires an interactive Windows desktop.
//! cargo test --test screenshot -- --ignored
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires an interactive Windows desktop and the committed Windows baseline"]
fn main_window_matches_reviewed_baseline() {
    compare_scene(None, "window-windows.png");
}

#[test]
#[ignore = "requires an interactive Windows desktop and reviewed brush material baselines"]
fn brush_materials_match_reviewed_baselines() {
    // A focused check for material-only changes; the full UI suite also keeps
    // these scenes so its broader coverage remains intact.
    compare_scene(Some("brushes"), "ui-brushes-windows.png");
    compare_scene(Some("brush-styles"), "ui-brush-styles-windows.png");
    compare_scene_size(Some("brushes"), "ribbon-brushes-windows.png", (720, 520));
}

#[test]
#[ignore = "requires an interactive Windows desktop and the reviewed UI scene baselines"]
fn menus_dialogs_text_and_layers_match_reviewed_baselines() {
    for scene in [
        "tabs-overflow",
        "file-menu",
        "brushes",
        "brush-styles",
        "brush-size-preview",
        "colors",
        "text",
        "properties",
        "layers",
        "shapes",
        "selection",
        "catalog",
        "rotate-custom",
        "selected-shape",
        "polygon-preview",
        "curve-preview",
        "selection-resize-preview",
        "selection-rotate-preview",
        "recovery",
        "close-dialog",
        "paint-project",
        "paint-export",
        "selection-options",
        "wand-settings",
        "mask-layer",
        "feather-selection",
        "rectangle-toolbar",
        "wand-toolbar",
        "freeform-toolbar",
        "freeform-preview",
        "feather-outline",
        "inverted-selection",
        "canvas-scrollbars",
        "brush-selection",
        "brush-selection-options",
        "processing-blur",
        "processing-shadow",
        "processing-glow",
        "processing-stack",
        "processing-effects-stack",
        "layer-blending",
        "filter-menu",
        "adjustments-menu",
        "blur-menu",
        "noise-menu",
        "pixelate-menu",
        "other-menu",
        "distort-menu",
        "sharpen-menu",
        "stylize-menu",
        "artistic-menu",
        "brush-strokes-menu",
        "sketch-menu",
        "render-menu",
        "blur-gallery-menu",
        "video-menu",
        "texture-menu",
        "processing-hue",
        "processing-stroke",
        "processing-inner-shadow",
        "processing-inner-glow",
        "processing-levels",
        "processing-curves",
        "processing-motion",
        "processing-noise",
        "processing-mosaic",
        "processing-high-pass",
        "processing-edges",
        "processing-emboss",
        "processing-unsharp",
        "processing-offset",
        "processing-twirl",
        "processing-wave",
        "processing-surface",
        "processing-radial",
        "processing-clouds",
        "processing-difference-clouds",
        "processing-poster-edges",
        "processing-grain",
        "processing-fragment",
        "processing-despeckle",
        "processing-dust",
        "processing-reduce-noise",
        "processing-pinch",
        "processing-spherize",
        "processing-ripple",
        "processing-sharpen",
        "processing-sharpen-more",
        "processing-polar",
        "processing-zigzag",
        "processing-fibers",
        "processing-glowing-edges",
        "processing-origin",
        "processing-cutout",
        "processing-film-grain",
        "processing-accented-edges",
        "processing-ink-outlines",
        "processing-photocopy",
        "processing-halftone",
        "processing-patchwork",
        "processing-craquelure",
        "processing-color-halftone",
        "processing-crystallize",
        "processing-facet",
        "processing-mezzotint",
        "processing-pointillize",
        "processing-stained-glass",
        "processing-mosaic-tiles",
        "processing-diffuse",
        "processing-color-balance",
        "processing-photo-filter",
        "processing-channel-mixer",
        "processing-gradient-map",
        "processing-selective-color",
        "processing-shadows-highlights",
        "processing-fixed-blur",
        "processing-blur-more",
        "processing-colored-pencil",
        "processing-dry-brush",
        "processing-paint-daubs",
        "processing-smudge-stick",
        "processing-angled-strokes",
        "processing-crosshatch",
        "processing-dark-strokes",
        "processing-spatter",
        "processing-sprayed-strokes",
        "processing-sumi-e",
        "processing-bas-relief",
        "processing-chalk-charcoal",
        "processing-charcoal",
        "processing-graphic-pen",
        "processing-note-paper",
        "processing-stamp",
        "processing-chrome",
        "processing-conte-crayon",
        "processing-plaster",
        "processing-reticulation",
        "processing-torn-edges",
        "processing-water-paper",
        "processing-fresco",
        "processing-neon-glow",
        "processing-palette-knife",
        "processing-plastic-wrap",
        "processing-rough-pastels",
        "processing-sponge",
        "processing-underpainting",
        "processing-watercolor",
        "processing-diffuse-glow",
        "processing-glass",
        "processing-ocean-ripple",
        "processing-texturizer",
        "processing-shape-blur",
        "processing-smart-blur",
        "processing-sharpen-edges",
        "processing-smart-sharpen",
        "processing-extrude",
        "processing-oil-paint",
        "processing-tiles",
        "processing-trace-contour",
        "processing-wind",
        "processing-lens-blur",
        "processing-lens-flare",
        "processing-lighting-effects",
        "processing-color-lookup",
        "processing-displace",
        "processing-shear",
        "processing-custom",
        "processing-custom-kernel",
        "processing-field-blur",
        "processing-iris-blur",
        "processing-tilt-shift",
        "processing-path-blur",
        "processing-spin-blur",
        "processing-flame",
        "processing-picture-frame",
        "processing-tree",
        "processing-ntsc",
        "processing-deinterlace",
        "processing-hsb-hsl",
        "processing-shear-curve",
        "processing-bevel",
        "processing-satin",
        "processing-gradient-overlay",
        "processing-pattern-overlay",
        "layer-style-stack",
        "layer-multi-select",
        "merge-layers-dialog",
        "merge-layers-result",
        "canvas-background",
        "canvas-background-hidden",
        "canvas-background-colors",
        "style-gradient-editor",
        "style-gloss-editor",
        "style-contour-editor",
        "style-glow-settings",
        "style-shadow-settings",
        "style-imported-pattern",
        "style-gradient-stroke",
    ] {
        compare_scene(Some(scene), &format!("ui-{scene}-windows.png"));
    }
}

#[test]
#[ignore = "requires an interactive Windows desktop and the reviewed responsive ribbon baselines"]
fn responsive_ribbon_matches_reviewed_baselines() {
    for (name, width, scene) in [
        ("compact", 1230, None),
        ("shapes-folded", 1100, None),
        ("image-folded", 1000, None),
        ("tools-folded", 940, None),
        ("colors-folded", 780, None),
        ("minimum", 720, None),
        ("image", 720, Some("ribbon-image")),
        ("tools", 720, Some("ribbon-tools")),
        ("shapes", 720, Some("ribbon-shapes")),
        ("colors", 720, Some("ribbon-colors")),
        ("brushes", 720, Some("brushes")),
    ] {
        compare_scene_size(scene, &format!("ribbon-{name}-windows.png"), (width, 520));
    }
}

fn compare_scene(scene: Option<&str>, fixture: &str) {
    compare_scene_size(scene, fixture, (1386, 963));
}

fn compare_scene_size(scene: Option<&str>, fixture: &str, size: (u32, u32)) {
    if !cfg!(target_os = "windows") {
        return;
    }
    let baseline = image::open(Path::new("tests/fixtures").join(fixture))
        .expect("reviewed baseline")
        .to_rgb8();
    let filename = format!("target/window-regression-{fixture}");
    let path = Path::new(&filename);
    let _ = std::fs::remove_file(path);
    let mut command = Command::new(env!("CARGO_BIN_EXE_palette"));
    command.arg("--screenshot").arg(path);
    command
        .arg("--screenshot-size")
        .arg(format!("{}x{}", size.0, size.1));
    if let Some(scene) = scene {
        command.arg("--screenshot-scene").arg(scene);
    }
    let mut child = command.spawn().expect("launch Palette");
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("poll Palette") {
            assert!(status.success(), "Palette exited with {status}");
            break;
        }
        if started.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            panic!("Screenshot timed out");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let actual = image::open(path).expect("captured screenshot").to_rgb8();
    assert_eq!(actual.dimensions(), size);
    assert_eq!(actual.dimensions(), baseline.dimensions());
    let differences = actual
        .pixels()
        .zip(baseline.pixels())
        .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 24))
        .count();
    let fraction = differences as f64 / (f64::from(size.0) * f64::from(size.1));
    assert!(
        fraction < 0.005,
        "{:.2}% of pixels differ; inspect {}",
        fraction * 100.0,
        path.display()
    );
}
