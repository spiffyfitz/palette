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
