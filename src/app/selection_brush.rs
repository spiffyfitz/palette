//! A bounded, incremental worker keeps capsule rasterization, perceptual color
//! tests, feathering, contours and overlay conversion off the UI thread.
use super::*;
use crate::session::{BrushFrame, BrushSelection};
use palette_core::{
    raster::Coverage,
    selection::{
        Combine,
        brush::{Settings, Stroke},
    },
};

struct StrokeRequest {
    token: u64,
    base: Document,
    settings: Settings,
    mode: Combine,
    radius: f32,
    points: Vec<[f32; 2]>,
}
enum Work {
    Reset,
    Stroke(StrokeRequest),
    Overlay(Coverage),
}
struct Request {
    session: u64,
    generation: u64,
    max_side: usize,
    work: Work,
}
struct Ready {
    session: u64,
    generation: u64,
    token: u64,
    result: Result<(Option<BrushFrame>, ColorImage), String>,
}
pub(super) struct Worker {
    tx: mpsc::SyncSender<Request>,
    rx: mpsc::Receiver<Ready>,
    pub busy: bool,
    next_token: u64,
    cached_stroke: bool,
}

fn overlay_image(mask: &Coverage, max_side: usize) -> ColorImage {
    let step = (mask.width.max(mask.height) as usize).div_ceil(max_side.max(1)) as u32;
    let (w, h) = (mask.width.div_ceil(step), mask.height.div_ceil(step));
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let coverage = if mask.enabled {
                mask.values[((y * step) * mask.width + x * step) as usize]
            } else {
                255
            };
            rgba.extend_from_slice(&[53, 215, 225, (u32::from(coverage) * 90 / 255) as u8]);
        }
    }
    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba)
}
impl Worker {
    pub fn new(ctx: Context) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Request>(1);
        let (results, received) = mpsc::channel();
        std::thread::spawn(move || {
            let mut incremental: Option<(u64, usize, Stroke)> = None;
            while let Ok(request) = rx.recv() {
                if matches!(request.work, Work::Reset) {
                    incremental = None;
                    continue;
                }
                let token = match &request.work {
                    Work::Stroke(s) => s.token,
                    Work::Overlay(_) | Work::Reset => 0,
                };
                let result = (|| match request.work {
                    Work::Reset => unreachable!("handled before processing"),
                    Work::Overlay(mask) => Ok((None, overlay_image(&mask, request.max_side))),
                    Work::Stroke(job) => {
                        if incremental
                            .as_ref()
                            .is_none_or(|(token, _, _)| *token != job.token)
                        {
                            incremental = Some((
                                job.token,
                                1,
                                Stroke::new(&job.base, job.points[0], job.settings)?,
                            ));
                        }
                        let (_, done, stroke) = incremental.as_mut().expect("stroke initialized");
                        for point in &job.points[*done..] {
                            stroke.extend(*point)?;
                        }
                        *done = job.points.len();
                        let raw = stroke.coverage()?;
                        let frame =
                            BrushFrame::compute(&job.base, &raw, job.mode, job.radius, *done)?;
                        let image = overlay_image(&frame.coverage, request.max_side);
                        Ok((Some(frame), image))
                    }
                })();
                if result.is_err() {
                    incremental = None;
                }
                if results
                    .send(Ready {
                        session: request.session,
                        generation: request.generation,
                        token,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            tx,
            rx: received,
            busy: false,
            next_token: 1,
            cached_stroke: false,
        }
    }
}
impl PaletteApp {
    pub(crate) fn begin_brush_selection(&mut self, point: [f32; 2], mode: Combine) {
        self.current_mut().end_gesture();
        // Selection refinement starts from an actual selection. Alt-clicking
        // an empty document must not accidentally select almost the full canvas.
        if self.current().document.selection.is_none()
            && matches!(mode, Combine::Subtract | Combine::Intersect)
        {
            return;
        }
        let token = self.selection_brush_worker.next_token;
        self.selection_brush_worker.next_token += 1;
        let session = self.current_mut();
        session.selection_brush_draft = Some(BrushSelection {
            token,
            generation: session.generation,
            base: session.document.clone(),
            settings: session.selection_brush,
            mode,
            radius: session.selection_feather,
            points: vec![point],
            released: false,
            frame: None,
            overlay: None,
        });
    }
    pub(super) fn poll_selection_brush(&mut self, ctx: &Context) {
        while let Ok(ready) = self.selection_brush_worker.rx.try_recv() {
            self.selection_brush_worker.busy = false;
            let Some(session) = self
                .sessions
                .iter_mut()
                .find(|s| s.id == ready.session && s.generation == ready.generation)
            else {
                continue;
            };
            if ready.token != 0
                && !session
                    .selection_brush_draft
                    .as_ref()
                    .is_some_and(|d| d.token == ready.token)
            {
                continue;
            }
            match ready.result {
                Ok((frame, image)) => {
                    let texture = ctx.load_texture(
                        format!("selection-overlay-{}", session.id),
                        image,
                        TextureOptions::NEAREST,
                    );
                    if let Some(frame) = frame {
                        let draft = session
                            .selection_brush_draft
                            .as_mut()
                            .expect("matching gesture");
                        draft.overlay = Some(texture);
                        let complete = frame.samples == draft.points.len() && draft.released;
                        draft.frame = Some(frame);
                        if complete {
                            session.finish_brush_selection();
                        }
                    } else {
                        session.selection_brush_overlay = Some((session.generation, texture));
                    }
                }
                Err(error) => {
                    session.cancel_brush_selection();
                    self.error = Some(error);
                }
            }
        }
    }
    pub(super) fn request_selection_brush(&mut self, ctx: &Context) {
        if self.selection_brush_worker.busy {
            return;
        }
        if self.current().selection_brush_draft.is_none()
            && self.selection_brush_worker.cached_stroke
        {
            let request = Request {
                session: 0,
                generation: 0,
                max_side: 1,
                work: Work::Reset,
            };
            if self.selection_brush_worker.tx.try_send(request).is_ok() {
                self.selection_brush_worker.cached_stroke = false;
            }
            ctx.request_repaint();
            return;
        }
        let s = self.current();
        let work = if let Some(draft) = &s.selection_brush_draft {
            if draft
                .frame
                .as_ref()
                .is_some_and(|f| f.samples == draft.points.len())
            {
                return;
            }
            Work::Stroke(StrokeRequest {
                token: draft.token,
                base: draft.base.clone(),
                settings: draft.settings,
                mode: draft.mode,
                radius: draft.radius,
                points: draft.points.clone(),
            })
        } else if self.tool == Tool::SelectBrush
            && s.brush_overlay_enabled
            && let Some(mask) = &s.document.selection
            && s.selection_brush_overlay
                .as_ref()
                .is_none_or(|(generation, _)| *generation != s.generation)
        {
            Work::Overlay(mask.clone())
        } else {
            return;
        };
        let request = Request {
            session: s.id,
            generation: s.generation,
            max_side: ctx.input(|i| i.max_texture_side),
            work,
        };
        let stroke = matches!(request.work, Work::Stroke(_));
        if self.selection_brush_worker.tx.try_send(request).is_ok() {
            self.selection_brush_worker.busy = true;
            self.selection_brush_worker.cached_stroke |= stroke;
        } else {
            ctx.request_repaint();
        }
    }
}
