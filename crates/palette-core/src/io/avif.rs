//! Real still AVIF I/O: rav1e encoding and independent rav1d decoding. Preflight
//! the coded AV1 sequence bounds before invoking the full decoder.
use super::*;

fn codec(error: impl std::fmt::Display) -> FileError {
    FileError::Invalid(format!("AVIF: {error}"))
}

/// Encoding quality is independent from speed. Alpha is encoded at quality 100.
#[derive(Clone, Copy, Debug)]
pub struct AvifOptions {
    pub quality: u8,
    pub speed: u8,
}
impl Default for AvifOptions {
    fn default() -> Self {
        Self {
            quality: 80,
            speed: 8,
        }
    }
}

// Limit the scope to still images with one sequence header per coded item.
// A later sequence could advertise larger allocations than the preflight header.
fn one_sequence(mut data: &[u8]) -> Result<(), FileError> {
    let mut sequences = 0;
    while let Some((&header, rest)) = data.split_first() {
        if header & 0x81 != 0 {
            return Err(codec("Invalid OBU header"));
        }
        let kind = (header >> 3) & 15;
        data = rest;
        if header & 4 != 0 {
            data = data
                .get(1..)
                .ok_or_else(|| codec("Truncated OBU extension"))?;
        }
        let length = if header & 2 != 0 {
            let mut length = 0u64;
            let mut complete = false;
            for shift in (0..56).step_by(7) {
                let (&byte, rest) = data
                    .split_first()
                    .ok_or_else(|| codec("Truncated OBU size"))?;
                data = rest;
                length |= u64::from(byte & 127) << shift;
                if byte & 128 == 0 {
                    complete = true;
                    break;
                }
            }
            if !complete {
                return Err(codec("Invalid OBU size"));
            }
            usize::try_from(length).map_err(codec)?
        } else {
            data.len()
        };
        data = data
            .get(length..)
            .ok_or_else(|| codec("Truncated OBU payload"))?;
        if kind == 1 {
            sequences += 1;
        }
    }
    if sequences != 1 {
        return Err(codec("Only a single AV1 still-image sequence is supported"));
    }
    Ok(())
}

// The decoder does not apply HEIF transformative properties. Reject those
// explicitly rather than silently showing an incorrectly oriented/cropped image.
fn check_boxes(mut data: &[u8], depth: usize, count: &mut usize) -> Result<(), FileError> {
    if depth > 8 {
        return Err(codec("Container nesting is too deep"));
    }
    while !data.is_empty() {
        *count += 1;
        if *count > 4096 || data.len() < 8 {
            return Err(codec("Invalid container boxes"));
        }
        let length = u32::from_be_bytes(data[..4].try_into().expect("box length"));
        let kind = &data[4..8];
        let (header, length) = if length == 1 {
            if data.len() < 16 {
                return Err(codec("Truncated extended box"));
            }
            (
                16,
                usize::try_from(u64::from_be_bytes(
                    data[8..16].try_into().expect("extended size"),
                ))
                .map_err(codec)?,
            )
        } else {
            (
                8,
                if length == 0 {
                    data.len()
                } else {
                    length as usize
                },
            )
        };
        if length < header || length > data.len() {
            return Err(codec("Invalid box bounds"));
        }
        let body = &data[header..length];
        match kind {
            b"irot" | b"imir" | b"clap" => {
                return Err(codec(
                    "AVIF rotation/mirroring/crop metadata is not supported yet; export an oriented image first",
                ));
            }
            b"meta" => check_boxes(
                body.get(4..).ok_or_else(|| codec("Truncated meta box"))?,
                depth + 1,
                count,
            )?,
            b"iprp" | b"ipco" => check_boxes(body, depth + 1, count)?,
            _ => {}
        }
        data = &data[length..];
    }
    Ok(())
}

pub(super) fn read(path: &Path) -> Result<Document, FileError> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_PROJECT_BYTES {
        return Err(codec("File exceeds 256 MiB"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(codec("File exceeds 256 MiB"));
    }
    check_boxes(&bytes, 0, &mut 0)?;
    let parsed = avif_parse::read_avif(&mut bytes.as_slice()).map_err(codec)?;
    for (payload, metadata) in std::iter::once((
        parsed.primary_item.as_slice(),
        parsed.primary_item_metadata().map_err(codec)?,
    ))
    .chain(
        parsed
            .alpha_item
            .as_ref()
            .zip(parsed.alpha_item_metadata().map_err(codec)?)
            .map(|(p, m)| (p.as_slice(), m)),
    ) {
        one_sequence(payload)?;
        if !metadata.still_picture {
            return Err(codec(
                "Animated/video AVIF is not supported; import a still frame",
            ));
        }
        let (w, h) = (
            metadata.max_frame_width.get(),
            metadata.max_frame_height.get(),
        );
        if w > 16384 || h > 16384 {
            return Err(codec("Image dimensions exceed 16384 pixels"));
        }
        Raster::new(w, h, [0; 4]).map_err(FileError::Invalid)?;
    }
    drop(parsed);
    let decoded = avif_decode::Decoder::from_avif(&bytes)
        .and_then(|d| d.to_image())
        .map_err(codec)?;
    let (w, h, rgba) = match decoded {
        avif_decode::Image::Rgba8(i) => (
            i.width(),
            i.height(),
            i.pixels()
                .flat_map(|p| [p.r, p.g, p.b, p.a])
                .collect::<Vec<_>>(),
        ),
        avif_decode::Image::Rgb8(i) => (
            i.width(),
            i.height(),
            i.pixels().flat_map(|p| [p.r, p.g, p.b, 255]).collect(),
        ),
        avif_decode::Image::Rgba16(i) => (
            i.width(),
            i.height(),
            i.pixels()
                .flat_map(|p| [p.r, p.g, p.b, p.a].map(|v| ((u32::from(v) + 128) / 257) as u8))
                .collect(),
        ),
        avif_decode::Image::Rgb16(i) => (
            i.width(),
            i.height(),
            i.pixels()
                .flat_map(|p| [p.r, p.g, p.b, u16::MAX].map(|v| ((u32::from(v) + 128) / 257) as u8))
                .collect(),
        ),
        avif_decode::Image::Gray8(i) => (
            i.width(),
            i.height(),
            i.pixels()
                .flat_map(|p| [p.value(), p.value(), p.value(), 255])
                .collect(),
        ),
        avif_decode::Image::Gray16(i) => (
            i.width(),
            i.height(),
            i.pixels()
                .flat_map(|p| {
                    let v = ((u32::from(p.value()) + 128) / 257) as u8;
                    [v, v, v, 255]
                })
                .collect(),
        ),
    };
    Ok(Document::from_raster(
        Raster::from_rgba(w as u32, h as u32, &rgba).map_err(FileError::Invalid)?,
    ))
}

/// Export the rendered composite with explicit lossy RGB quality and speed.
pub fn export_avif(path: &Path, doc: &Document, options: AvifOptions) -> Result<(), FileError> {
    if extension(path) != "avif"
        || !(1..=100).contains(&options.quality)
        || !(1..=10).contains(&options.speed)
    {
        return Err(codec("Use .avif, quality 1–100 and speed 1–10"));
    }
    doc.validate().map_err(FileError::Invalid)?;
    let raster = composite::render(doc);
    let pixels: Vec<_> = raster
        .rgba()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| ravif::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    atomic_write(path, |file| {
        let encoded = ravif::Encoder::new()
            .with_quality(f32::from(options.quality))
            .with_speed(options.speed)
            .with_alpha_quality(100.)
            .with_bit_depth(ravif::BitDepth::Eight)
            .with_num_threads(Some(4))
            .with_alpha_color_mode(ravif::AlphaColorMode::UnassociatedDirty)
            .encode_rgba(ravif::Img::new(
                &pixels,
                raster.width() as usize,
                raster.height() as usize,
            ))
            .map_err(codec)?;
        file.write_all(&encoded.avif_file)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_boxes_and_multiple_sequences_are_rejected_before_decode() {
        assert!(check_boxes(b"\0\0\0\x07ftyp", 0, &mut 0).is_err());
        let rotation = [0, 0, 0, 9, b'i', b'r', b'o', b't', 1];
        let mut property = vec![0, 0, 0, 17, b'i', b'p', b'c', b'o'];
        property.extend_from_slice(&rotation);
        assert!(check_boxes(&property, 0, &mut 0).is_err());
        assert!(one_sequence(&[0x0a, 0, 0x0a, 0]).is_err());
        assert!(one_sequence(&[0x0a, 127, 0]).is_err());
        assert!(one_sequence(&[0x0a, 255, 255, 255, 255, 255, 255, 255, 255]).is_err());
    }
}
