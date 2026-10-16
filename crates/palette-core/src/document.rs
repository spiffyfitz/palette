//! Version-independent editing state. No GUI or GPU types belong here.
use crate::raster::{Coverage, Raster};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

/// Blend functions currently supported by the reference compositor.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
}

/// Only implemented filters enter the model and the registry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Filter {
    Invert,
    Brightness { amount: f32 },
}

/// Stable, individually maskable instance in a layer's ordered filter pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FilterInstance {
    pub id: u64,
    pub enabled: bool,
    pub filter: Filter,
    pub mask: Option<Coverage>,
}

/// Implemented layer-style operations; new styles extend this versioned enum.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Effect {
    /// Blend constant RGB through the layer's alpha silhouette.
    ColorOverlay { color: [u8; 3], opacity: f32 },
}

/// An independently enabled, nondestructive layer-style operation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EffectInstance {
    pub id: u64,
    pub enabled: bool,
    pub effect: Effect,
}

/// Source pixels remain untouched by masks, filters, blending and positioning.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    pub source: Raster,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub offset: [i32; 2],
    pub mask: Option<Coverage>,
    pub filters: Vec<FilterInstance>,
    pub effects: Vec<EffectInstance>,
}

/// Layers are stored bottom to top; IDs are monotonic within a document.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: u64,
    pub selection: Option<Coverage>,
    pub(crate) next_id: u64,
    /// Opaque source metadata retained by native projects. Pixels currently use
    /// the editor's RGBA8/sRGB working interpretation; profiles are not applied.
    #[serde(default)]
    pub metadata: DocumentMetadata,
}

/// Bounded original metadata, preserved independently from raster source data.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct DocumentMetadata {
    pub icc_profile: Arc<Vec<u8>>,
    pub exif: Arc<Vec<u8>>,
}

impl Document {
    pub(crate) fn allocations(&self) -> std::collections::HashMap<usize, usize> {
        let mut output = std::collections::HashMap::new();
        for data in [&self.metadata.icc_profile, &self.metadata.exif] {
            output.insert(Arc::as_ptr(data) as usize, data.capacity());
        }
        let mut mask = |mask: &Coverage| {
            output.insert(
                std::sync::Arc::as_ptr(&mask.values) as usize,
                mask.values.capacity(),
            );
        };
        if let Some(selection) = &self.selection {
            mask(selection);
        }
        for layer in &self.layers {
            if let Some(m) = &layer.mask {
                mask(m);
            }
            for filter in &layer.filters {
                if let Some(m) = &filter.mask {
                    mask(m);
                }
            }
        }
        for layer in &self.layers {
            layer.source.allocations(&mut output);
        }
        output
    }
    /// Create a white default background of the requested bounded dimensions.
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let source = Raster::new(width, height, [255; 4])?;
        Ok(Self::from_raster(source))
    }

    /// Wrap a raster as an editable single-layer document.
    pub fn from_raster(source: Raster) -> Self {
        Self {
            width: source.width(),
            height: source.height(),
            layers: vec![Layer {
                id: 1,
                name: "Background".into(),
                source,
                visible: true,
                locked: false,
                opacity: 1.0,
                blend: BlendMode::Normal,
                offset: [0, 0],
                mask: None,
                filters: vec![],
                effects: vec![],
            }],
            active_layer: 1,
            selection: None,
            next_id: 2,
            metadata: DocumentMetadata::default(),
        }
    }

    /// Allocate a stable ID within this document's current command history.
    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Return the active layer; requires a validated document.
    pub fn active(&self) -> &Layer {
        self.layers
            .iter()
            .find(|l| l.id == self.active_layer)
            .expect("validated active layer")
    }
    /// Mutably access the active layer; requires a validated document.
    pub fn active_mut(&mut self) -> &mut Layer {
        self.layers
            .iter_mut()
            .find(|l| l.id == self.active_layer)
            .expect("validated active layer")
    }
    /// Add and activate a transparent, full-canvas layer at the top of the stack.
    pub fn add_layer(&mut self) {
        let id = self.allocate_id();
        self.layers.push(Layer {
            id,
            name: format!("Layer {id}"),
            source: Raster::new(self.width, self.height, [0; 4]).expect("valid canvas"),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend: BlendMode::Normal,
            offset: [0, 0],
            mask: None,
            filters: vec![],
            effects: vec![],
        });
        self.active_layer = id;
    }
    /// Duplicate the active layer and allocate fresh layer/filter/effect IDs.
    pub fn duplicate_layer(&mut self) {
        let mut copy = self.active().clone();
        copy.id = self.allocate_id();
        for filter in &mut copy.filters {
            filter.id = self.allocate_id();
        }
        for effect in &mut copy.effects {
            effect.id = self.allocate_id();
        }
        copy.name.push_str(" copy");
        let index = self
            .layers
            .iter()
            .position(|l| l.id == self.active_layer)
            .expect("active");
        self.active_layer = copy.id;
        self.layers.insert(index + 1, copy);
    }
    /// Remove the active layer while retaining at least one raster layer.
    pub fn delete_layer(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        let index = self
            .layers
            .iter()
            .position(|l| l.id == self.active_layer)
            .expect("active");
        self.layers.remove(index);
        self.active_layer = self.layers[index.min(self.layers.len() - 1)].id;
    }

    /// Logical raster/mask bytes; shared backing allocations may use less RAM.
    pub fn byte_len(&self) -> usize {
        self.layers
            .iter()
            .map(|l| {
                l.source.byte_len()
                    + l.mask.as_ref().map_or(0, |m| m.values.len())
                    + l.filters
                        .iter()
                        .map(|f| f.mask.as_ref().map_or(0, |m| m.values.len()))
                        .sum::<usize>()
            })
            .sum::<usize>()
            + self.selection.as_ref().map_or(0, |m| m.values.len())
            + self.metadata.icc_profile.len()
            + self.metadata.exif.len()
    }

    /// Validate all serialized invariants before any indexing or rendering.
    pub fn validate(&self) -> Result<(), String> {
        if self.metadata.icc_profile.len() > 4 * 1024 * 1024
            || self.metadata.exif.len() > 4 * 1024 * 1024
        {
            return Err("ICC/Exif metadata exceeds the 4 MiB per-field limit".into());
        }
        if self.layers.is_empty() || self.layers.len() > 256 || self.next_id > u64::MAX - 1024 {
            return Err("Invalid layer count or ID counter".into());
        }
        Raster::new(self.width, self.height, [0; 4])?;
        let mut ids = HashSet::new();
        for layer in &self.layers {
            if layer.id == 0
                || layer.id >= self.next_id
                || !ids.insert(layer.id)
                || !layer.opacity.is_finite()
                || !(0.0..=1.0).contains(&layer.opacity)
                || layer.name.len() > 4096
                || layer.filters.len() > 128
                || layer.effects.len() > 128
            {
                return Err("Invalid layer properties".into());
            }
            layer.source.validate()?;
            if let Some(mask) = &layer.mask {
                mask.validate()?;
            }
            for filter in &layer.filters {
                if filter.id == 0 || filter.id >= self.next_id || !ids.insert(filter.id) {
                    return Err("Invalid filter ID".into());
                }
                if let Filter::Brightness { amount } = filter.filter
                    && (!amount.is_finite() || !(-1.0..=1.0).contains(&amount))
                {
                    return Err("Invalid brightness".into());
                }
                if let Some(mask) = &filter.mask {
                    mask.validate()?;
                }
            }
        }
        for effect in self.layers.iter().flat_map(|l| &l.effects) {
            if effect.id == 0 || effect.id >= self.next_id || !ids.insert(effect.id) {
                return Err("Invalid effect ID".into());
            }
            let Effect::ColorOverlay { opacity, .. } = effect.effect;
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
                return Err("Invalid effect opacity".into());
            }
        }
        if !self.layers.iter().any(|l| l.id == self.active_layer) {
            return Err("Active layer is missing".into());
        }
        if let Some(selection) = &self.selection {
            selection.validate()?;
            if selection.width != self.width || selection.height != self.height {
                return Err("Selection dimensions must match the canvas".into());
            }
        }
        Ok(())
    }
}
