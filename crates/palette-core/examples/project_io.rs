//! Repeatable sparse-document file benchmark, excluding startup/GPU/UI costs.
use palette_core::{brush::Brush, document::Document, io};
use std::{fs, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    for (width, height) in [(1280, 720), (3840, 2160)] {
        let mut doc = Document::new(width, height)?;
        doc.add_layer();
        Brush {
            diameter: 40.,
            opacity: 0.7,
            hardness: 0.5,
            color: [200, 30, 80, 255],
            erase: false,
        }
        .segment(&mut doc.active_mut().source, [100., 100.], [600., 400.]);
        let path = directory.path().join("benchmark.palette");
        let write = Instant::now();
        io::save_project(&path, &doc)?;
        let write_ms = write.elapsed().as_secs_f64() * 1000.;
        let read = Instant::now();
        let reopened = io::open(&path)?;
        let read_ms = read.elapsed().as_secs_f64() * 1000.;
        assert!(reopened == doc);
        println!(
            "{width}x{height}, 2 sparse layers: {} bytes, save {write_ms:.2} ms, open {read_ms:.2} ms",
            fs::metadata(path)?.len()
        );
    }
    Ok(())
}
