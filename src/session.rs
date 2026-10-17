use eframe::egui::{TextureHandle, Vec2};
use palette_core::editing::PixelRect;
use palette_core::{document::Document, history::History, raster::Raster};
use std::path::PathBuf;

mod selection_brush;
pub use selection_brush::{BrushFrame, BrushSelection};

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
    pub lasso_drag: Option<LassoDrag>,
    pub paint_stroke: Option<palette_core::brush::PaintStroke>,
    pub text_drag: Option<[f32; 2]>,
    pub layer_drag: Option<LayerDrag>,
    pub selection_transform: Option<SelectionGesture>,
    pub gesture_error: Option<String>,
    pub construction: Option<Construction>,
    pub selection_mode: palette_core::selection::Combine,
    pub wand: palette_core::selection::Wand,
    pub selection_feather: f32,
    pub selection_feather_text: String,
    pub wand_tolerance_text: String,
    pub feather_preview: Option<SelectionFeather>,
    pub selection_brush: palette_core::selection::brush::Settings,
    pub selection_brush_text: String,
    pub brush_overlay_enabled: bool,
    pub selection_brush_draft: Option<BrushSelection>,
    pub selection_brush_overlay: Option<(u64, TextureHandle)>,
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
            lasso_drag: None,
            paint_stroke: None,
            text_drag: None,
            layer_drag: None,
            selection_transform: None,
            gesture_error: None,
            construction: None,
            selection_mode: Default::default(),
            wand: Default::default(),
            selection_feather: 0.,
            selection_feather_text: "0".into(),
            wand_tolerance_text: "12".into(),
            feather_preview: None,
            selection_brush: Default::default(),
            selection_brush_text: "12".into(),
            brush_overlay_enabled: true,
            selection_brush_draft: None,
            selection_brush_overlay: None,
            selection_cache: None,
        }
    }
    pub fn modified(&self) -> bool {
        self.selection_brush_draft.is_some()
            || self
                .feather_preview
                .as_ref()
                .is_some_and(|p| !p.dialog && p.radius != p.setting_before)
            || self.selection_transform.as_ref().is_some_and(|g| {
                g.desired != palette_core::transform::SelectionTransform::Move([0, 0])
            })
            || self.history.revision != self.saved_revision
            || self
                .layer_drag
                .as_ref()
                .is_some_and(|drag| self.document.active().offset != drag.original)
    }
    pub fn invalidate(&mut self) {
        self.cancel_brush_selection();
        self.cancel_selection_feather();
        self.generation += 1;
        self.render_epoch += 1;
        self.paint_dirty = None;
        self.requested = None;
    }
    /// A gesture may present progressively newer worker results within its epoch.
    /// Start/end/cancel use invalidate() to fence off unrelated snapshots.
    pub fn invalidate_preview(&mut self) {
        self.generation += 1;
        self.requested = None;
    }
    pub fn invalidate_paint(&mut self, region: Option<PixelRect>) {
        self.invalidate_preview();
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
        self.finish_brush_selection();
        self.finish_selection_feather();
        self.construction = None;
        self.finish_selection_transform();
        if let Some(drag) = self.layer_drag.take() {
            let position = self.document.active().offset;
            if position != drag.original {
                self.document.active_mut().offset = drag.original;
                self.history.begin("Move layer", &self.document);
                self.document.active_mut().offset = position;
                self.invalidate();
            }
        }
        self.paint_stroke = None;
        self.selection_drag = None;
        self.lasso_drag = None;
        self.text_drag = None;
        self.history.finish(&self.document);
    }
    pub fn undo(&mut self) {
        self.end_gesture();
        self.paint_stroke = None;
        self.selection_drag = None;
        self.history.undo(&mut self.document);
        self.invalidate();
    }
    pub fn redo(&mut self) {
        self.end_gesture();
        self.paint_stroke = None;
        self.selection_drag = None;
        self.history.redo(&mut self.document);
        self.invalidate();
    }
    pub fn selection_bounds(&mut self) -> Option<PixelRect> {
        if let Some(frame) = self
            .selection_brush_draft
            .as_ref()
            .and_then(|d| d.frame.as_ref())
        {
            return frame.bounds;
        }
        self.cache_selection();
        self.selection_cache.as_ref().and_then(|cache| cache.bounds)
    }
    fn cache_selection(&mut self) {
        let Some(mask) = &self.document.selection else {
            self.selection_cache = None;
            return;
        };
        if self.selection_cache.as_ref().is_some_and(|cache| {
            std::sync::Arc::ptr_eq(&cache.values, &mask.values)
                && cache.geometry == (mask.width, mask.height, mask.enabled)
                && match (&cache.base, &self.document.selection_base) {
                    (None, None) => true,
                    (Some(a), Some(b)) => {
                        std::sync::Arc::ptr_eq(&a.values, &b.values)
                            && (a.width, a.height, a.enabled) == (b.width, b.height, b.enabled)
                    }
                    _ => false,
                }
        }) {
            return;
        }
        self.selection_cache = Some(SelectionCache {
            values: mask.values.clone(),
            geometry: (mask.width, mask.height, mask.enabled),
            base: self.document.selection_base.clone(),
            bounds: palette_core::editing::bounds(mask),
            contours: std::sync::Arc::new(palette_core::editing::contours(
                self.document.selection_base.as_ref().unwrap_or(mask),
            )),
            feather: std::sync::Arc::new(if self.document.selection_base.is_some() {
                palette_core::editing::contours_at(mask, 1)
            } else {
                vec![]
            }),
        });
    }
    pub fn selection_contours(&mut self) -> &[Vec<[f32; 2]>] {
        if self
            .selection_brush_draft
            .as_ref()
            .is_some_and(|d| d.frame.is_some())
        {
            return &self
                .selection_brush_draft
                .as_ref()
                .unwrap()
                .frame
                .as_ref()
                .unwrap()
                .contours;
        }
        self.cache_selection();
        self.selection_cache
            .as_ref()
            .map_or(&[], |cache| &cache.contours)
    }
    pub fn feather_contours(&mut self) -> &[Vec<[f32; 2]>] {
        if self
            .selection_brush_draft
            .as_ref()
            .is_some_and(|d| d.frame.is_some())
        {
            return &self
                .selection_brush_draft
                .as_ref()
                .unwrap()
                .frame
                .as_ref()
                .unwrap()
                .feather;
        }
        if self
            .feather_preview
            .as_ref()
            .is_some_and(|p| p.frame.is_some())
        {
            return &self
                .feather_preview
                .as_ref()
                .unwrap()
                .frame
                .as_ref()
                .unwrap()
                .contours;
        }
        self.cache_selection();
        self.selection_cache
            .as_ref()
            .map_or(&[], |cache| &cache.feather)
    }
    pub fn selection_outline(&mut self) -> std::sync::Arc<Vec<Vec<[f32; 2]>>> {
        self.cache_selection();
        self.selection_cache
            .as_ref()
            .map(|c| c.contours.clone())
            .unwrap_or_default()
    }
}

/// Presentation transaction: source/committed coverage stays immutable while
/// a bounded worker evaluates progressively newer radii. One release = one undo.
pub struct SelectionFeather {
    pub token: u64,
    pub generation: u64,
    pub source: palette_core::raster::Coverage,
    pub outline: std::sync::Arc<Vec<Vec<[f32; 2]>>>,
    pub radius: f32,
    pub setting_before: f32,
    pub dialog: bool,
    pub commit_requested: bool,
    pub frame: Option<FeatherFrame>,
}
pub struct FeatherFrame {
    pub radius: f32,
    pub coverage: palette_core::raster::Coverage,
    pub contours: std::sync::Arc<Vec<Vec<[f32; 2]>>>,
    pub bounds: Option<PixelRect>,
}
impl FeatherFrame {
    pub fn compute(source: &palette_core::raster::Coverage, radius: f32) -> Result<Self, String> {
        let coverage = palette_core::selection::feather_region(source, radius)?;
        let contours = if coverage == *source {
            vec![]
        } else {
            palette_core::editing::contours_at(&coverage, 1)
        };
        let bounds = palette_core::editing::bounds(&coverage);
        Ok(Self {
            radius,
            coverage,
            contours: std::sync::Arc::new(contours),
            bounds,
        })
    }
}
impl Session {
    /// Explicit document commands finish a toolbar gesture before proceeding.
    /// A pending dialog is discarded until Apply; the normal release path calls
    /// this only after the worker has the exact final result. The synchronous
    /// fallback is reserved for tool/tab/save/undo changes during a pending job.
    pub fn finish_selection_feather(&mut self) {
        let Some(preview) = self.feather_preview.take() else {
            return;
        };
        if preview.generation != self.generation || (preview.dialog && !preview.commit_requested) {
            return;
        }
        let frame = match preview.frame.filter(|f| f.radius == preview.radius) {
            Some(frame) => frame,
            None => match FeatherFrame::compute(&preview.source, preview.radius) {
                Ok(frame) => frame,
                Err(error) => {
                    self.gesture_error = Some(error);
                    return;
                }
            },
        };
        let base = (frame.coverage != preview.source).then_some(preview.source);
        if self.document.selection.as_ref() != Some(&frame.coverage)
            || self.document.selection_base != base
        {
            self.history.begin("Feather selection", &self.document);
            let cache = SelectionCache {
                values: frame.coverage.values.clone(),
                geometry: (
                    frame.coverage.width,
                    frame.coverage.height,
                    frame.coverage.enabled,
                ),
                base: base.clone(),
                bounds: frame.bounds,
                contours: preview.outline,
                feather: frame.contours,
            };
            self.document.selection = Some(frame.coverage);
            self.document.selection_base = base;
            self.history.finish(&self.document);
            self.invalidate();
            self.selection_cache = Some(cache);
        }
    }
    pub fn cancel_selection_feather(&mut self) -> bool {
        let Some(preview) = self.feather_preview.take() else {
            return false;
        };
        if !preview.dialog {
            self.selection_feather = preview.setting_before;
        }
        true
    }
}

/// A layer move changes only placement during preview; one snapshot is recorded
/// at completion. Cancel restores placement without consuming a history revision.
pub struct LayerDrag {
    pub start: [f32; 2],
    pub original: [i32; 2],
}
impl Session {
    pub fn cancel_layer_move(&mut self) -> bool {
        if let Some(drag) = self.layer_drag.take() {
            self.document.active_mut().offset = drag.original;
            self.invalidate();
            true
        } else {
            false
        }
    }
}

struct SelectionCache {
    values: std::sync::Arc<Vec<u8>>,
    geometry: (u32, u32, bool),
    base: Option<palette_core::raster::Coverage>,
    bounds: Option<PixelRect>,
    contours: std::sync::Arc<Vec<Vec<[f32; 2]>>>,
    feather: std::sync::Arc<Vec<Vec<[f32; 2]>>>,
}

pub struct SelectionDrag {
    pub start: [f32; 2],
    pub current: [f32; 2],
    pub mode: palette_core::selection::Combine,
}

/// In-progress freehand contour, retained per tab until release or cancellation.
pub struct LassoDrag {
    pub points: Vec<[f32; 2]>,
    pub mode: palette_core::selection::Combine,
}
impl LassoDrag {
    pub fn sample(&mut self, point: [f32; 2], zoom: f32) {
        if self
            .points
            .last()
            .is_some_and(|p| (p[0] - point[0]).hypot(p[1] - point[1]) * zoom < 0.5)
        {
            return;
        }
        if self.points.len() >= 4096 {
            // Preserve both endpoints while bounding exceptionally long drags.
            // Sampling uses display distance so high zoom retains fine details.
            let last = self.points.last().copied();
            self.points = self.points.iter().step_by(2).copied().collect();
            if let Some(last) = last {
                self.points.push(last);
            }
        }
        self.points.push(point);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    SelectBrush,
    Wand,
    Lasso,
    Polygon,
    Curve,
    MoveLayer,
    Shape(palette_core::shapes::Kind),
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
            Self::MoveLayer => "Move layer",
            Self::Shape(kind) => kind.label(),
            Self::Polygon => "Polygon",
            Self::Curve => "Curve",
            Self::Select => "Rectangular selection",
            Self::SelectBrush => "Selection brush",
            Self::Wand => "Magic Wand",
            Self::Lasso => "Freeform selection",
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
        self.shape_kind().is_some() || matches!(self, Self::Polygon | Self::Curve)
    }
    pub fn shape_kind(self) -> Option<palette_core::shapes::Kind> {
        use palette_core::shapes::Kind;
        match self {
            Self::Line => Some(Kind::Line),
            Self::Rectangle => Some(Kind::Rectangle),
            Self::Ellipse => Some(Kind::Ellipse),
            Self::Shape(kind) => Some(kind),
            _ => None,
        }
    }
}

/// A tool transaction retains image-space geometry until it is explicitly closed.
/// Curve points are [start, control1, control2, end]; polygon points are vertices.
pub struct Construction {
    pub tool: Tool,
    pub points: Vec<[f32; 2]>,
    pub hover: [f32; 2],
    pub dragging: bool,
    pub stage: u8,
    pub style: palette_core::shapes::Style,
}

/// A floating raster transaction. COW source tiles are retained once per gesture.
/// Worker previews may progress behind the pointer; only the exact final target
/// is committed. Escape restores the original document and fences stale results.
pub struct SelectionGesture {
    pub base: Document,
    pub bounds: PixelRect,
    pub start: [f32; 2],
    pub handle: SelectionHandle,
    pub desired: palette_core::transform::SelectionTransform,
    pub applied: Option<palette_core::transform::SelectionTransform>,
    pub released: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionHandle {
    Move,
    Resize([i8; 2]),
    Rotate,
}

impl Session {
    pub fn begin_selection_transform(&mut self, handle: SelectionHandle, start: [f32; 2]) {
        if self.document.active().locked {
            return;
        }
        let Some(bounds) = self.selection_bounds() else {
            return;
        };
        self.end_gesture();
        self.invalidate();
        self.selection_transform = Some(SelectionGesture {
            base: self.document.clone(),
            bounds,
            start,
            handle,
            desired: palette_core::transform::SelectionTransform::Move([0, 0]),
            applied: None,
            released: false,
        });
    }

    pub fn cancel_selection_transform(&mut self) -> bool {
        let Some(gesture) = self.selection_transform.take() else {
            return false;
        };
        self.document = gesture.base;
        self.invalidate();
        true
    }

    /// Normal pointer release waits for the worker's exact final preview. Other
    /// commands must finish the target before saving, switching tabs or undoing;
    /// this fallback evaluates synchronously only when that preview is pending.
    pub fn finish_selection_transform(&mut self) {
        let Some(gesture) = self.selection_transform.take() else {
            return;
        };
        let mut result = if gesture.applied == Some(gesture.desired) {
            self.document.clone()
        } else {
            gesture.base.clone()
        };
        if gesture.applied != Some(gesture.desired)
            && let Err(error) = gesture.desired.apply(&mut result)
        {
            self.gesture_error = Some(error);
            result = gesture.base.clone();
        }
        self.document = gesture.base;
        if result != self.document {
            self.history.begin("Transform selection", &self.document);
            self.document = result;
            self.history.finish(&self.document);
        }
        self.invalidate();
    }
}
