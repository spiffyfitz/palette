//! Visual reference paired with analytic tests in advanced_layers_selections.rs.
//! Each labeled row has opaque, partial-alpha and transparent-backdrop ramps.
use palette_core::{composite, document::BlendMode};
use std::{fmt::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "target/blend-modes.png".into());
    let mut image = image::RgbaImage::new(768, 19 * 32);
    let mut legend = String::from(
        "Rows are 32 pixels high, top to bottom. Columns: opaque / partial alpha / transparent backdrop.\n\n",
    );
    for (row, mode) in BlendMode::ALL.into_iter().enumerate() {
        writeln!(legend, "{:2}: {}", row + 1, mode.label())?;
        for y in 0..32 {
            for x in 0..768 {
                let t = (x % 256) as u8;
                let (back, front) = match x / 256 {
                    0 => ([t, 90, 170, 255], [230, 70, t, 255]),
                    1 => ([t, 90, 170, 128], [230, 70, t, 128]),
                    _ => ([255, 0, 255, 0], [230, 70, t, t]),
                };
                let checker = if (x / 8 + y / 8) % 2 == 0 { 185 } else { 225 };
                let blended = composite::over(back, front, 1., mode);
                image.put_pixel(
                    x,
                    row as u32 * 32 + y,
                    image::Rgba(composite::over(
                        [checker, checker, checker, 255],
                        blended,
                        1.,
                        BlendMode::Normal,
                    )),
                );
            }
        }
    }
    image.save_with_format(&path, image::ImageFormat::Png)?;
    std::fs::write(path.with_extension("txt"), legend)?;
    println!("Wrote {} and row legend", path.display());
    Ok(())
}
