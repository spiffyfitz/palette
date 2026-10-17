//! Generate independently inspectable .paint/native/PNG artifacts without ever
//! rewriting the supplied specimen. Run from the workspace root.
use palette_core::{composite, io};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let input = std::path::Path::new(
        args.first()
            .ok_or("Pass input.paint and a different output.paint")?,
    );
    let output = std::path::Path::new(
        args.get(1)
            .ok_or("Pass input.paint and a different output.paint")?,
    );
    if input == output {
        return Err("Use a new output path".into());
    }
    let doc = io::open(input)?;
    println!(
        "Canvas {} x {}, {} editable layers, background {:?}",
        doc.width,
        doc.height,
        doc.layers.len(),
        doc.background
    );
    for layer in &doc.layers {
        let rgba = layer.source.rgba();
        let transparent = rgba.as_chunks::<4>().0.iter().filter(|p| p[3] == 0).count();
        let partial = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0 && p[3] < 255)
            .count();
        println!(
            "{}: {} x {}, offset {:?}, transparent {transparent}, partial alpha {partial}",
            layer.name,
            layer.source.width(),
            layer.source.height(),
            layer.offset
        );
    }
    io::paint::save_paint(output, &doc)?;
    let reopened = io::open(output)?;
    assert_eq!(doc.layers.len(), reopened.layers.len());
    for (a, b) in doc.layers.iter().zip(&reopened.layers) {
        assert_eq!(a.source.rgba(), b.source.rgba());
        assert_eq!(a.offset, b.offset);
    }
    assert_eq!(composite::render(&doc), composite::render(&reopened));
    io::export(&output.with_extension("png"), &doc)?;
    io::save_project(&output.with_extension("palette"), &doc)?;
    println!("Exact source/composite round trip passed; PNG and native artifacts written.");
    Ok(())
}
