//! Pointer sampling stays in image coordinates. All expensive coverage work is
//! performed by the bounded worker, preserving the latest complete sample path.
use super::*;

impl PaletteApp {
    pub(super) fn brush_selection_interaction(
        &mut self,
        painter: &egui::Painter,
        canvas: Rect,
        pointer: Option<Pos2>,
        buttons: [bool; 2],
    ) {
        if self.tool != Tool::SelectBrush {
            return;
        }
        let point = pointer.map(|p| image_point(p, canvas, self.current().zoom));
        let capturing = self.is_capturing();
        if let Some(draft) = &mut self.current_mut().selection_brush_draft {
            if !draft.released && !capturing {
                if let Some(point) = point
                    && draft.points.last() != Some(&point)
                {
                    if draft.points.len() >= 16384 {
                        self.current_mut().cancel_brush_selection();
                        self.error=Some("Selection brush reached its 16384-sample stroke limit; start another stroke".into());
                        return;
                    }
                    draft.points.push(point);
                }
                if buttons[1] || !buttons[0] {
                    draft.released = true;
                }
            }
            if draft.released
                && draft
                    .frame
                    .as_ref()
                    .is_some_and(|f| f.samples == draft.points.len())
            {
                self.current_mut().finish_brush_selection();
            }
        }
        let session = self.current();
        if !session.brush_overlay_enabled {
            return;
        }
        let overlay = session
            .selection_brush_draft
            .as_ref()
            .and_then(|d| d.overlay.as_ref())
            .or_else(|| {
                session
                    .selection_brush_overlay
                    .as_ref()
                    .filter(|(generation, _)| {
                        *generation == session.generation && session.document.selection.is_some()
                    })
                    .map(|(_, texture)| texture)
            });
        if let Some(texture) = overlay {
            painter.image(
                texture.id(),
                canvas,
                Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
                Color32::WHITE,
            );
        }
        // A brush selection never invokes the move/resize selected-pixel handles.
        // The +/- cursor is drawn by the shared canvas brush cursor code.
    }
}
