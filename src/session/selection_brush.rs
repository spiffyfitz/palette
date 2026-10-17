//! A brush selection is a presentation draft until release. Saving/switching
//! tools finishes its exact target; Escape simply drops it and fences old jobs.
use super::*;
use palette_core::{
    raster::Coverage,
    selection::{self, Combine, brush::Settings},
};
use std::sync::Arc;

pub struct BrushSelection {
    pub token: u64,
    pub generation: u64,
    pub base: Document,
    pub settings: Settings,
    pub mode: Combine,
    pub radius: f32,
    pub points: Vec<[f32; 2]>,
    pub released: bool,
    pub frame: Option<BrushFrame>,
    pub overlay: Option<TextureHandle>,
}
pub struct BrushFrame {
    pub samples: usize,
    pub coverage: Coverage,
    pub base: Option<Coverage>,
    pub contours: Arc<Vec<Vec<[f32; 2]>>>,
    pub feather: Arc<Vec<Vec<[f32; 2]>>>,
    pub bounds: Option<PixelRect>,
}
impl BrushFrame {
    pub fn compute(
        doc: &Document,
        raw: &Coverage,
        mode: Combine,
        radius: f32,
        samples: usize,
    ) -> Result<Self, String> {
        let mut result = doc.clone();
        selection::apply_region(&mut result, raw, mode, radius)?;
        let coverage = result.selection.expect("selection region");
        let base = result.selection_base;
        Ok(Self {
            samples,
            contours: Arc::new(palette_core::editing::contours(
                base.as_ref().unwrap_or(&coverage),
            )),
            feather: Arc::new(if base.is_some() {
                palette_core::editing::contours_at(&coverage, 1)
            } else {
                vec![]
            }),
            bounds: palette_core::editing::bounds(&coverage),
            coverage,
            base,
        })
    }
}
impl Session {
    pub fn cancel_brush_selection(&mut self) -> bool {
        self.selection_brush_draft.take().is_some()
    }
    /// Normal release waits for the exact worker frame. Explicit commands use a
    /// synchronous fallback when necessary, preserving the complete stroke.
    pub fn finish_brush_selection(&mut self) {
        let Some(draft) = self.selection_brush_draft.take() else {
            return;
        };
        if draft.generation != self.generation {
            return;
        }
        let exact = draft
            .frame
            .as_ref()
            .is_some_and(|f| f.samples == draft.points.len());
        let frame = match draft.frame.filter(|f| f.samples == draft.points.len()) {
            Some(frame) => Ok(frame),
            None => selection::brush::from_points(&draft.base, &draft.points, draft.settings)
                .and_then(|raw| {
                    BrushFrame::compute(
                        &draft.base,
                        &raw,
                        draft.mode,
                        draft.radius,
                        draft.points.len(),
                    )
                }),
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(error) => {
                self.gesture_error = Some(error);
                return;
            }
        };
        if self.document.selection.as_ref() == Some(&frame.coverage)
            && self.document.selection_base == frame.base
        {
            return;
        }
        self.history.begin("Brush selection", &self.document);
        self.document.selection = Some(frame.coverage);
        self.document.selection_base = frame.base;
        self.history.finish(&self.document);
        if self.selection_mode == Combine::Replace {
            self.selection_mode = Combine::Add;
        }
        self.invalidate();
        let mask = self
            .document
            .selection
            .as_ref()
            .expect("committed coverage");
        self.selection_cache = Some(SelectionCache {
            values: mask.values.clone(),
            geometry: (mask.width, mask.height, mask.enabled),
            base: self.document.selection_base.clone(),
            bounds: frame.bounds,
            contours: frame.contours,
            feather: frame.feather,
        });
        if exact && let Some(overlay) = draft.overlay {
            self.selection_brush_overlay = Some((self.generation, overlay));
        }
    }
}
