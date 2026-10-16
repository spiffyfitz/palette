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
            requested: None,
            texture: None,
            rendered: None,
            zoom: 0.75,
            pan: Vec2::ZERO,
            fit: false,
            selection_drag: None,
            paint_stroke: None,
            selection_cache: None,
        }
    }
    pub fn modified(&self) -> bool {
        self.history.revision != self.saved_revision
    }
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.requested = None;
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
