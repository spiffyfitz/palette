use eframe::egui::{TextureHandle, Vec2};
use palette_core::editing::PixelRect;
use palette_core::{document::Document, history::History, raster::Raster};
use std::path::PathBuf;

/// Per-tab editing and viewport state; document data itself stays in palette-core.
pub struct Session {
    pub id: u64,
    pub name: String,
    pub path: Option<PathBuf>,
    pub document: Document,
    pub history: History,
    pub saved_revision: u64,
    pub generation: u64,
    /// General edits start a new render epoch. Painting within one epoch may
    /// display older snapshots progressively, then patch every dirty pixel.
    pub render_epoch: u64,
    pub rendered_epoch: Option<u64>,
    pub rendered_generation: Option<u64>,
    pub paint_dirty: Option<PixelRect>,
    pub requested: Option<u64>,
    pub texture: Option<TextureHandle>,
    pub rendered: Option<Raster>,
    pub zoom: f32,
    pub pan: Vec2,
    pub fit: bool,
    /// Selection gestures belong to a tab, so switching documents cannot commit
    /// a drag into another document or discard a different tab's tool state.
    pub selection_drag: Option<SelectionDrag>,
    pub paint_stroke: Option<palette_core::brush::PaintStroke>,
    pub text_drag: Option<[f32; 2]>,
    selection_cache: Option<SelectionCache>,
}

impl Session {
    pub fn new(id: u64, name: String, document: Document) -> Self {
        Self {
            id,
            name,
            path: None,
            document,
            history: History::default(),
            saved_revision: 0,
            generation: 0,
            render_epoch: 0,
            rendered_epoch: None,
            rendered_generation: None,
            paint_dirty: None,
            requested: None,
            texture: None,
            rendered: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit: false,
            selection_drag: None,
            paint_stroke: None,
            text_drag: None,
            selection_cache: None,
        }
    }
    pub fn modified(&self) -> bool {
        self.history.revision != self.saved_revision
    }
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.render_epoch += 1;
        self.paint_dirty = None;
        self.requested = None;
    }
    pub fn invalidate_paint(&mut self, region: Option<PixelRect>) {
        self.generation += 1;
        self.requested = None;
        if let Some(region) = region {
            self.paint_dirty = Some(self.paint_dirty.map_or(region, |previous| {
                let x = previous.x.min(region.x);
                let y = previous.y.min(region.y);
                PixelRect {
                    x,
                    y,
                    width: (previous.x + previous.width).max(region.x + region.width) - x,
                    height: (previous.y + previous.height).max(region.y + region.height) - y,
                }
            }));
        }
    }
    pub fn command(&mut self, label: &str, change: impl FnOnce(&mut Document)) {
        self.end_gesture();
        self.history.begin(label, &self.document);
        change(&mut self.document);
        self.history.finish(&self.document);
        self.invalidate();
    }
    pub fn end_gesture(&mut self) {
        self.paint_stroke = None;
        self.selection_drag = None;
        self.text_drag = None;
        self.history.finish(&self.document);
    }
    pub fn undo(&mut self) {
        self.paint_stroke = None;
        self.selection_drag = None;
        self.history.undo(&mut self.document);
        self.invalidate();
    }
    pub fn redo(&mut self) {
        self.paint_stroke = None;
        self.selection_drag = None;
        self.history.redo(&mut self.document);
        self.invalidate();
    }
    pub fn selection_bounds(&mut self) -> Option<PixelRect> {
        self.cache_selection();
        self.selection_cache.as_ref().and_then(|cache| cache.bounds)
    }
    fn cache_selection(&mut self) {
        let Some(mask) = &self.document.selection else {
            self.selection_cache = None;
            return;
        };
        if self
            .selection_cache
            .as_ref()
            .is_some_and(|cache| std::sync::Arc::ptr_eq(&cache.values, &mask.values))
        {
            return;
        }
        self.selection_cache = Some(SelectionCache {
            values: mask.values.clone(),
            bounds: palette_core::editing::bounds(mask),
            edges: palette_core::editing::outline(mask),
        });
    }
    pub fn selection_edges(&mut self) -> &[[[f32; 2]; 2]] {
        self.cache_selection();
        self.selection_cache
            .as_ref()
            .map_or(&[], |cache| &cache.edges)
    }
}

struct SelectionCache {
    values: std::sync::Arc<Vec<u8>>,
    bounds: Option<PixelRect>,
    edges: Vec<[[f32; 2]; 2]>,
}

pub struct SelectionDrag {
    pub start: [f32; 2],
    pub current: [f32; 2],
    pub moving: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Fill,
    Pencil,
    Brush,
    Eraser,
    Picker,
    Magnifier,
    Text,
    Line,
    Rectangle,
    Ellipse,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Rectangular selection",
            Self::Fill => "Fill bucket",
            Self::Pencil => "Pencil",
            Self::Brush => "Brush",
            Self::Eraser => "Eraser",
            Self::Picker => "Color picker",
            Self::Magnifier => "Magnifier",
            Self::Text => "Text",
            Self::Line => "Line",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
        }
    }
    pub fn paints(self) -> bool {
        matches!(self, Self::Pencil | Self::Brush | Self::Eraser)
    }
    pub fn shape(self) -> bool {
        matches!(self, Self::Line | Self::Rectangle | Self::Ellipse)
    }
}
