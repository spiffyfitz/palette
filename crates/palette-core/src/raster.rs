//! Straight-alpha sRGB pixels in copy-on-write tiles. History shares untouched tiles.
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Tile edge length in source pixels, including boundary padding.
pub const TILE: u32 = 128;
/// Shared allocation/decoding policy: at most 64 megapixels per raster.
pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;

/// A bounded raster; out-of-bounds reads are transparent and writes are ignored.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Raster {
    width: u32,
    height: u32,
    tiles: Vec<Arc<Vec<u8>>>,
}

impl Raster {
    pub(crate) fn allocations(&self, output: &mut std::collections::HashMap<usize, usize>) {
        for tile in &self.tiles {
            output.insert(Arc::as_ptr(tile) as usize, tile.capacity());
        }
    }
    /// Allocate a solid raster, sharing equal tiles until they are modified.
    pub fn new(width: u32, height: u32, color: [u8; 4]) -> Result<Self, String> {
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
            return Err("Image dimensions must be positive and at most 64 megapixels".into());
        }
        let tile = Arc::new(color.repeat((TILE * TILE) as usize));
        Ok(Self {
            width,
            height,
            tiles: vec![tile; (width.div_ceil(TILE) * height.div_ceil(TILE)) as usize],
        })
    }

    /// Import tightly packed row-major, straight-alpha RGBA8 pixels.
    pub fn from_rgba(width: u32, height: u32, bytes: &[u8]) -> Result<Self, String> {
        let mut raster = Self::new(width, height, [0; 4])?;
        if bytes.len() as u64 != u64::from(width) * u64::from(height) * 4 {
            return Err("Pixel buffer length does not match dimensions".into());
        }
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                raster.set(x, y, bytes[i..i + 4].try_into().expect("four channels"));
            }
        }
        Ok(raster)
    }

    /// Width in source pixels, excluding tile padding.
    pub fn width(&self) -> u32 {
        self.width
    }
    /// Height in source pixels, excluding tile padding.
    pub fn height(&self) -> u32 {
        self.height
    }
    /// Conservative storage estimate; counts shared tile allocations repeatedly.
    pub fn byte_len(&self) -> usize {
        self.tiles.len() * (TILE * TILE * 4) as usize
    }

    /// Check serialized dimensions and tile lengths before reading pixels.
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || u64::from(self.width) * u64::from(self.height) > MAX_PIXELS
            || self.tiles.len() != (self.width.div_ceil(TILE) * self.height.div_ceil(TILE)) as usize
            || self
                .tiles
                .iter()
                .any(|t| t.len() != (TILE * TILE * 4) as usize)
        {
            return Err("Invalid raster dimensions or tile data".into());
        }
        Ok(())
    }

    fn address(&self, x: u32, y: u32) -> (usize, usize) {
        let tile = (y / TILE * self.width.div_ceil(TILE) + x / TILE) as usize;
        (tile, ((y % TILE * TILE + x % TILE) * 4) as usize)
    }

    /// Read a straight-alpha pixel, returning transparent outside the raster.
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0; 4];
        }
        let (tile, i) = self.address(x, y);
        self.tiles[tile][i..i + 4]
            .try_into()
            .expect("four channels")
    }

    /// Replace a pixel using tile-level copy-on-write; ignore outside writes.
    pub fn set(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let (tile, i) = self.address(x, y);
        Arc::make_mut(&mut self.tiles[tile])[i..i + 4].copy_from_slice(&color);
    }

    /// Materialize tightly packed row-major RGBA8 without tile padding.
    pub fn rgba(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                bytes.extend_from_slice(&self.get(x, y));
            }
        }
        bytes
    }
}

/// Grayscale coverage: white reveals, black hides. Used for selections and masks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Coverage {
    pub width: u32,
    pub height: u32,
    pub values: Arc<Vec<u8>>,
    pub enabled: bool,
}

impl Coverage {
    /// Validate the coverage plane before indexing deserialized input.
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || u64::from(self.width) * u64::from(self.height) > MAX_PIXELS
            || self.values.len() as u64 != u64::from(self.width) * u64::from(self.height)
        {
            return Err("Invalid coverage mask".into());
        }
        Ok(())
    }

    /// Normalized coverage. Disabled masks reveal all, including outside bounds.
    pub fn at(&self, x: u32, y: u32) -> f32 {
        if !self.enabled {
            return 1.0;
        }
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        f32::from(self.values[(y * self.width + x) as usize]) / 255.0
    }
}
