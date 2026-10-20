//! Genuine HEIF/iovl Microsoft Paint projects, based on the supplied specimen.
//! This independent, safe Rust adapter deliberately supports a bounded subset:
//! direct 8-bit RGBA unci overlays with whole-item raw Deflate/zlib compression.
//! Other codecs/layouts/essential properties fail rather than flattening silently.
mod boxes;
mod writer;
use super::*;
use crate::document::BlendMode;
use boxes::*;
use flate2::{Decompress, FlushDecompress, Status};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
pub use writer::save_paint;
const MAX_FILE: usize = 256 * 1024 * 1024;
const MAX_DECODED: u64 = 512 * 1024 * 1024;
fn invalid(message: &str) -> FileError {
    FileError::Invalid(format!("Microsoft Paint project: {message}"))
}

struct Item {
    kind: [u8; 4],
    name: String,
}
fn items(data: &[u8], budget: &mut usize) -> Result<HashMap<u32, Item>, FileError> {
    let mut c = Cursor::new(data);
    let (version, _) = c.full(1)?;
    let count = c.id(version == 1)?;
    if count == 0 || count > 1024 {
        return Err(invalid("Invalid item count"));
    }
    let entries = boxes(&data[c.position..], budget)?;
    if entries.len() != count as usize {
        return Err(invalid("Item count mismatch"));
    }
    let mut result = HashMap::new();
    for entry in entries {
        if entry.kind != *b"infe" {
            return Err(invalid("Unsupported item information"));
        }
        let mut c = Cursor::new(entry.data);
        let (version, _) = c.full(3)?;
        if version < 2 {
            return Err(invalid("Legacy item information is unsupported"));
        }
        let id = c.id(version == 3)?;
        if c.u16()? != 0 {
            return Err(invalid("Protected/encrypted items are unsupported"));
        }
        let kind = c.array()?;
        let name = &c.data[c.position..];
        let end = name
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| invalid("Unterminated item name"))?;
        if end > 4096 {
            return Err(invalid("Item name is too long"));
        }
        if result
            .insert(
                id,
                Item {
                    kind,
                    name: String::from_utf8_lossy(&name[..end]).into_owned(),
                },
            )
            .is_some()
        {
            return Err(invalid("Duplicate item ID"));
        }
    }
    Ok(result)
}
type Properties<'a> = HashMap<u32, Vec<(BoxData<'a>, bool)>>;
fn properties<'a>(data: &'a [u8], budget: &mut usize) -> Result<Properties<'a>, FileError> {
    let children = boxes(data, budget)?;
    let definitions = boxes(find(&children, b"ipco")?, budget)?;
    let mut c = Cursor::new(find(&children, b"ipma")?);
    let (version, flags) = c.full(1)?;
    if flags > 1 {
        return Err(invalid("Unsupported property association flags"));
    }
    let count = c.u32()?;
    if count > 1024 {
        return Err(invalid("Too many property associations"));
    }
    let mut out = HashMap::new();
    for _ in 0..count {
        let id = c.id(version == 1)?;
        let count = c.u8()?;
        let mut props = vec![];
        let mut kinds = HashSet::new();
        for _ in 0..count {
            let value = if flags == 1 {
                c.u16()?
            } else {
                u16::from(c.u8()?)
            };
            let bit = if flags == 1 { 0x8000 } else { 0x80 };
            let index = usize::from(value & !bit);
            if index == 0 {
                continue;
            }
            let property = definitions
                .get(index - 1)
                .ok_or_else(|| invalid("Invalid property index"))?
                .clone();
            if !kinds.insert(property.kind) && understood(&property.kind) {
                return Err(invalid("Ambiguous duplicate property"));
            }
            props.push((property, value & bit != 0));
        }
        if out.insert(id, props).is_some() {
            return Err(invalid("Duplicate item properties"));
        }
    }
    c.done()?;
    Ok(out)
}
fn understood(kind: &[u8; 4]) -> bool {
    matches!(
        kind,
        b"cmpC" | b"ispe" | b"colr" | b"cmpd" | b"uncC" | b"pixi"
    )
}
fn property<'a>(
    properties: &'a [(BoxData<'a>, bool)],
    kind: &[u8; 4],
) -> Result<&'a [u8], FileError> {
    properties
        .iter()
        .find(|(p, _)| &p.kind == kind)
        .map(|(p, _)| p.data)
        .ok_or_else(|| {
            invalid(&format!(
                "Missing {} property",
                String::from_utf8_lossy(kind)
            ))
        })
}
fn dimensions(properties: &[(BoxData<'_>, bool)]) -> Result<[u32; 2], FileError> {
    // Unknown optional properties and unused items may be retained by future
    // Paint versions. Only essential properties on the rendered graph matter.
    for (p, essential) in properties {
        if *essential && !understood(&p.kind) {
            return Err(invalid(&format!(
                "Unsupported essential property {}",
                String::from_utf8_lossy(&p.kind)
            )));
        }
        if matches!(&p.kind, b"irot" | b"imir" | b"clap") {
            return Err(invalid("Transformed source items are not supported yet"));
        }
    }
    let mut c = Cursor::new(property(properties, b"ispe")?);
    c.full(0)?;
    let size = [c.u32()?, c.u32()?];
    c.done()?;
    if size.iter().any(|n| *n > 16384) {
        return Err(invalid("Dimensions exceed 16384 pixels"));
    }
    Raster::new(size[0], size[1], [0; 4]).map_err(FileError::Invalid)?;
    Ok(size)
}
fn channel_order(properties: &[(BoxData<'_>, bool)]) -> Result<[usize; 4], FileError> {
    let mut definitions = Cursor::new(property(properties, b"cmpd")?);
    if definitions.u32()? != 4 {
        return Err(invalid("Only four-component RGBA is supported"));
    }
    let types = [
        definitions.u16()?,
        definitions.u16()?,
        definitions.u16()?,
        definitions.u16()?,
    ];
    definitions.done()?;
    let mut c = Cursor::new(property(properties, b"uncC")?);
    c.full(0)?;
    let profile = c.array::<4>()?;
    if profile != *b"gene" && profile != [0; 4] {
        return Err(invalid("Unsupported uncompressed profile"));
    }
    if c.u32()? != 4 {
        return Err(invalid("Unsupported component count"));
    }
    let mut order = [0; 4];
    let mut channels = HashSet::new();
    for position in 0..4 {
        let index = c.u16()? as usize;
        let component = *types
            .get(index)
            .ok_or_else(|| invalid("Invalid component index"))?;
        if !(4..=7).contains(&component)
            || !channels.insert(component)
            || c.u8()? != 7
            || c.u8()? != 0
            || c.u8()? > 1
        {
            return Err(invalid("Only unsigned 8-bit RGBA components are supported"));
        }
        order[(component - 4) as usize] = position;
    }
    if c.take(4)? != [0, 1, 0, 0] || c.take(20)?.iter().any(|b| *b != 0) {
        return Err(invalid(
            "Only packed pixel-interleaved, untiled RGBA is supported",
        ));
    }
    c.done()?;
    if let Ok(pixi) = property(properties, b"pixi") {
        let mut p = Cursor::new(pixi);
        p.full(0)?;
        if p.take(5)? != [4, 8, 8, 8, 8] {
            return Err(invalid("Inconsistent channel depths"));
        }
        p.done()?;
    }
    Ok(order)
}
fn color(properties: &[(BoxData<'_>, bool)]) -> Result<Vec<u8>, FileError> {
    let Ok(data) = property(properties, b"colr") else {
        return Ok(vec![]);
    };
    if data.starts_with(b"prof") || data.starts_with(b"rICC") {
        if data.len() > 4 * 1024 * 1024 + 4 {
            return Err(invalid("ICC profile is too large"));
        }
        return Ok(data[4..].to_vec());
    }
    if data != b"nclx\0\x01\0\x0d\0\0\x80" {
        return Err(invalid(
            "Only sRGB/full-range RGB or retained ICC profiles are supported",
        ));
    }
    Ok(vec![])
}

/// Import each direct overlay source as an independently editable raster layer.
pub(super) fn read(path: &Path) -> Result<Document, FileError> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_FILE as u64 {
        return Err(invalid("Input exceeds 256 MiB"));
    }
    let mut bytes = vec![];
    file.take(MAX_FILE as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FILE {
        return Err(invalid("Input exceeds 256 MiB"));
    }
    let mut budget = 4096;
    let root = boxes(&bytes, &mut budget)?;
    let brands = find(&root, b"ftyp")?;
    if brands.len() < 8
        || brands.len() % 4 != 0
        || !(brands[..4] == *b"mif1" || brands[8..].as_chunks::<4>().0.iter().any(|b| b == b"mif1"))
    {
        return Err(invalid("Not a supported HEIF/mif1 project"));
    }
    let mut meta = Cursor::new(find(&root, b"meta")?);
    meta.full(0)?;
    let meta = boxes(&meta.data[meta.position..], &mut budget)?;
    let info = items(find(&meta, b"iinf")?, &mut budget)?;
    let locations = locations(find(&meta, b"iloc")?)?;
    let idat = meta
        .iter()
        .find(|b| b.kind == *b"idat")
        .map_or(&[][..], |b| b.data);
    let props = properties(find(&meta, b"iprp")?, &mut budget)?;
    let mut primary = Cursor::new(find(&meta, b"pitm")?);
    let (version, _) = primary.full(1)?;
    let primary_id = primary.id(version == 1)?;
    primary.done()?;
    let primary = primary_id;
    if info.get(&primary).is_none_or(|i| i.kind != *b"iovl") {
        return Err(invalid("Primary image must be a layered iovl overlay"));
    }
    let mut references = Cursor::new(find(&meta, b"iref")?);
    let (version, _) = references.full(1)?;
    let refs = boxes(&references.data[references.position..], &mut budget)?;
    let mut sources = None;
    let mut exif_id = None;
    for r in refs {
        let mut c = Cursor::new(r.data);
        let from = c.id(version == 1)?;
        let count = c.u16()?;
        if count > 256 {
            return Err(invalid("Too many item references"));
        }
        let to = (0..count)
            .map(|_| c.id(version == 1))
            .collect::<Result<Vec<_>, _>>()?;
        c.done()?;
        if r.kind == *b"dimg" && from == primary && sources.replace(to.clone()).is_some() {
            return Err(invalid("Duplicate overlay references"));
        }
        if r.kind == *b"cdsc"
            && to.contains(&primary)
            && info.get(&from).is_some_and(|i| i.kind == *b"Exif")
            && exif_id.replace(from).is_some()
        {
            return Err(invalid("Multiple Exif items are unsupported"));
        }
    }
    let sources = sources
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid("Missing overlay source layers"))?;
    let read_item = |id| {
        locations
            .get(&id)
            .ok_or_else(|| invalid("Missing item location"))?
            .read(&bytes, idat)
    };
    let overlay = read_item(primary)?;
    let mut c = Cursor::new(&overlay);
    if c.u8()? != 0 {
        return Err(invalid("Unsupported overlay version"));
    }
    let flags = c.u8()?;
    if flags > 1 {
        return Err(invalid("Unsupported overlay flags"));
    }
    let mut background = [0; 4];
    for channel in &mut background {
        *channel = ((u32::from(c.u16()?) + 128) / 257) as u8;
    }
    let wide = flags == 1;
    let width = c.id(wide)?;
    let height = c.id(wide)?;
    let canvas = dimensions(
        props
            .get(&primary)
            .ok_or_else(|| invalid("Missing overlay properties"))?,
    )?;
    if canvas != [width, height] {
        return Err(invalid("Overlay dimensions disagree with ispe"));
    }
    let mut plan = vec![];
    let mut total = 0u64;
    for id in sources {
        if info.get(&id).is_none_or(|i| i.kind != *b"unci") {
            return Err(invalid(
                "Only direct unci overlay inputs are supported (no cycles or nested derived images)",
            ));
        }
        let x = if wide {
            c.u32()? as i32
        } else {
            c.u16()? as i16 as i32
        };
        let y = if wide {
            c.u32()? as i32
        } else {
            c.u16()? as i16 as i32
        };
        let p = props
            .get(&id)
            .ok_or_else(|| invalid("Missing layer properties"))?;
        let size = dimensions(p)?;
        total += u64::from(size[0]) * u64::from(size[1]) * 4;
        if total > MAX_DECODED {
            return Err(invalid("Decoded layers exceed 512 MiB"));
        }
        plan.push((id, [x, y], size, channel_order(p)?, color(p)?));
    }
    c.done()?;
    let mut doc = Document::new(width, height).map_err(FileError::Invalid)?;
    doc.layers.clear();
    doc.background = background;
    let mut profile = None;
    for (id, offset, size, order, icc) in plan {
        if profile.as_ref().is_some_and(|p| *p != icc) {
            return Err(invalid("Mixed layer color profiles are unsupported"));
        }
        profile = Some(icc);
        let expected = u64::from(size[0]) * u64::from(size[1]) * 4;
        let data = read_item(id)?;
        let p = &props[&id];
        let mut raw = vec![];
        if let Ok(config) = property(p, b"cmpC") {
            let mut config = Cursor::new(config);
            config.full(0)?;
            let codec = config.array::<4>()?;
            if config.u8()? != 0 {
                return Err(invalid("Only whole-image compression is supported"));
            }
            config.done()?;
            if !matches!(&codec, b"defl" | b"zlib") {
                return Err(invalid("Unsupported generic compression method"));
            }
            raw.resize(expected as usize + 1, 0);
            let mut decoder = Decompress::new(codec == *b"zlib");
            // Require the actual end marker, not merely enough decoded pixels:
            // a truncated Deflate stream can otherwise look like a valid layer.
            let status = decoder
                .decompress(&data, &mut raw, FlushDecompress::Finish)
                .map_err(|_| invalid("Invalid compressed pixels"))?;
            if status != Status::StreamEnd
                || decoder.total_in() != data.len() as u64
                || decoder.total_out() != expected
            {
                return Err(invalid(
                    "Incomplete, oversized or trailing compressed pixels",
                ));
            }
            raw.truncate(expected as usize);
        } else {
            raw = data;
        }
        if raw.len() as u64 != expected {
            return Err(invalid("Uncompressed pixel length mismatch"));
        }
        for pixel in raw.as_chunks_mut::<4>().0 {
            let original = *pixel;
            for c in 0..4 {
                pixel[c] = original[order[c]];
            }
        }
        let source = Raster::from_rgba(size[0], size[1], &raw).map_err(FileError::Invalid)?;
        let mut layer = Document::from_raster(source).layers.remove(0);
        layer.id = doc.layers.len() as u64 + 1;
        layer.offset = offset;
        layer.name = if info[&id].name.is_empty() {
            format!("Layer {}", layer.id)
        } else {
            info[&id].name.clone()
        };
        doc.layers.push(layer);
    }
    doc.active_layer = doc.layers.last().expect("nonempty sources").id;
    doc.next_id = doc.active_layer + 1;
    doc.metadata.icc_profile = Arc::new(profile.unwrap_or_default());
    if let Some(id) = exif_id {
        let bytes = read_item(id)?;
        let mut c = Cursor::new(&bytes);
        let offset = c.u32()? as usize;
        let tiff = bytes
            .get(
                4usize
                    .checked_add(offset)
                    .ok_or_else(|| invalid("Exif offset overflow"))?..,
            )
            .ok_or_else(|| invalid("Exif offset outside item"))?;
        if !(tiff.starts_with(b"II\x2a\0") || tiff.starts_with(b"MM\0\x2a")) {
            return Err(invalid("Invalid Exif TIFF header"));
        }
        doc.metadata.exif = Arc::new(tiff.to_vec());
    }
    doc.validate().map_err(FileError::Invalid)?;
    Ok(doc)
}

/// Reasons a lossless editable export cannot use the verified Paint subset.
/// Callers must preserve the original as .palette before explicit baking.
pub fn limitations(doc: &Document) -> Vec<&'static str> {
    let mut reasons = vec![];
    if !doc.background_visible {
        reasons.push("Hidden canvas background color");
    }
    if doc.layers.iter().any(|l| !l.visible || l.locked) {
        reasons.push("Layer visibility and locks");
    }
    if doc
        .layers
        .iter()
        .any(|l| l.blend != BlendMode::Normal || l.opacity != 1.)
    {
        reasons.push("Blend modes and layer opacity");
    }
    if doc
        .layers
        .iter()
        .any(|l| l.mask.is_some() || !l.filters.is_empty() || !l.effects.is_empty())
    {
        reasons.push("Masks, live filters and layer styles");
    }
    reasons
}

/// Explicit compatibility conversion, used only after the full .palette project
/// is preserved. Bake local processing into separate raster layers. Non-Normal
/// blends require the backdrop: merge only the prefix through the last such
/// layer, retaining all later independent layers rather than flattening all.
pub fn compatible_copy(doc: &Document) -> Result<Document, FileError> {
    doc.validate().map_err(FileError::Invalid)?;
    let mut out = doc.clone();
    // The verified Paint subset stores an effective backing RGBA, without a
    // separate retained hidden color. The original is preserved as .palette.
    out.background = doc.backing_color();
    out.background_visible = true;
    if let Some(last) = doc
        .layers
        .iter()
        .rposition(|l| l.blend != BlendMode::Normal)
    {
        let mut prefix = doc.clone();
        for l in prefix.layers.iter_mut().skip(last + 1) {
            l.visible = false;
        }
        prefix.active_layer = prefix.layers[last].id;
        let mut merged = Document::from_raster(composite::render(&prefix))
            .layers
            .remove(0);
        merged.id = out.allocate_id();
        merged.name = "Composited blend layers".into();
        out.layers.drain(..=last);
        out.layers.insert(0, merged);
        out.background = [0; 4];
    }
    for layer in &mut out.layers {
        if crate::processing::has_neighborhood(layer) {
            // Spatial styles may extend past source bounds. Bake the complete
            // canvas-space appearance into this independent compatible layer.
            let mut single = doc.clone();
            single.background = [0; 4];
            single.selection = None;
            single.selection_base = None;
            single.layers = doc
                .layers
                .iter()
                .map(|l| {
                    let mut l = if l.id == layer.id {
                        layer.clone()
                    } else {
                        l.clone()
                    };
                    if l.id != layer.id {
                        l.visible = false;
                    }
                    l
                })
                .collect();
            single.active_layer = layer.id;
            layer.source = composite::render(&single);
            layer.offset = [0, 0];
            layer.visible = true;
            layer.locked = false;
            layer.opacity = 1.;
            layer.blend = BlendMode::Normal;
            layer.mask = None;
            layer.filters.clear();
            layer.effects.clear();
            continue;
        }
        let mut pixels = Raster::new(layer.source.width(), layer.source.height(), [0; 4])
            .map_err(FileError::Invalid)?;
        if layer.visible {
            for y in 0..pixels.height() {
                for x in 0..pixels.width() {
                    let mut pixel = composite::layer_pixel(layer, x, y);
                    pixel[3] = (f32::from(pixel[3]) * layer.opacity).round() as u8;
                    pixels.set(x, y, pixel);
                }
            }
        }
        layer.source = pixels;
        layer.visible = true;
        layer.locked = false;
        layer.opacity = 1.;
        layer.blend = BlendMode::Normal;
        layer.mask = None;
        layer.filters.clear();
        layer.effects.clear();
    }
    out.selection = None;
    out.selection_base = None;
    out.active_layer = out.layers.last().expect("validated layers").id;
    out.validate().map_err(FileError::Invalid)?;
    Ok(out)
}
