//! Versions 2–5: bounded JSON descriptors followed by independently compressed
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
struct LutRecord {
    name: String,
    size: u8,
    domain_min: [f32; 3],
    domain_max: [f32; 3],
    asset: usize,
}
#[derive(Serialize, Deserialize)]
struct FilterRecord {
    id: u64,
    enabled: bool,
    filter: Filter,
    mask: Option<MaskRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map: Option<RasterRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lut: Option<LutRecord>,
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
    lut_ids: HashMap<usize, usize>,
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
    fn raster(&mut self, r: &Raster) -> RasterRecord {
        RasterRecord {
            width: r.width(),
            height: r.height(),
            tiles: r.tiles().iter().map(|t| self.add(t)).collect(),
        }
    }
    fn filter(&mut self, f: &FilterInstance) -> FilterRecord {
        let mut filter = f.filter.clone();
        let mut map_record = None;
        let mut lut_record = None;
        match &mut filter {
            Filter::LandscapeMixer { reference, .. } => {
                map_record = reference.take().map(|r| self.raster(&r));
            }
            Filter::MakeupTransfer { reference, .. } => {
                if let Some(t) = &mut reference.texture {
                    map_record = Arc::make_mut(t).pixels.take().map(|r| self.raster(&r));
                }
            }
            Filter::Displace { map, .. } => {
                map_record = map.take().map(|r| self.raster(&r));
            }
            Filter::ColorLookup { lut, .. } => {
                if let Some(l) = lut.take() {
                    let key = Arc::as_ptr(&l.values) as usize;
                    let asset = if let Some(id) = self.lut_ids.get(&key) {
                        *id
                    } else {
                        let mut bytes = Vec::with_capacity(l.values.len() * 12);
                        for v in l.values.iter().flatten() {
                            bytes.extend_from_slice(&v.to_le_bytes());
                        }
                        let id = self.add(&Arc::new(bytes));
                        self.lut_ids.insert(key, id);
                        id
                    };
                    lut_record = Some(LutRecord {
                        name: l.name,
                        size: l.size,
                        domain_min: l.domain_min,
                        domain_max: l.domain_max,
                        asset,
                    });
                }
            }
            _ => {}
        }
        FilterRecord {
            id: f.id,
            enabled: f.enabled,
            filter,
            mask: f.mask.as_ref().map(|m| self.mask(m)),
            map: map_record,
            lut: lut_record,
        }
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
            filters: l.filters.iter().map(|f| assets.filter(f)).collect(),
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
    fn raster_asset(&self, r: &RasterRecord) -> Result<(), FileError> {
        Raster::new(r.width, r.height, [0; 4]).map_err(FileError::Invalid)?;
        if r.tiles.len() != (r.width.div_ceil(TILE) * r.height.div_ceil(TILE)) as usize {
            return Err(invalid("Invalid filter-map tile count"));
        }
        for id in &r.tiles {
            self.asset(*id, TILE_BYTES)?;
        }
        Ok(())
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
            for f in &layer.filters {
                match (&f.filter, &f.map, &f.lut) {
                    (
                        Filter::LandscapeMixer {
                            reference: None,
                            reference_name,
                            method,
                            ..
                        },
                        Some(r),
                        None,
                    ) => {
                        let limit = if *method == LandscapeMethod::HorizonBlendV1 {
                            512
                        } else {
                            crate::processing::landscape::REFERENCE_EDGE
                        };
                        if r.width > limit || r.height > limit || reference_name.is_empty() {
                            return Err(invalid("Invalid landscape reference asset"));
                        }
                        self.raster_asset(r)?;
                    }
                    (
                        Filter::LandscapeMixer {
                            reference: None,
                            reference_name,
                            ..
                        },
                        None,
                        None,
                    ) if reference_name.is_empty() => {}
                    (Filter::LandscapeMixer { .. }, _, _) => {
                        return Err(invalid("Missing or inline landscape reference asset"));
                    }
                    (Filter::MakeupTransfer { reference, .. }, Some(r), None) => {
                        if reference
                            .texture
                            .as_ref()
                            .is_none_or(|t| t.pixels.is_some())
                            || r.width != r.height
                            || !(128..=512).contains(&r.width)
                        {
                            return Err(invalid("Invalid spatial makeup asset"));
                        }
                        self.raster_asset(r)?;
                    }
                    (Filter::MakeupTransfer { reference, .. }, None, None)
                        if reference.texture.is_some() =>
                    {
                        return Err(invalid("Missing spatial makeup asset"));
                    }
                    (Filter::Displace { map: None, .. }, Some(r), None) => {
                        if r.width > 1024 || r.height > 1024 {
                            return Err(invalid("Displacement map exceeds 1024 × 1024"));
                        }
                        self.raster_asset(r)?;
                    }
                    (Filter::ColorLookup { lut: None, .. }, None, Some(l)) => {
                        if !(2..=33).contains(&l.size) {
                            return Err(invalid("Invalid LUT dimensions"));
                        }
                        self.asset(l.asset, usize::from(l.size).pow(3) * 12)?;
                        // Domains/name are checked again with decoded finite values.
                        if l.name.chars().count() > 128 {
                            return Err(invalid("Invalid LUT name"));
                        }
                    }
                    (Filter::Displace { .. } | Filter::ColorLookup { .. }, _, _) => {
                        return Err(invalid("Missing or inline filter asset"));
                    }
                    (_, None, None) => {}
                    _ => return Err(invalid("Unexpected filter asset")),
                }
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
    let mut lut_values = HashMap::<usize, Arc<Vec<[f32; 3]>>>::new();
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
                    .map(|f| {
                        let mut filter = f.filter;
                        match &mut filter {
                            Filter::LandscapeMixer { reference, .. } => {
                                if let Some(r) = f.map {
                                    *reference = Some(
                                        Raster::from_tiles(
                                            r.width,
                                            r.height,
                                            r.tiles.iter().map(|id| assets[*id].clone()).collect(),
                                        )
                                        .map_err(FileError::Invalid)?,
                                    );
                                }
                            }
                            Filter::MakeupTransfer { reference, .. } => {
                                if let Some(t) = &mut reference.texture {
                                    let r =
                                        f.map.ok_or_else(|| invalid("Missing makeup pixels"))?;
                                    Arc::make_mut(t).pixels = Some(
                                        Raster::from_tiles(
                                            r.width,
                                            r.height,
                                            r.tiles.iter().map(|id| assets[*id].clone()).collect(),
                                        )
                                        .map_err(FileError::Invalid)?,
                                    );
                                }
                            }
                            Filter::Displace { map, .. } => {
                                let r = f.map.ok_or_else(|| invalid("Missing map"))?;
                                *map = Some(
                                    Raster::from_tiles(
                                        r.width,
                                        r.height,
                                        r.tiles.iter().map(|id| assets[*id].clone()).collect(),
                                    )
                                    .map_err(FileError::Invalid)?,
                                );
                            }
                            Filter::ColorLookup { lut, .. } => {
                                let l = f.lut.ok_or_else(|| invalid("Missing LUT"))?;
                                let values = lut_values
                                    .entry(l.asset)
                                    .or_insert_with(|| {
                                        Arc::new(
                                            assets[l.asset]
                                                .as_chunks::<12>()
                                                .0
                                                .iter()
                                                .map(|p| {
                                                    std::array::from_fn(|c| {
                                                        f32::from_le_bytes(
                                                            p[c * 4..c * 4 + 4]
                                                                .try_into()
                                                                .expect("float bytes"),
                                                        )
                                                    })
                                                })
                                                .collect(),
                                        )
                                    })
                                    .clone();
                                *lut = Some(Lut3D {
                                    name: l.name,
                                    size: l.size,
                                    domain_min: l.domain_min,
                                    domain_max: l.domain_max,
                                    values,
                                });
                            }
                            _ => {}
                        }
                        Ok(FilterInstance {
                            id: f.id,
                            enabled: f.enabled,
                            filter,
                            mask: f.mask.map(mask),
                        })
                    })
                    .collect::<Result<Vec<_>, FileError>>()?,
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
