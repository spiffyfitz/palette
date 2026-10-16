//! Small reproducible CPU-only comparison; excludes input, uploads and vsync.
//! cargo run --locked -p palette-core --example render_bench
use palette_core::{composite, document::Document, editing::PixelRect, raster::Raster};
use std::{hint::black_box, time::Instant};

fn main() {
    for (width, height) in [(819, 594), (3840, 2160)] {
        let mut document = Document::new(width, height).unwrap();
        for color in [[220, 80, 50, 140], [60, 130, 230, 100]] {
            document.add_layer();
            document.active_mut().source = Raster::new(width, height, color).unwrap();
        }
        let started = Instant::now();
        black_box(composite::render(&document));
        let full = started.elapsed();
        let region = PixelRect {
            x: 120,
            y: 120,
            width: 128,
            height: 128,
        };
        let started = Instant::now();
        for _ in 0..20 {
            black_box(composite::render_region(&document, region).unwrap());
        }
        let partial = started.elapsed() / 20;
        println!(
            "{width}x{height}, 3 layers: full {:.2} ms; 128x128 region {:.2} ms (mean of 20)",
            full.as_secs_f64() * 1000.0,
            partial.as_secs_f64() * 1000.0
        );
    }
}
