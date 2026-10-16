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
    if !cfg!(target_os = "windows") {
        return;
    }
    let baseline = image::open("tests/fixtures/window-windows.png")
        .expect("reviewed baseline")
        .to_rgb8();
    let path = Path::new("target/window-regression.png");
    let _ = std::fs::remove_file(path);
    let mut child = Command::new(env!("CARGO_BIN_EXE_palette"))
        .arg("--screenshot")
        .arg(path)
        .spawn()
        .expect("launch Palette");
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
        "{:.2}% of pixels differ; inspect target/window-regression.png",
        fraction * 100.0
    );
}
