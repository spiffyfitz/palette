//! Versions 2–4: bounded JSON descriptors followed by independently compressed
//! binary assets. Shared COW tiles are stored once; no pixel-sized JSON buffer.
use super::*;
use crate::{
    document::*,
    raster::{Coverage, TILE},
};
use flate2::{Compression, write::ZlibEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Seek, SeekFrom},
    sync::Arc,
};

pub(super) const MAX_FILE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
const TILE_BYTES: usize = (TILE * TILE * 4) as usize;
fn invalid(message: &str) -> FileError {
    FileError::Invalid(message.into())
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    width: u32,
    height: u32,
    #[serde(default)]
    background: [u8; 4],
    active_layer: u64,
    next_id: u64,
    layers: Vec<LayerRecord>,
    selection: Option<MaskRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selection_base: Option<MaskRecord>,
    icc: usize,
    exif: usize,
    asset_lengths: Vec<usize>,
}
#[derive(Serialize, Deserialize)]
struct RasterRecord {
    width: u32,
    height: u32,
    tiles: Vec<usize>,
}
#[derive(Serialize, Deserialize)]
struct MaskRecord {
    width: u32,
    height: u32,
    enabled: bool,
    asset: usize,
}
#[derive(Serialize, Deserialize)]
struct FilterRecord {
    id: u64,
    enabled: bool,
    filter: Filter,
    mask: Option<MaskRecord>,
}
#[derive(Serialize, Deserialize)]
struct LayerRecord {
    id: u64,
    name: String,
    source: RasterRecord,
    visible: bool,
    locked: bool,
    opacity: f32,
    blend: BlendMode,
    offset: [i32; 2],
    mask: Option<MaskRecord>,
    filters: Vec<FilterRecord>,
    effects: Vec<EffectInstance>,
}
#[derive(Default)]
struct Assets {
    ids: HashMap<usize, usize>,
    data: Vec<Arc<Vec<u8>>>,
}
impl Assets {
    fn add(&mut self, bytes: &Arc<Vec<u8>>) -> usize {
        let pointer = Arc::as_ptr(bytes) as usize;
        *self.ids.entry(pointer).or_insert_with(|| {
            let id = self.data.len();
            self.data.push(bytes.clone());
            id
        })
    }
    fn mask(&mut self, m: &Coverage) -> MaskRecord {
        MaskRecord {
            width: m.width,
            height: m.height,
            enabled: m.enabled,
            asset: self.add(&m.values),
        }
    }
}

pub(super) fn write(file: &mut File, doc: &Document) -> Result<(), FileError> {
    let mut assets = Assets::default();
    let icc = assets.add(&doc.metadata.icc_profile);
    let exif = assets.add(&doc.metadata.exif);
    let layers = doc
        .layers
        .iter()
        .map(|l| LayerRecord {
            id: l.id,
            name: l.name.clone(),
            source: RasterRecord {
                width: l.source.width(),
                height: l.source.height(),
                tiles: l.source.tiles().iter().map(|t| assets.add(t)).collect(),
            },
            visible: l.visible,
            locked: l.locked,
            opacity: l.opacity,
            blend: l.blend,
            offset: l.offset,
            mask: l.mask.as_ref().map(|m| assets.mask(m)),
            filters: l
                .filters
                .iter()
                .map(|f| FilterRecord {
                    id: f.id,
                    enabled: f.enabled,
                    filter: f.filter.clone(),
                    mask: f.mask.as_ref().map(|m| assets.mask(m)),
                })
                .collect(),
            effects: l.effects.clone(),
        })
        .collect();
    let manifest = Manifest {
        width: doc.width,
        height: doc.height,
        active_layer: doc.active_layer,
        background: doc.background,
        next_id: doc.next_id,
        layers,
        selection: doc.selection.as_ref().map(|m| assets.mask(m)),
        selection_base: doc.selection_base.as_ref().map(|m| assets.mask(m)),
        icc,
        exif,
        asset_lengths: assets.data.iter().map(|a| a.len()).collect(),
    };
    manifest.validate()?;
    let json = serde_json::to_vec(&manifest)?;
    if json.len() > MAX_MANIFEST_BYTES {
        return Err(invalid("Project metadata exceeds 4 MiB"));
    }
    file.write_all(&(json.len() as u32).to_le_bytes())?;
    file.write_all(&json)?;
    for asset in assets.data {
        // Reserve the compressed length, stream directly to disk, then backfill.
        let header = file.stream_position()?;
        file.write_all(&0u64.to_le_bytes())?;
        let start = file.stream_position()?;
        let mut encoder = ZlibEncoder::new(&mut *file, Compression::fast());
        encoder.write_all(&asset)?;
        encoder.finish()?;
        let end = file.stream_position()?;
        if end > MAX_FILE_BYTES {
            return Err(invalid("Project exceeds the 1 GiB file limit"));
        }
        file.seek(SeekFrom::Start(header))?;
        file.write_all(&(end - start).to_le_bytes())?;
        file.seek(SeekFrom::Start(end))?;
    }
    Ok(())
}

impl Manifest {
    fn asset(&self, id: usize, length: usize) -> Result<(), FileError> {
        if self.asset_lengths.get(id) != Some(&length) {
            return Err(invalid("Invalid asset reference or length"));
        }
        Ok(())
    }
    fn mask(&self, m: &MaskRecord) -> Result<(), FileError> {
        Raster::new(m.width, m.height, [0; 4]).map_err(FileError::Invalid)?;
        self.asset(m.asset, m.width as usize * m.height as usize)
    }
    fn validate(&self) -> Result<(), FileError> {
        Raster::new(self.width, self.height, [0; 4]).map_err(FileError::Invalid)?;
        if self.layers.is_empty()
            || self.layers.len() > 256
            || self.asset_lengths.len() > 65536
            || self
                .asset_lengths
                .iter()
                .map(|n| *n as u64)
                .try_fold(0u64, |a, n| a.checked_add(n))
                .is_none_or(|n| n > MAX_ASSET_BYTES)
        {
            return Err(invalid(
                "Project exceeds layer/asset or 512 MiB decoded-asset limits",
            ));
        }
        for id in [self.icc, self.exif] {
            if self
                .asset_lengths
                .get(id)
                .is_none_or(|n| *n > 4 * 1024 * 1024)
            {
                return Err(invalid("Invalid ICC/Exif asset"));
            }
        }
        for layer in &self.layers {
            let r = &layer.source;
            Raster::new(r.width, r.height, [0; 4]).map_err(FileError::Invalid)?;
            if r.tiles.len() != (r.width.div_ceil(TILE) * r.height.div_ceil(TILE)) as usize
                || layer.filters.len() > 128
                || layer.effects.len() > 128
            {
                return Err(invalid("Invalid layer geometry or processing counts"));
            }
            for id in &r.tiles {
                self.asset(*id, TILE_BYTES)?;
            }
            for mask in layer
                .mask
                .iter()
                .chain(layer.filters.iter().filter_map(|f| f.mask.as_ref()))
            {
                self.mask(mask)?;
            }
        }
        if self.selection_base.is_some() && self.selection.is_none() {
            return Err(invalid("Original selection requires editing coverage"));
        }
        for mask in self.selection.iter().chain(&self.selection_base) {
            self.mask(mask)?;
            if [mask.width, mask.height] != [self.width, self.height] {
                return Err(invalid("Selection dimensions do not match canvas"));
            }
        }
        Ok(())
    }
}

pub(super) fn read(mut file: File) -> Result<Document, FileError> {
    let mut length = [0; 4];
    file.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_MANIFEST_BYTES {
        return Err(invalid("Project metadata exceeds 4 MiB"));
    }
    let mut json = vec![0; length];
    file.read_exact(&mut json)?;
    let m: Manifest = serde_json::from_slice(&json)?;
    m.validate()?;
    let mut assets = Vec::with_capacity(m.asset_lengths.len());
    for length in &m.asset_lengths {
        let mut header = [0; 8];
        file.read_exact(&mut header)?;
        let compressed = u64::from_le_bytes(header);
        if compressed > MAX_FILE_BYTES
            || compressed
                > file
                    .metadata()?
                    .len()
                    .saturating_sub(file.stream_position()?)
        {
            return Err(invalid("Truncated or oversized compressed asset"));
        }
        let mut decoder = ZlibDecoder::new((&mut file).take(compressed));
        let mut bytes = Vec::new();
        (&mut decoder)
            .take(*length as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() != *length || decoder.total_in() != compressed {
            return Err(invalid(
                "Invalid decoded asset length or trailing compressed data",
            ));
        }
        assets.push(Arc::new(bytes));
    }
    if file.read(&mut [0])? != 0 {
        return Err(invalid("Unexpected trailing project data"));
    }
    let mask = |r: MaskRecord| Coverage {
        width: r.width,
        height: r.height,
        enabled: r.enabled,
        values: assets[r.asset].clone(),
    };
    let layers = m
        .layers
        .into_iter()
        .map(|l| {
            Ok(Layer {
                id: l.id,
                name: l.name,
                source: Raster::from_tiles(
                    l.source.width,
                    l.source.height,
                    l.source
                        .tiles
                        .iter()
                        .map(|id| assets[*id].clone())
                        .collect(),
                )
                .map_err(FileError::Invalid)?,
                visible: l.visible,
                locked: l.locked,
                opacity: l.opacity,
                blend: l.blend,
                offset: l.offset,
                mask: l.mask.map(mask),
                filters: l
                    .filters
                    .into_iter()
                    .map(|f| FilterInstance {
                        id: f.id,
                        enabled: f.enabled,
                        filter: f.filter,
                        mask: f.mask.map(mask),
                    })
                    .collect(),
                effects: l.effects,
            })
        })
        .collect::<Result<Vec<_>, FileError>>()?;
    let doc = Document {
        width: m.width,
        height: m.height,
        layers,
        active_layer: m.active_layer,
        background: m.background,
        next_id: m.next_id,
        selection: m.selection.map(mask),
        selection_base: m.selection_base.map(mask),
        metadata: DocumentMetadata {
            icc_profile: assets[m.icc].clone(),
            exif: assets[m.exif].clone(),
        },
    };
    doc.validate().map_err(FileError::Invalid)?;
    Ok(doc)
}
