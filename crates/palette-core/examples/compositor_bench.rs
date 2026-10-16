//! Small reproducible CPU timing probe; not a GPU or interactive 8K benchmark.
use palette_core::{composite, document::Document, raster::Raster};
use std::{hint::black_box, time::Instant};

fn main() {
    for (width, height) in [(819, 594), (3840, 2160), (7680, 4320)] {
        let mut doc = Document::new(width, height).unwrap();
        doc.add_layer();
        doc.active_mut().source = Raster::new(width, height, [220, 30, 50, 128]).unwrap();
        let mut samples = vec![];
        for _ in 0..3 {
            let start = Instant::now();
            black_box(composite::render(black_box(&doc)));
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "{width}x{height}, 2 visible layers: median {:.2} ms (3 CPU composites)",
            samples[1]
        );
    }
}
