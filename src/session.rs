use eframe::egui::{TextureHandle, Vec2};
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
        self.history.begin(label, &self.document);
        change(&mut self.document);
        self.invalidate();
    }
    pub fn undo(&mut self) {
        self.history.undo(&mut self.document);
        self.invalidate();
    }
    pub fn redo(&mut self) {
        self.history.redo(&mut self.document);
        self.invalidate();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
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
