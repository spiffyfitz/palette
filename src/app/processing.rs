//! Transactional processing controls. Preview jobs use reduced documents and
//! a cancellation ticket; only Apply mutates the original full-resolution model.
mod origin;
mod toolbar;
use super::*;
use palette_core::{
    document::EffectInstance,
    processing::{Kind, Operation, ParameterType, REGISTRY, Value},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Filters,
    Effects,
}
impl Section {
    fn effects(self) -> bool {
        self == Self::Effects
    }
    fn label(self) -> &'static str {
        if self.effects() { "Effects" } else { "Filters" }
    }
    fn icon(self) -> Icon {
        if self.effects() {
            Icon::Effects
        } else {
            Icon::Filters
        }
    }
}
/// The contextual bar is bound to one layer; tab/layer changes dismiss it.
pub(crate) struct Toolbar {
    session: u64,
    layer: u64,
    section: Section,
    selected: Option<u64>,
}

pub(super) struct Dialog {
    pub session: u64,
    pub generation: u64,
    pub layer: u64,
    pub instance: Option<u64>,
    pub operation: Operation,
    pub original: Document,
    pub preview: bool,
    pub ticket: u64,
    requested: Option<u64>,
    pub ready: Option<u64>,
    pub image: Option<TextureHandle>,
    pub(super) parameter: usize,
    pub(super) pick_origin: bool,
    pub(super) origin_drag: bool,
}
impl Dialog {
    fn document(&self) -> Result<Document, String> {
        let mut doc = self.original.clone();
        if doc.active_layer != self.layer {
            return Err("Processing layer changed".into());
        }
        match &self.operation {
            Operation::Filter(filter) => {
                if let Some(id) = self.instance {
                    doc.active_mut()
                        .filters
                        .iter_mut()
                        .find(|f| f.id == id)
                        .ok_or("Filter no longer exists")?
                        .filter = filter.clone();
                } else {
                    let id = doc.allocate_id();
                    doc.active_mut().filters.push(FilterInstance {
                        id,
                        enabled: true,
                        filter: filter.clone(),
                        mask: None,
                    });
                }
            }
            Operation::Effect(effect) => {
                if let Some(id) = self.instance {
                    doc.active_mut()
                        .effects
                        .iter_mut()
                        .find(|e| e.id == id)
                        .ok_or("Effect no longer exists")?
                        .effect = effect.clone();
                } else {
                    let id = doc.allocate_id();
                    doc.active_mut().effects.push(EffectInstance {
                        id,
                        enabled: true,
                        effect: effect.clone(),
                    });
                }
            }
        }
        doc.validate()?;
        Ok(doc)
    }
}
struct Request {
    ticket: u64,
    session: u64,
    generation: u64,
    document: Document,
}
struct Ready {
    ticket: u64,
    session: u64,
    generation: u64,
    result: Result<ColorImage, String>,
}
pub(super) struct Worker {
    tx: mpsc::SyncSender<Request>,
    rx: mpsc::Receiver<Ready>,
    pub busy: bool,
    pub ticket: Arc<AtomicU64>,
}
impl Worker {
    pub(super) fn new(ctx: Context) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Request>(1);
        let (results, received) = mpsc::channel();
        let ticket = Arc::new(AtomicU64::new(0));
        let worker_ticket = ticket.clone();
        std::thread::spawn(move || {
            let mut renderer = composite::Renderer::default();
            while let Ok(request) = rx.recv() {
                let cancelled = || worker_ticket.load(Ordering::Relaxed) != request.ticket;
                let result = (|| {
                    if cancelled() {
                        return Err("Processing cancelled".into());
                    }
                    let doc = palette_core::processing::preview_document(&request.document, 2048)?;
                    let pixels = renderer.render_cancel(&doc, &cancelled)?;
                    if cancelled() {
                        return Err("Processing cancelled".into());
                    }
                    Ok(texture_image(&pixels, 2048))
                })();
                if results
                    .send(Ready {
                        ticket: request.ticket,
                        session: request.session,
                        generation: request.generation,
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
            ticket,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum MaskAction {
    FromSelection,
    Remove,
    Toggle,
}
#[derive(Clone, Copy)]
pub(super) enum Action {
    Toggle(u64, bool),
    Move(u64, i32),
    Duplicate(u64),
    Delete(u64),
    Mask(u64, MaskAction),
}
impl PaletteApp {
    pub(crate) fn open_processing(&mut self, kind: Kind, instance: Option<u64>) {
        if !self.close_pending_none() {
            return;
        }
        self.current_mut().end_gesture();
        if self.current().document.active().locked {
            self.error = Some("Unlock the layer before editing filters or effects".into());
            return;
        }
        let mut operation = if let Some(id) = instance {
            let l = self.current().document.active();
            if let Some(f) = l.filters.iter().find(|f| f.id == id) {
                Operation::Filter(f.filter.clone())
            } else if let Some(e) = l.effects.iter().find(|e| e.id == id) {
                Operation::Effect(e.effect.clone())
            } else {
                return;
            }
        } else {
            kind.default_operation()
        };
        if instance.is_none()
            && matches!(
                operation.kind(),
                Kind::Clouds
                    | Kind::GradientMap
                    | Kind::DifferenceClouds
                    | Kind::Fibers
                    | Kind::Photocopy
                    | Kind::HalftonePattern
            )
        {
            let [r, g, b, _] = self.brush.color;
            operation
                .set_value("low", Value::Color([r, g, b]))
                .expect("captured color");
            let [r, g, b, _] = self.background.to_srgba_unmultiplied();
            operation
                .set_value("high", Value::Color([r, g, b]))
                .expect("captured color");
        }
        // egui colors store premultiplied bytes; filter colors are straight
        // RGB and retain the source layer's alpha independently.
        if instance.is_none() && matches!(kind, Kind::Pointillize | Kind::StainedGlass) {
            let rgba = if kind == Kind::Pointillize {
                self.background.to_srgba_unmultiplied()
            } else {
                self.brush.color
            };
            operation
                .set_value("color", Value::Color([rgba[0], rgba[1], rgba[2]]))
                .expect("captured color");
        }
        if instance.is_none() && kind == Kind::ColoredPencil {
            let [r, g, b, _] = self.background.to_srgba_unmultiplied();
            operation
                .set_value("paper", Value::Color([r, g, b]))
                .expect("captured paper color");
        }
        let kind = operation.kind();
        let ticket = self
            .processing_worker
            .ticket
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let session = self.current();
        self.processing_toolbar = Some(Toolbar {
            session: session.id,
            layer: session.document.active_layer,
            section: if matches!(operation, Operation::Effect(_)) {
                Section::Effects
            } else {
                Section::Filters
            },
            selected: instance,
        });
        let session = self.current();
        self.processing_dialog = Some(Dialog {
            session: session.id,
            generation: session.generation,
            layer: session.document.active_layer,
            instance,
            operation,
            pick_origin: true,
            origin_drag: false,
            original: session.document.clone(),
            preview: true,
            ticket,
            requested: None,
            ready: None,
            image: None,
            parameter: kind
                .descriptor()
                .parameters
                .iter()
                .position(|p| {
                    p.key
                        == match kind {
                            Kind::OuterGlow | Kind::InnerGlow => "radius",
                            Kind::DropShadow | Kind::InnerShadow => "angle",
                            Kind::Stroke => "width",
                            _ => "",
                        }
                })
                .unwrap_or(0),
        });
    }
    pub(super) fn cancel_processing(&mut self) {
        if self.processing_dialog.take().is_some() {
            self.processing_worker
                .ticket
                .fetch_add(1, Ordering::Relaxed);
        }
    }
    pub(super) fn poll_processing(&mut self, ctx: &Context) {
        if self.processing_dialog.as_ref().is_some_and(|d| {
            d.session != self.current().id
                || d.generation != self.current().generation
                || d.layer != self.current().document.active_layer
        }) {
            self.cancel_processing();
            self.processing_toolbar = None;
        }
        while let Ok(ready) = self.processing_worker.rx.try_recv() {
            self.processing_worker.busy = false;
            let Some(d) = &mut self.processing_dialog else {
                continue;
            };
            if d.ticket != ready.ticket
                || d.session != ready.session
                || d.generation != ready.generation
            {
                continue;
            }
            match ready.result {
                Ok(image) => {
                    let full_size =
                        image.size == [d.original.width as usize, d.original.height as usize];
                    d.image = Some(ctx.load_texture(
                        "processing-preview",
                        image,
                        if full_size {
                            TextureOptions::NEAREST
                        } else {
                            TextureOptions::LINEAR
                        },
                    ));
                    d.ready = Some(ready.ticket);
                }
                Err(error) => self.error = Some(error),
            }
        }
    }
    pub(super) fn request_processing(&mut self, ctx: &Context) {
        if self.processing_worker.busy {
            return;
        }
        let Some(d) = &self.processing_dialog else {
            return;
        };
        if d.requested == Some(d.ticket) {
            return;
        }
        let document = if d.preview {
            d.document()
        } else {
            Ok(d.original.clone())
        };
        match document {
            Ok(document) => {
                let request = Request {
                    ticket: d.ticket,
                    session: d.session,
                    generation: d.generation,
                    document,
                };
                if self.processing_worker.tx.try_send(request).is_ok() {
                    self.processing_worker.busy = true;
                    self.processing_dialog.as_mut().expect("dialog").requested = Some(d.ticket);
                } else {
                    ctx.request_repaint();
                }
            }
            Err(error) => {
                self.processing_dialog.as_mut().expect("dialog").requested = Some(d.ticket);
                self.error = Some(error);
            }
        }
    }
    pub(super) fn apply_processing(&mut self) {
        let Some(d) = self.processing_dialog.take() else {
            return;
        };
        self.processing_worker
            .ticket
            .fetch_add(1, Ordering::Relaxed);
        if self.current().id != d.session
            || self.current().generation != d.generation
            || self.current().document.active_layer != d.layer
        {
            self.message = "Processing cancelled because its layer/document changed".into();
            return;
        }
        match d.document() {
            Ok(document) => {
                if document != self.current().document {
                    let preview = (d.preview && d.ready == Some(d.ticket))
                        .then_some(d.image)
                        .flatten();
                    self.current_mut()
                        .command(d.operation.kind().descriptor().label, |doc| *doc = document);
                    if let Some(preview) = preview {
                        self.current_mut().texture = Some(preview);
                        self.current_mut().rendered_generation = None;
                    }
                    let node = if matches!(d.operation, Operation::Filter(_)) {
                        self.current()
                            .document
                            .active()
                            .filters
                            .last()
                            .map(|f| f.id)
                    } else {
                        self.current()
                            .document
                            .active()
                            .effects
                            .last()
                            .map(|e| e.id)
                    };
                    if let Some(toolbar) = &mut self.processing_toolbar {
                        toolbar.selected = d.instance.or(node);
                    }
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn processing_action(&mut self, effect: bool, action: Action) {
        if self.current().document.active().locked || !self.close_pending_none() {
            return;
        }
        self.current_mut().end_gesture();
        let mut doc = self.current().document.clone();
        let result = (|| {
            if effect {
                let id = match action {
                    Action::Toggle(id, _)
                    | Action::Move(id, _)
                    | Action::Duplicate(id)
                    | Action::Delete(id)
                    | Action::Mask(id, _) => id,
                };
                let index = doc
                    .active()
                    .effects
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or("Effect changed")?;
                match action {
                    Action::Toggle(_, enabled) => doc.active_mut().effects[index].enabled = enabled,
                    Action::Move(_, delta) => {
                        let to = (index as i32 + delta)
                            .clamp(0, doc.active().effects.len() as i32 - 1)
                            as usize;
                        doc.active_mut().effects.swap(index, to);
                    }
                    Action::Duplicate(_) => {
                        let mut node = doc.active().effects[index].clone();
                        node.id = doc.allocate_id();
                        doc.active_mut().effects.insert(index + 1, node);
                    }
                    Action::Delete(_) => {
                        doc.active_mut().effects.remove(index);
                    }
                    Action::Mask(..) => return Err("Effect masks are not implemented".into()),
                }
            } else {
                let id = match action {
                    Action::Toggle(id, _)
                    | Action::Move(id, _)
                    | Action::Duplicate(id)
                    | Action::Delete(id)
                    | Action::Mask(id, _) => id,
                };
                let index = doc
                    .active()
                    .filters
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or("Filter changed")?;
                match action {
                    Action::Toggle(_, enabled) => doc.active_mut().filters[index].enabled = enabled,
                    Action::Move(_, delta) => {
                        let to = (index as i32 + delta)
                            .clamp(0, doc.active().filters.len() as i32 - 1)
                            as usize;
                        doc.active_mut().filters.swap(index, to);
                    }
                    Action::Duplicate(_) => {
                        let mut node = doc.active().filters[index].clone();
                        node.id = doc.allocate_id();
                        doc.active_mut().filters.insert(index + 1, node);
                    }
                    Action::Delete(_) => {
                        doc.active_mut().filters.remove(index);
                    }
                    Action::Mask(_, MaskAction::FromSelection) => {
                        let saved = doc.active().mask.clone();
                        palette_core::selection::mask_from_selection(&mut doc)?;
                        let mask = doc.active().mask.clone();
                        doc.active_mut().mask = saved;
                        doc.active_mut().filters[index].mask = mask;
                    }
                    Action::Mask(_, MaskAction::Remove) => {
                        doc.active_mut().filters[index].mask = None
                    }
                    Action::Mask(_, MaskAction::Toggle) => {
                        if let Some(m) = &mut doc.active_mut().filters[index].mask {
                            m.enabled = !m.enabled;
                        }
                    }
                }
            }
            doc.validate()
        })();
        match result {
            Ok(()) => {
                if doc != self.current().document {
                    self.current_mut()
                        .command("Edit layer processing", |d| *d = doc);
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
}
