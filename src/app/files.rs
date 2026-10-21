//! File workflows and recovery UI. Decoding, encoding and autosave run on
//! snapshots; completion is routed to the originating tab, never the active one.
use super::*;
use crate::persistence::Store;
use std::{sync::Arc, time::Instant};

pub(super) struct PaintExport {
    pub path: PathBuf,
    pub session: u64,
    pub revision: u64,
    pub backup_complete: bool,
}

impl PaletteApp {
    pub(super) fn init_store(&mut self, root: PathBuf) {
        match Store::new(root) {
            Ok(store) => {
                match store.recent() {
                    Ok(recent) => self.recent = recent,
                    Err(error) => {
                        self.error = Some(format!("Recent files could not be loaded: {error}"))
                    }
                }
                match store.discover() {
                    Ok(entries) => {
                        self.recovery_prompt = !entries.is_empty();
                        self.recoveries = entries;
                    }
                    Err(error) => {
                        self.error = Some(format!("Recovery entries could not be listed: {error}"))
                    }
                }
                self.store = Some(Arc::new(store));
            }
            Err(error) => self.error = Some(format!("Recovery storage is unavailable: {error}")),
        }
    }

    pub(super) fn remember(&mut self, path: &std::path::Path) {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.recent.retain(|p| *p != path);
        self.recent.insert(0, path);
        self.recent.truncate(crate::persistence::RECENT_LIMIT);
        if let Some(store) = &self.store
            && let Err(error) = store.save_recent(&self.recent)
        {
            self.error = Some(format!(
                "The file operation succeeded, but recent-file history could not be saved: {error}"
            ));
        }
    }
    pub(super) fn import_image(&mut self, path: Option<PathBuf>, ctx: &Context) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Import(path));
            return;
        }
        if self.edit_pending || self.pending_files >= 4 {
            self.message = "Wait for the current file/edit operation".into();
            return;
        }
        self.current_mut().end_gesture();
        let path = path.or_else(|| {
            rfd::FileDialog::new()
                .set_title("Import image as a new layer")
                .add_filter(
                    "Raster images",
                    &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"],
                )
                .pick_file()
        });
        let Some(path) = path else {
            return;
        };
        let session = self.current().id;
        let generation = self.current().generation;
        let tx = self.file_tx.clone();
        let repaint = ctx.clone();
        self.pending_files += 1;
        self.busy = true;
        std::thread::spawn(move || {
            let result = if matches!(io::extension(&path).as_str(), "palette" | "paint") {
                Err(
                    "Choose a raster image to import; open layered projects in their own tab"
                        .into(),
                )
            } else {
                io::open_with_report(&path).map_err(|e| e.to_string())
            };
            let _ = tx.send(FileResult::Imported {
                session,
                generation,
                path,
                result,
            });
            repaint.request_repaint();
        });
    }

    pub(super) fn forget_recovery(&mut self, id: u64) {
        if let Some(store) = &self.store
            && let Err(error) = store.forget(id)
        {
            self.error = Some(format!("Could not remove recovery copy: {error}"));
        }
        if let Some(entry) = self.restored.remove(&id)
            && let Err(error) = entry.discard()
        {
            self.error = Some(format!("Could not remove restored recovery copy: {error}"));
        }
        self.recovery_seen.remove(&id);
    }
    pub(super) fn clean_saved_recovery(&mut self, id: u64) {
        if self
            .sessions
            .iter()
            .find(|s| s.id == id)
            .is_none_or(|s| !s.modified())
        {
            self.forget_recovery(id);
        }
    }
    pub(super) fn autosave_tick(&mut self, ctx: &Context) {
        if self.store.is_none()
            || self.recovery_pending.is_some()
            || self.deferred_close
            || self.recovery_prompt
            || self.busy
            || self.edit_pending
            || self.text_editor.is_some()
        {
            return;
        }
        ctx.request_repaint_after(Duration::from_secs(5));
        if self.recovery_timer.elapsed() < Duration::from_secs(30) {
            return;
        }
        let Some(session) = self.sessions.iter().find(|s| {
            s.modified()
                && self.recovery_seen.get(&s.id) != Some(&s.history.revision)
                && s.paint_stroke.is_none()
                && s.selection_transform.is_none()
                && s.layer_drag.is_none()
                && s.construction.is_none()
                && s.selection_drag.is_none()
                && s.selection_brush_draft.is_none()
        }) else {
            self.recovery_timer = Instant::now();
            return;
        };
        let store = self.store.as_ref().expect("checked store").clone();
        let (id, revision) = (session.id, session.history.revision);
        let doc = session.document.clone();
        let name = session.name.clone();
        let original = session.path.clone();
        let tx = self.file_tx.clone();
        let repaint = ctx.clone();
        self.recovery_pending = Some(id);
        std::thread::spawn(move || {
            let result = store.autosave(id, &name, original, &doc);
            let _ = tx.send(FileResult::Autosaved {
                session: id,
                revision,
                result,
            });
            repaint.request_repaint();
        });
    }
    pub(super) fn recover(&mut self, index: usize, ctx: &Context) {
        if self.pending_files >= 4 {
            return;
        }
        let entry = self.recoveries.remove(index);
        self.recovery_prompt = !self.recoveries.is_empty();
        let tx = self.file_tx.clone();
        let repaint = ctx.clone();
        self.pending_files += 1;
        self.busy = true;
        std::thread::spawn(move || {
            let result = io::open(&entry.project_path()).map_err(|e| e.to_string());
            let _ = tx.send(FileResult::Recovered(entry, result));
            repaint.request_repaint();
        });
    }

    pub(super) fn handle_auxiliary_file_result(&mut self, result: FileResult, ctx: &Context) {
        match result {
            FileResult::Imported {
                session,
                generation,
                path,
                result,
            } => match result {
                Ok(image) => {
                    let Some(tab) = self
                        .sessions
                        .iter_mut()
                        .find(|s| s.id == session && s.generation == generation)
                    else {
                        self.message =
                            "Import cancelled because its document changed or closed".into();
                        return;
                    };
                    let mut doc = tab.document.clone();
                    match palette_core::editing::paste(
                        &mut doc,
                        image.document.active().source.clone(),
                    )
                    .and_then(|_| doc.validate())
                    {
                        Ok(()) => {
                            doc.active_mut().name = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .chars()
                                .take(64)
                                .collect();
                            tab.command("Import image as layer", |d| *d = doc);
                            if self.current().id == session {
                                self.choose_tool(Tool::Select);
                            }
                            self.remember(&path);
                            self.message = if let Some(notice) = image.notice {
                                format!("{notice} (imported as a layer)")
                            } else {
                                "Imported image as a layer".into()
                            };
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(error) => self.error = Some(error),
            },
            FileResult::Autosaved {
                session,
                revision,
                result,
            } => {
                self.recovery_pending = None;
                match result {
                    Ok(()) => {
                        self.recovery_seen.insert(session, revision);
                        // The restored original is retained until its replacement
                        // snapshot or an explicit user save has succeeded.
                        if let Some(entry) = self.restored.remove(&session)
                            && let Err(error) = entry.discard()
                        {
                            self.message = format!("Recovery cleanup failed: {error}");
                        }
                        self.clean_saved_recovery(session);
                    }
                    Err(error) => {
                        self.message = format!("Recovery copy failed: {error}");
                        self.recovery_timer = Instant::now();
                    }
                }
                if self.deferred_close {
                    self.deferred_close = false;
                    // Let the pending close reach the save prompt before a
                    // different dirty tab can start another recovery write.
                    self.recovery_timer = Instant::now();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            FileResult::Recovered(entry, result) => match result {
                Ok(doc) => {
                    self.current_mut().end_gesture();
                    let id = self.next_id;
                    self.next_id += 1;
                    let mut session = Session::new(id, entry.name.clone(), doc);
                    session.path = entry.original.clone();
                    session.saved_revision = u64::MAX;
                    self.sessions.push(session);
                    self.active = self.sessions.len() - 1;
                    self.restored.insert(id, entry);
                    self.recovery_timer = Instant::now()
                        .checked_sub(Duration::from_secs(30))
                        .unwrap_or_else(Instant::now);
                    self.message = "Recovered unsaved document; save the project to keep it".into();
                }
                Err(error) => {
                    self.recoveries.push(entry);
                    self.error = Some(error);
                }
            },
            _ => unreachable!("ordinary file result"),
        }
    }

    pub(super) fn file_windows(&mut self, ctx: &Context) {
        self.paint_export_window(ctx);
        if self.recovery_prompt && self.error.is_none() {
            let mut restore = None;
            let mut discard = None;
            egui::Modal::new(egui::Id::unique("recover-documents"))
                .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24))
                .show(ctx, |ui| {
                    ui.set_width(420.);
                    ui.heading("Recover unsaved documents");
                    ui.label("These recovery copies came from a previous session.");
                    egui::ScrollArea::vertical()
                        .max_height(320.)
                        .show(ui, |ui| {
                            for (index, entry) in self.recoveries.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add(egui::Label::new(&entry.name).truncate())
                                        .on_hover_text(entry.original.as_ref().map_or_else(
                                            || "Unsaved project".into(),
                                            |p| p.display().to_string(),
                                        ));
                                    if ui
                                        .add_enabled(
                                            self.pending_files < 4,
                                            egui::Button::new("Restore"),
                                        )
                                        .clicked()
                                    {
                                        restore = Some(index);
                                    }
                                    if ui.button("Discard").clicked() {
                                        discard = Some(index);
                                    }
                                });
                            }
                        });
                    if ui.button("Later").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        self.recovery_prompt = false;
                    }
                });
            if let Some(index) = restore {
                self.recover(index, ctx);
            }
            if let Some(index) = discard {
                match self.recoveries[index].discard() {
                    Ok(()) => {
                        self.recoveries.remove(index);
                        self.recovery_prompt = !self.recoveries.is_empty();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
    }

    pub(super) fn recent_menu(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(!self.recent.is_empty(), |ui| {
            ui.menu_button("Recent", |ui| {
                let mut chosen = None;
                for path in &self.recent {
                    if ui
                        .add_enabled(
                            path.is_file(),
                            egui::Button::new(
                                path.file_name().unwrap_or_default().to_string_lossy(),
                            ),
                        )
                        .on_hover_text(path.display().to_string())
                        .clicked()
                    {
                        chosen = Some(path.clone());
                    }
                }
                ui.separator();
                if ui.button("Clear recent files").clicked() {
                    self.recent.clear();
                    if let Some(store) = &self.store
                        && let Err(error) = store.save_recent(&[])
                    {
                        self.error = Some(error);
                    }
                    ui.close();
                }
                if let Some(path) = chosen {
                    self.open_path(path, ui.ctx());
                    ui.close();
                }
            });
        });
    }
}

impl PaletteApp {
    fn paint_export_window(&mut self, ctx: &Context) {
        if self.error.is_some() {
            return;
        }
        let Some(mut draft) = self.paint_export.take() else {
            return;
        };
        let Some(session) = self.sessions.iter().find(|s| s.id == draft.session) else {
            return;
        };
        if session.history.revision != draft.revision || self.current().id != draft.session {
            self.error = Some("The document changed; start the Paint export again".into());
            return;
        }
        let reasons = io::paint::limitations(&session.document);
        let needs_backup = !reasons.is_empty();
        let mut backup = false;
        let mut export = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("paint-export"))
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24)).show(ctx, |ui| {
                ui.set_width(430.); ui.heading("Export Microsoft Paint project");
                ui.label("Experimental: validate this copy in Microsoft Paint.");
                ui.label("The verified format preserves raster layers, offsets and alpha.");
                if needs_backup {
                    ui.separator(); ui.label("These properties cannot stay editable in this format:");
                    for reason in &reasons { ui.label(format!("• {reason}")); }
                    ui.label("Save the full .palette project first. Export then bakes the appearance into a separate compatible copy.");
                    if session.document.layers.iter().any(|l| l.blend != BlendMode::Normal) {
                        ui.label("Blended layers and their backdrop are merged; later layers remain separate.");
                    }
                }
                if draft.backup_complete { ui.label("Full .palette project saved."); }
                ui.add_space(12.);
                ui.horizontal(|ui| {
                    backup = ui.add_enabled(!self.busy, egui::Button::new("Save .palette first…")).clicked();
                    export = ui.add_enabled(!self.busy && (!needs_backup || draft.backup_complete),
                        egui::Button::new(egui::RichText::new("Export copy").color(Color32::BLACK)).fill(ACCENT)).clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                cancel |= ui.input(|i| i.key_pressed(Key::Escape));
            });
        if backup {
            self.active = self
                .sessions
                .iter()
                .position(|s| s.id == draft.session)
                .expect("export tab");
            // Keep the dialog alive while the ordinary atomic project save runs.
            draft.backup_complete = false;
            self.paint_export = Some(draft);
            self.save(ctx, true, false);
        } else if export {
            let document = session.document.clone();
            let id = session.id;
            let revision = session.history.revision;
            let tx = self.file_tx.clone();
            let repaint = ctx.clone();
            let path = draft.path;
            self.pending_files += 1;
            self.busy = true;
            std::thread::spawn(move || {
                let result = if needs_backup {
                    io::paint::compatible_copy(&document)
                } else {
                    Ok(document)
                }
                .and_then(|doc| io::paint::save_paint(&path, &doc))
                .map_err(|e| e.to_string());
                let _ = tx.send(FileResult::Saved {
                    id,
                    revision,
                    path,
                    export: true,
                    result,
                });
                repaint.request_repaint();
            });
        } else if !cancel {
            self.paint_export = Some(draft);
        }
    }
}
