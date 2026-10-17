//! Compact Phase 4 controls; settings are per tab, processing uses edit workers.
use super::*;
use palette_core::selection::Combine;

pub(super) struct FeatherDialog {
    pub session: u64,
    pub generation: u64,
    pub radius: f32,
    pub mask: bool,
}
impl PaletteApp {
    pub(crate) fn selection_mode(&self, ctx: &Context) -> Combine {
        let (shift, alt) = ctx.input(|i| (i.modifiers.shift, i.modifiers.alt));
        match (shift, alt) {
            (true, true) => Combine::Intersect,
            (true, false) => Combine::Add,
            (false, true) => Combine::Subtract,
            _ => self.current().selection_mode,
        }
    }
    pub(crate) fn selection_options(&mut self, ui: &mut Ui) {
        ui.menu_button("Combine selection", |ui| {
            for (v, shortcut) in [
                (Combine::Replace, ""),
                (Combine::Add, "Shift"),
                (Combine::Subtract, "Alt"),
                (Combine::Intersect, "Shift+Alt"),
            ] {
                let selected = self.current().selection_mode == v;
                if ui
                    .add(
                        egui::Button::new(v.label())
                            .selected(selected)
                            .shortcut_text(shortcut),
                    )
                    .clicked()
                {
                    self.current_mut().selection_mode = v;
                    ui.close();
                }
            }
        });
        if self.tool == Tool::Wand {
            let response = ui.menu_button("Magic Wand options", |ui| {
                ui.set_min_width(230.);
                let options = &mut self.current_mut().wand;
                ui.add(
                    crate::interface::selection_slider(&mut options.tolerance, 100.)
                        .text("Tolerance"),
                );
                ui.checkbox(&mut options.contiguous, "Contiguous");
                ui.checkbox(&mut options.sample_merged, "Sample all visible layers");
                ui.checkbox(&mut options.antialias, "Antialias edges");
            });
            self.capture_submenu("wand-settings", ui, &response.response);
        }
    }
    pub(crate) fn show_feather(&mut self, mask: bool) {
        self.current_mut().end_gesture();
        if self.text_editor.is_some() || self.edit_pending {
            return;
        }
        self.feather_dialog = Some(FeatherDialog {
            session: self.current().id,
            generation: self.current().generation,
            radius: 4.,
            mask,
        });
    }
    pub(super) fn feather_window(&mut self, ctx: &Context) {
        if self.error.is_some() {
            return;
        }
        let Some(mut draft) = self.feather_dialog.take() else {
            return;
        };
        if self.current().id != draft.session || self.current().generation != draft.generation {
            if let Some(session) = self.sessions.iter_mut().find(|s| s.id == draft.session) {
                session.cancel_selection_feather();
            }
            self.message = "Feather cancelled because its document changed".into();
            return;
        }
        let (mut apply, mut cancel) = (false, false);
        egui::Modal::new(egui::Id::unique("feather-coverage"))
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24))
            .show(ctx, |ui| {
                ui.set_width(300.);
                ui.heading(if draft.mask {
                    "Feather layer mask"
                } else {
                    "Feather selection"
                });
                ui.add_space(8.);
                let decimals = usize::from(draft.radius < 5.);
                ui.add(
                    crate::interface::selection_slider(&mut draft.radius, 128.)
                        .min_decimals(decimals)
                        .text("Radius (px)"),
                );
                ui.label("Softens coverage edges. Original image pixels are preserved.");
                ui.add_space(12.);
                ui.horizontal(|ui| {
                    apply = ui
                        .add(
                            egui::Button::new(egui::RichText::new("Apply").color(Color32::BLACK))
                                .fill(ACCENT),
                        )
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                cancel |= ui.input(|i| i.key_pressed(Key::Escape));
            });
        if !draft.mask && !cancel {
            self.preview_selection_feather(draft.radius, true, self.current().selection_feather);
        }
        if apply && !draft.mask {
            self.commit_selection_feather();
        } else if apply {
            self.edit(
                editing::Edit::Feather {
                    radius: draft.radius,
                    mask: draft.mask,
                },
                ctx,
            );
        } else if cancel {
            self.current_mut().cancel_selection_feather();
        } else {
            self.feather_dialog = Some(draft);
        }
    }
    pub(super) fn mask_properties(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Layer mask");
        let available = !self.current().document.active().locked && !self.edit_pending;
        ui.add_enabled_ui(available, |ui| {
            if let Some(mask) = self.current().document.active().mask.as_ref() {
                let mut enabled = mask.enabled;
                let (rect, _) = ui.allocate_exact_size(vec2(64., 40.), Sense::hover());
                // A distinct grayscale preview identifies coverage rather than
                // confusing the original-content thumbnail with masked output.
                for y in 0..10 {
                    for x in 0..16 {
                        let value = mask.values
                            [((y * mask.height / 10) * mask.width + x * mask.width / 16) as usize];
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                rect.min + vec2(x as f32 * 4., y as f32 * 4.),
                                vec2(4., 4.),
                            ),
                            0,
                            Color32::from_gray(value),
                        );
                    }
                }
                if ui.checkbox(&mut enabled, "Enabled").changed() {
                    self.edit(editing::Edit::ToggleMask, ui.ctx());
                }
                if ui.button("Invert mask").clicked() {
                    self.edit(editing::Edit::InvertMask, ui.ctx());
                }
                if ui.button("Feather mask…").clicked() {
                    self.show_feather(true);
                }
                if ui
                    .button("Delete mask")
                    .on_hover_text("Removes coverage; source pixels stay intact")
                    .clicked()
                {
                    self.edit(editing::Edit::DeleteMask, ui.ctx());
                }
            } else {
                let label = if self.current().document.selection.is_some() {
                    "Mask from selection"
                } else {
                    "Add reveal-all mask"
                };
                if ui.button(label).clicked() {
                    self.edit(editing::Edit::MaskFromSelection, ui.ctx());
                }
            }
        });
    }
}
