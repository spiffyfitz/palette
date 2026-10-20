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
#[ignore = "requires an interactive Windows desktop and the reviewed UI scene baselines"]
fn menus_dialogs_text_and_layers_match_reviewed_baselines() {
    for scene in [
        "file-menu",
        "brushes",
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
        "avif-export",
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
        "processing-style-transfer",
        "processing-jpeg-artifacts",
        "processing-colorize",
        "processing-skin-smoothing",
        "processing-smart-portrait",
        "processing-smart-portrait-smile",
        "processing-smart-portrait-frown",
        "processing-makeup",
        "processing-makeup-reference",
        "processing-makeup-spatial",
        "processing-harmonization",
        "processing-photo-restoration",
        "processing-photo-restoration-damage",
        "processing-photo-restoration-boost",
        "processing-landscape",
        "processing-landscape-reference",
        "processing-landscape-reference-winter",
        "processing-landscape-protection",
        "processing-color-transfer",
        "processing-color-transfer-reference",
        "processing-depth-blur",
        "processing-depth-map",
        "processing-super-zoom",
        "processing-super-zoom-options",
        "neural-menu",
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

fn compare_scene(scene: Option<&str>, fixture: &str) {
    if !cfg!(target_os = "windows") {
        return;
    }
    let baseline = image::open(Path::new("tests/fixtures").join(fixture))
        .expect("reviewed baseline")
        .to_rgb8();
    let filename = format!("target/window-regression-{}.png", scene.unwrap_or("main"));
    let path = Path::new(&filename);
    let _ = std::fs::remove_file(path);
    let mut command = Command::new(env!("CARGO_BIN_EXE_palette"));
    command.arg("--screenshot").arg(path);
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
    assert_eq!(actual.dimensions(), (1386, 963));
    assert_eq!(actual.dimensions(), baseline.dimensions());
    let differences = actual
        .pixels()
        .zip(baseline.pixels())
        .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 24))
        .count();
    let fraction = differences as f64 / (1386.0 * 963.0);
    assert!(
        fraction < 0.005,
        "{:.2}% of pixels differ; inspect {}",
        fraction * 100.0,
        path.display()
    );
}
