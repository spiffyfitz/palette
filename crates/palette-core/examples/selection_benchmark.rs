//! Representative CPU-only selection timings, not UI latency or memory bounds.
use palette_core::{
    document::Document,
    editing::{self, PixelRect},
    raster::Raster,
    selection::{self, Wand},
};
use std::time::Instant;
fn main() -> Result<(), String> {
    for (w, h) in [(1280, 720), (3840, 2160)] {
        let doc = Document::from_raster(Raster::new(w, h, [30, 100, 200, 255])?);
        let started = Instant::now();
        let selected = selection::wand(&doc, [w / 2, h / 2], Wand::default())?;
        let wand = started.elapsed();
        assert!(selected.values.iter().all(|v| *v == 255));
        let mask = editing::rectangle(
            w,
            h,
            PixelRect {
                x: w / 4,
                y: h / 4,
                width: w / 2,
                height: h / 2,
            },
        );
        let started = Instant::now();
        let feathered = selection::feather(&mask, 12.)?;
        let feather = started.elapsed();
        assert!(feathered.values.iter().any(|v| *v > 0 && *v < 255));
        println!(
            "{w} x {h}: uniform contiguous antialiased Wand {wand:.2?}; 12 px feather {feather:.2?}"
        );
        let started = Instant::now();
        let points: Vec<_> = (0..80)
            .map(|i| {
                [
                    w as f32 * 0.25 + i as f32 * 5.,
                    h as f32 * 0.5 + (i as f32 * 0.2).sin() * 30.,
                ]
            })
            .collect();
        let raw =
            selection::brush::from_points(&doc, &points, selection::brush::Settings::default())?;
        let raw_time = started.elapsed();
        let feathered = selection::feather_region(&raw, 4.)?;
        let contours = editing::contours(&raw);
        let extent = editing::contours_at(&feathered, 1);
        assert!(!contours.is_empty() && !extent.is_empty());
        println!(
            "{w} x {h}: 80-sample edge-aware 40 px brush {raw_time:.2?}; including 4 px feather and both contour sets {:.2?}",
            started.elapsed()
        );
    }
    Ok(())
}
