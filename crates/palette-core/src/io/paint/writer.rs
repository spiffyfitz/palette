//! Rebuild a standard HEIF item graph, never patch the imported container.
//! Source IDs, dimg ordering, offsets and properties are emitted explicitly.
use super::*;
use flate2::{Compression, write::DeflateEncoder};

fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut b = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    b.extend_from_slice(kind);
    b.extend_from_slice(payload);
    b
}
fn full(kind: &[u8; 4], version_flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut b = version_flags.to_be_bytes().to_vec();
    b.extend_from_slice(payload);
    boxed(kind, &b)
}
fn ispe(size: [u32; 2]) -> Vec<u8> {
    full(
        b"ispe",
        0,
        &size
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>(),
    )
}
fn infe(id: u16, kind: &[u8; 4], name: &str) -> Vec<u8> {
    let mut b = id.to_be_bytes().to_vec();
    b.extend_from_slice(&0u16.to_be_bytes());
    b.extend_from_slice(kind);
    // Null is the item-name terminator, not a permitted embedded character.
    b.extend(name.as_bytes().iter().copied().filter(|b| *b != 0));
    b.push(0);
    full(b"infe", 0x02000000 | u32::from(kind == b"Exif"), &b)
}

fn overlay(doc: &Document) -> Vec<u8> {
    let wide = doc.width > u16::MAX as u32
        || doc.height > u16::MAX as u32
        || doc
            .layers
            .iter()
            .flat_map(|l| l.offset)
            .any(|n| i16::try_from(n).is_err());
    let mut b = vec![0, u8::from(wide)];
    for channel in doc.background {
        b.extend_from_slice(&(u16::from(channel) * 257).to_be_bytes());
    }
    for n in [doc.width, doc.height] {
        if wide {
            b.extend_from_slice(&n.to_be_bytes());
        } else {
            b.extend_from_slice(&(n as u16).to_be_bytes());
        }
    }
    for layer in &doc.layers {
        for n in layer.offset {
            if wide {
                b.extend_from_slice(&n.to_be_bytes());
            } else {
                b.extend_from_slice(&(n as i16).to_be_bytes());
            }
        }
    }
    b
}
fn meta(doc: &Document, payloads: &[Vec<u8>], start: u32) -> Vec<u8> {
    let primary = doc.layers.len() as u16 + 1;
    let exif = !doc.metadata.exif.is_empty();
    let overlay = overlay(doc);
    let mut handler = 0u32.to_be_bytes().to_vec();
    handler.extend_from_slice(b"pict");
    handler.extend_from_slice(&[0; 13]);
    let mut children = full(b"hdlr", 0, &handler);
    children.extend_from_slice(&full(b"pitm", 0, &primary.to_be_bytes()));
    let mut info = (primary + u16::from(exif)).to_be_bytes().to_vec();
    for (i, layer) in doc.layers.iter().enumerate() {
        info.extend_from_slice(&infe(i as u16 + 1, b"unci", &layer.name));
    }
    info.extend_from_slice(&infe(primary, b"iovl", ""));
    if exif {
        info.extend_from_slice(&infe(primary + 1, b"Exif", ""));
    }
    children.extend_from_slice(&full(b"iinf", 0, &info));
    let mut location = vec![0x44, 0];
    location.extend_from_slice(&(primary + u16::from(exif)).to_be_bytes());
    let mut offset = start;
    for id in 1..=primary + u16::from(exif) {
        let derived = id == primary;
        let length = if derived {
            overlay.len() as u32
        } else {
            payloads[if id < primary {
                id as usize - 1
            } else {
                doc.layers.len()
            }]
            .len() as u32
        };
        location.extend_from_slice(&id.to_be_bytes());
        location.extend_from_slice(&u16::from(derived).to_be_bytes());
        location.extend_from_slice(&0u16.to_be_bytes());
        location.extend_from_slice(&1u16.to_be_bytes());
        location.extend_from_slice(&(if derived { 0 } else { offset }).to_be_bytes());
        location.extend_from_slice(&length.to_be_bytes());
        if !derived {
            offset += length;
        }
    }
    children.extend_from_slice(&full(b"iloc", 0x01000000, &location));
    // Same verified generic 8-bit BGRA layout as the specimen. Component types
    // are R/G/B/A (4/5/6/7); the pixel stream maps indices 2,1,0,3 to BGRA.
    let mut defs = full(b"cmpC", 0, b"defl\0");
    defs.extend_from_slice(&ispe([doc.width, doc.height]));
    let colr = if doc.metadata.icc_profile.is_empty() {
        b"nclx\0\x01\0\x0d\0\0\x80".to_vec()
    } else {
        let mut b = b"prof".to_vec();
        b.extend_from_slice(&doc.metadata.icc_profile);
        b
    };
    defs.extend_from_slice(&boxed(b"colr", &colr));
    let mut cmpd = 4u32.to_be_bytes().to_vec();
    for kind in 4u16..=7 {
        cmpd.extend_from_slice(&kind.to_be_bytes());
    }
    defs.extend_from_slice(&boxed(b"cmpd", &cmpd));
    let mut unc = b"gene".to_vec();
    unc.extend_from_slice(&4u32.to_be_bytes());
    for index in [2u16, 1, 0, 3] {
        unc.extend_from_slice(&index.to_be_bytes());
        unc.extend_from_slice(&[7, 0, 0]);
    }
    unc.extend_from_slice(&[0, 1, 0, 0]);
    unc.extend_from_slice(&[0; 20]);
    defs.extend_from_slice(&full(b"uncC", 0, &unc));
    defs.extend_from_slice(&full(b"pixi", 0, &[4, 8, 8, 8, 8]));
    let mut sizes = HashMap::from([([doc.width, doc.height], 2u16)]);
    let mut property_count = 6;
    let mut spatial = vec![];
    for layer in &doc.layers {
        let size = [layer.source.width(), layer.source.height()];
        let index = *sizes.entry(size).or_insert_with(|| {
            property_count += 1;
            defs.extend_from_slice(&ispe(size));
            property_count
        });
        spatial.push(index);
    }
    let wide = property_count > 127;
    let mut assoc = u32::from(primary).to_be_bytes().to_vec();
    for id in 1..=primary {
        assoc.extend_from_slice(&id.to_be_bytes());
        let indices = if id == primary {
            vec![(2, false), (6, false)]
        } else {
            vec![
                (1, true),
                (spatial[id as usize - 1], false),
                (3, false),
                (4, true),
                (5, true),
                (6, false),
            ]
        };
        assoc.push(indices.len() as u8);
        for (index, essential) in indices {
            let n = index
                | if essential {
                    if wide { 0x8000 } else { 0x80 }
                } else {
                    0
                };
            if wide {
                assoc.extend_from_slice(&n.to_be_bytes());
            } else {
                assoc.push(n as u8);
            }
        }
    }
    let mut iprp = boxed(b"ipco", &defs);
    iprp.extend_from_slice(&full(b"ipma", u32::from(wide), &assoc));
    children.extend_from_slice(&boxed(b"iprp", &iprp));
    let mut reference = primary.to_be_bytes().to_vec();
    reference.extend_from_slice(&(doc.layers.len() as u16).to_be_bytes());
    for id in 1..primary {
        reference.extend_from_slice(&id.to_be_bytes());
    }
    let mut refs = boxed(b"dimg", &reference);
    if exif {
        let mut r = (primary + 1).to_be_bytes().to_vec();
        r.extend_from_slice(&1u16.to_be_bytes());
        r.extend_from_slice(&primary.to_be_bytes());
        refs.extend_from_slice(&boxed(b"cdsc", &r));
    }
    children.extend_from_slice(&full(b"iref", 0, &refs));
    children.extend_from_slice(&boxed(b"idat", &overlay));
    full(b"meta", 0, &children)
}

/// Atomically write genuine HEIF unci layers with an iovl primary image.
/// This preserves only the verified subset. Unsupported editing state causes an
/// error before touching the destination; callers may explicitly bake a copy.
/// Microsoft Paint application validation remains a separate acceptance gate.
pub fn save_paint(path: &Path, doc: &Document) -> Result<(), FileError> {
    if extension(path) != "paint" {
        return Err(invalid("Use the .paint extension"));
    }
    doc.validate().map_err(FileError::Invalid)?;
    if doc.layers.iter().any(|l| l.name.contains('\0')) {
        return Err(invalid("Layer names cannot contain NUL in this format"));
    }
    let exif = doc.metadata.exif.as_slice();
    if !exif.is_empty() && !(exif.starts_with(b"II\x2a\0") || exif.starts_with(b"MM\0\x2a")) {
        return Err(invalid("Exif must contain an original TIFF header"));
    }
    let reasons = limitations(doc);
    if !reasons.is_empty() {
        return Err(invalid(&format!(
            "Preserve the document as .palette first. Unsupported editable state: {}",
            reasons.join(", ")
        )));
    }
    if doc
        .layers
        .iter()
        .any(|l| l.source.width() > 16384 || l.source.height() > 16384)
        || doc.width > 16384
        || doc.height > 16384
    {
        return Err(invalid("Dimensions exceed the verified 16384-pixel limit"));
    }
    let total: u64 = doc
        .layers
        .iter()
        .map(|l| u64::from(l.source.width()) * u64::from(l.source.height()) * 4)
        .sum();
    if total > MAX_DECODED {
        return Err(invalid("Layer data exceeds 512 MiB"));
    }
    let mut payloads = vec![];
    let mut compressed_total = 0usize;
    for layer in &doc.layers {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
        let mut row = Vec::with_capacity(layer.source.width() as usize * 4);
        for y in 0..layer.source.height() {
            row.clear();
            for x in 0..layer.source.width() {
                let [r, g, b, a] = layer.source.get(x, y);
                row.extend_from_slice(&[b, g, r, a]);
            }
            encoder.write_all(&row)?;
            if encoder.get_ref().len() + compressed_total > MAX_FILE {
                return Err(invalid("Encoded project exceeds 256 MiB"));
            }
        }
        let payload = encoder.finish()?;
        compressed_total += payload.len();
        payloads.push(payload);
    }
    if !doc.metadata.exif.is_empty() {
        let mut exif = vec![0; 4];
        exif.extend_from_slice(&doc.metadata.exif);
        payloads.push(exif);
    }
    let ftyp = boxed(b"ftyp", b"mif1\0\0\0\0mif1gcmiisoamiaf");
    let empty_meta = meta(doc, &payloads, 0);
    let start = ftyp.len() + empty_meta.len() + 64 + 8;
    let metadata = meta(doc, &payloads, start as u32);
    let size: usize = payloads.iter().map(Vec::len).sum();
    if start + size > MAX_FILE {
        return Err(invalid("Encoded project exceeds 256 MiB"));
    }
    atomic_write(path, |file| {
        file.write_all(&ftyp)?;
        file.write_all(&metadata)?;
        file.write_all(&boxed(b"free", &[0; 56]))?;
        file.write_all(&((size + 8) as u32).to_be_bytes())?;
        file.write_all(b"mdat")?;
        for payload in payloads {
            file.write_all(&payload)?;
        }
        Ok(())
    })
}
