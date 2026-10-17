//! One bounded feather worker coalesces slider changes. Results carry both tab
//! generation and a gesture token; cancelled or superseded gestures cannot win.
use super::*;
use crate::session::{FeatherFrame, SelectionFeather};
use palette_core::raster::Coverage;

struct Request {
    session: u64,
    generation: u64,
    token: u64,
    source: Coverage,
    radius: f32,
}
struct ResultFrame {
    session: u64,
    generation: u64,
    token: u64,
    frame: Result<FeatherFrame, String>,
}
pub(super) struct Worker {
    tx: mpsc::SyncSender<Request>,
    rx: mpsc::Receiver<ResultFrame>,
    pub busy: bool,
    next_token: u64,
}
impl Worker {
    pub fn new(ctx: Context) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Request>(1);
        let (results, received) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(request) = rx.recv() {
                let frame = FeatherFrame::compute(&request.source, request.radius);
                if results
                    .send(ResultFrame {
                        session: request.session,
                        generation: request.generation,
                        token: request.token,
                        frame,
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
        }
    }
}
impl PaletteApp {
    pub(super) fn preview_selection_feather(
        &mut self,
        radius: f32,
        dialog: bool,
        setting_before: f32,
    ) {
        if self.current().document.selection.is_none() {
            return;
        }
        if self
            .current()
            .feather_preview
            .as_ref()
            .is_some_and(|p| p.dialog != dialog || p.generation != self.current().generation)
        {
            self.current_mut().end_gesture();
        }
        if let Some(preview) = &mut self.current_mut().feather_preview {
            if preview.radius != radius {
                preview.radius = radius;
                preview.commit_requested = false;
            }
            return;
        }
        self.current_mut().end_gesture();
        let token = self.feather_worker.next_token;
        self.feather_worker.next_token += 1;
        let session = self.current_mut();
        let outline = session.selection_outline();
        let source = session
            .document
            .selection_base
            .as_ref()
            .or(session.document.selection.as_ref())
            .expect("selection")
            .clone();
        session.feather_preview = Some(SelectionFeather {
            token,
            generation: session.generation,
            source,
            outline,
            radius,
            setting_before,
            dialog,
            commit_requested: false,
            frame: None,
        });
    }
    pub(super) fn commit_selection_feather(&mut self) {
        if let Some(preview) = &mut self.current_mut().feather_preview {
            preview.commit_requested = true;
            if preview
                .frame
                .as_ref()
                .is_some_and(|f| f.radius == preview.radius)
            {
                self.current_mut().finish_selection_feather();
            }
        }
    }
    pub(super) fn poll_feather(&mut self) {
        while let Ok(result) = self.feather_worker.rx.try_recv() {
            self.feather_worker.busy = false;
            let Some(session) = self
                .sessions
                .iter_mut()
                .find(|s| s.id == result.session && s.generation == result.generation)
            else {
                continue;
            };
            let Some(preview) = session
                .feather_preview
                .as_mut()
                .filter(|p| p.token == result.token)
            else {
                continue;
            };
            match result.frame {
                Ok(frame) => {
                    preview.frame = Some(frame);
                    if preview.commit_requested
                        && preview.frame.as_ref().unwrap().radius == preview.radius
                    {
                        session.finish_selection_feather();
                    }
                }
                Err(error) => {
                    session.cancel_selection_feather();
                    self.error = Some(error);
                }
            }
        }
    }
    pub(super) fn request_feather(&mut self) {
        if self.feather_worker.busy {
            return;
        }
        let session = self.current();
        let Some(preview) = session.feather_preview.as_ref() else {
            return;
        };
        if preview
            .frame
            .as_ref()
            .is_some_and(|f| f.radius == preview.radius)
        {
            return;
        }
        let request = Request {
            session: session.id,
            generation: preview.generation,
            token: preview.token,
            source: preview.source.clone(),
            radius: preview.radius,
        };
        if self.feather_worker.tx.try_send(request).is_ok() {
            self.feather_worker.busy = true;
        }
    }
}
