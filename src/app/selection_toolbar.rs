//! Tool settings are per document. The popup overlays the workspace without
//! shifting image coordinates; its measured rectangle also fences canvas input.
use super::*;
use palette_core::selection::Combine;

fn parameter_text(name: &str, value: f32) -> String {
    let text = format!("{value:.1}");
    if name == "Feather" && value < 5. {
        text
    } else {
        text.trim_end_matches('0').trim_end_matches('.').into()
    }
}

/// An ordinary editable field accompanies each slider. Incomplete/invalid text
/// never enters processing parameters and is restored when focus leaves it.
fn number(
    ui: &mut Ui,
    session: u64,
    name: &str,
    value: &mut f32,
    text: &mut String,
    max: f32,
    compact: bool,
) -> bool {
    let id = egui::Id::unique(("selection-number", session, name));
    if !ui.memory(|m| m.has_focus(id)) {
        *text = parameter_text(name, *value);
    }
    ui.label(name);
    let slider_active=ui.scope(|ui| {
        ui.spacing_mut().slider_width = if compact { 28. } else { 64. };
        let response=ui
            .add(
                crate::interface::selection_slider(value, max).show_value(false),
            ).on_hover_text(if name=="Tolerance" {"Higher tolerance includes more colors; 0 matches the sampled color, 100 includes all colors"} else {"Feather radius in image pixels"})
            ;
        if response.changed() {
            *text = parameter_text(name, *value);
        }
        response.is_pointer_button_down_on() || (response.has_focus() && ui.input(|i|[Key::ArrowLeft,Key::ArrowRight,Key::ArrowUp,Key::ArrowDown].iter().any(|key|i.key_down(*key))))
    }).inner;
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .id(id)
            .desired_width(36.)
            .align(egui::Align2::LEFT_CENTER),
    );
    if response.changed()
        && let Ok(parsed) = text.parse::<f32>()
        && parsed.is_finite()
        && (0. ..=max).contains(&parsed)
    {
        *value = (parsed * 10.).round() / 10.;
    }
    let finish =
        response.lost_focus() || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
    if finish {
        *text = parameter_text(name, *value);
        response.surrender_focus();
    }
    let active = slider_active || (response.has_focus() && !finish);
    response.on_hover_text(format!(
        "{name}: 0–{max}{}",
        if name == "Feather" {
            " image pixels; previews the current selection and sets new selections"
        } else {
            ""
        }
    ));
    active
}

impl PaletteApp {
    pub(crate) fn selection_toolbar(&mut self, ctx: &Context) {
        self.selection_toolbar_rect = egui::Rect::NOTHING;
        if self.processing_toolbar.is_some()
            || !matches!(
                self.tool,
                Tool::Select | Tool::Lasso | Tool::PolygonSelect | Tool::Wand | Tool::SelectBrush
            )
        {
            return;
        }
        let wand = self.tool == Tool::Wand;
        let brush = self.tool == Tool::SelectBrush;
        let before = self.current().selection_feather;
        let mut feather_active = false;
        let compact = self.canvas_rect.width() < 620.;
        let enabled = self.close_pending_none()
            && self.current().document.floating_shape.is_none()
            && !self.edit_pending
            && self.current().selection_drag.is_none()
            && self.current().selection_transform.is_none()
            && self.current().lasso_drag.is_none()
            && self.current().polygon_selection.is_none();
        let enabled = enabled && self.current().selection_brush_draft.is_none();
        let toolbar = egui::Area::new(egui::Id::unique("selection-toolbar"))
            .anchor(
                egui::Align2::CENTER_TOP,
                vec2(
                    self.canvas_rect.center().x - ctx.content_rect().center().x,
                    self.canvas_rect.top() + 4.,
                ),
            )
            .default_size(Vec2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // No fixed width: the frame tightly fits only this tool's controls.
                crate::interface::context_toolbar_frame(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_min_height(crate::interface::CONTEXT_CONTROL_HEIGHT);
                        ui.spacing_mut().item_spacing.x = if compact {4.} else {5.};
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.horizontal(|ui| {
                                ui.push_id(self.current().id, |ui| {
                                    let session = self.current_mut();
                                    if wand || brush {
                                        if brush {
                                            ui.add_enabled_ui(session.selection_brush.edge_aware, |ui| {
                                                number(ui,session.id,"Tolerance",&mut session.selection_brush.tolerance,&mut session.selection_brush_text,100.,compact);
                                            });
                                        } else {
                                        number(
                                            ui,
                                            session.id,
                                            "Tolerance",
                                            &mut session.wand.tolerance,
                                            &mut session.wand_tolerance_text,
                                            100.,
                                            compact,
                                        );
                                        }
                                        ui.separator();
                                    }
                                    feather_active=number(
                                        ui,
                                        session.id,
                                        "Feather",
                                        &mut session.selection_feather,
                                        &mut session.selection_feather_text,
                                        128.,
                                        compact,
                                    );
                                });
                                ui.separator();
                                for (mode, hint) in [
                                    (Combine::Replace, "Replace selection"),
                                    (Combine::Add, "Add to selection (Shift)"),
                                    (Combine::Subtract, "Subtract from selection (Alt)"),
                                    (Combine::Intersect, "Intersect selection (Shift+Alt)"),
                                ] {
                                    ui.push_id(mode.label(), |ui| {
                                        if icons::button(
                                            ui,
                                            Icon::Combine(mode),
                                            hint,
                                            self.current().selection_mode == mode,
                                            true,
                                            Vec2::splat(28.),
                                        )
                                        .clicked()
                                        {
                                            self.current_mut().selection_mode = mode;
                                        }
                                    });
                                }
                                ui.separator();
                                if icons::button(
                                    ui,
                                    Icon::InvertSelection,
                                    "Invert selection",
                                    false,
                                    !self.edit_pending,
                                    Vec2::splat(28.),
                                )
                                .clicked()
                                {
                                    self.invert_selection();
                                }
                                if brush {
                                    let response=ui.menu_button("...", |ui| {
                                        let session=self.current_mut();
                                        ui.checkbox(&mut session.selection_brush.edge_aware,"Color matching / edge aware").on_hover_text("Follows connected colors within the brush footprint; samples once at the start of each stroke");
                                        ui.add_enabled_ui(session.selection_brush.edge_aware,|ui| {
                                            ui.checkbox(&mut session.selection_brush.sample_merged,"Sample all visible layers");
                                        });
                                        ui.checkbox(&mut session.brush_overlay_enabled,"Selection overlay");
                                        ui.label("Size, strength and hardness: left sliders");
                                    }).response.on_hover_text("Selection brush options");
                                    self.capture_popup("brush-selection-options",&response);
                                } else if wand {
                                    ui.menu_button("...", |ui| {
                                        let settings = &mut self.current_mut().wand;
                                        ui.checkbox(&mut settings.contiguous, "Contiguous");
                                        ui.checkbox(
                                            &mut settings.sample_merged,
                                            "Sample all visible layers",
                                        );
                                        ui.checkbox(&mut settings.antialias, "Antialias edges");
                                    })
                                    .response
                                    .on_hover_text(if self.current().wand.sample_merged {"Samples all visible layers"} else {"Samples the active layer, including transparent pixels; enable Sample all visible layers to match the displayed composite"});
                                }
                            });
                        });
                    });
                });
            });
        self.selection_toolbar_rect = toolbar.response.rect;
        let radius = self.current().selection_feather;
        if radius != before {
            self.preview_selection_feather(radius, false, before);
        }
        let toolbar_preview = self
            .current()
            .feather_preview
            .as_ref()
            .is_some_and(|p| !p.dialog);
        if toolbar_preview && ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.stop_dragging();
            self.current_mut().cancel_selection_feather();
        } else if toolbar_preview && !feather_active && !self.is_capturing() {
            self.commit_selection_feather();
        }
    }
}
