//! Centered, transactional Paint-style image and color dialogs.
use super::*;
mod colors;
use crate::interface::{CanvasProperties, ColorEditor, ColorTarget};

impl PaletteApp {
    pub(crate) fn show_canvas_properties(&mut self) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Properties);
            return;
        }
        self.current_mut().end_gesture();
        self.interface.canvas_properties = Some(CanvasProperties {
            session: self.current().id,
            generation: self.current().generation,
            width: self.current().document.width,
            height: self.current().document.height,
        });
    }
    pub(crate) fn edit_color(&mut self, target: ColorTarget) {
        if target == ColorTarget::CanvasBackground {
            if self.edit_pending || !self.finish_inline_text() {
                return;
            }
            self.cancel_processing();
            self.current_mut().end_gesture();
        }
        let color = match target {
            ColorTarget::Foreground => self.brush.color,
            ColorTarget::Background => self.background.to_srgba_unmultiplied(),
            ColorTarget::CanvasBackground => self.current().document.background,
        };
        let mut draft = ColorEditor::new(target, [color[0], color[1], color[2]]);
        if target == ColorTarget::CanvasBackground {
            draft.document = Some((self.current().id, self.current().generation));
        }
        self.interface.color_editor = Some(draft);
    }
    pub(super) fn appearance_windows(&mut self, ctx: &Context) {
        self.rotation_window(ctx);
        self.canvas_properties_window(ctx);
        self.colors_window(ctx);
    }
    fn canvas_properties_window(&mut self, ctx: &Context) {
        let Some(mut draft) = self.interface.canvas_properties.take() else {
            return;
        };
        if draft.session != self.current().id || draft.generation != self.current().generation {
            return;
        }
        let mut apply = false;
        let mut cancel = false;
        let result = egui::Modal::new(egui::Id::unique("canvas-properties"))
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24))
            .backdrop_color(Color32::from_black_alpha(18))
            .show(ctx, |ui| {
                ui.set_width(480.0);
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 150.0).max(200.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.heading("Image Properties");
                        ui.add_space(22.0);
                        ui.strong("File attributes");
                        ui.add_space(12.0);
                        egui::Grid::new("file-attributes")
                            .min_col_width(230.0)
                            .spacing(vec2(12.0, 14.0))
                            .show(ui, |ui| {
                                ui.label("Project");
                                ui.add(egui::Label::new(self.current().name.clone()).truncate());
                                ui.end_row();
                                ui.label("Layers");
                                ui.label(self.current().document.layers.len().to_string());
                                ui.end_row();
                                ui.label("Color space");
                                ui.label("sRGB (assumed)");
                                ui.end_row();
                            });
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(12.0);
                        ui.strong("Units");
                        ui.add_space(10.0);
                        ui.add_enabled(false, egui::RadioButton::new(false, "Inches"))
                            .on_hover_text("Print resolution is not yet tracked");
                        ui.add_enabled(false, egui::RadioButton::new(false, "Centimeters"))
                            .on_hover_text("Print resolution is not yet tracked");
                        let _ = ui.radio(true, "Pixels");
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(12.0);
                        ui.strong("Image size");
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("Width");
                                ui.add_sized(
                                    vec2(150.0, 32.0),
                                    egui::DragValue::new(&mut draft.width).range(1..=16384),
                                );
                            });
                            ui.add_space(16.0);
                            ui.vertical(|ui| {
                                ui.label("Height");
                                ui.add_sized(
                                    vec2(150.0, 32.0),
                                    egui::DragValue::new(&mut draft.height).range(1..=16384),
                                );
                            });
                            ui.vertical(|ui| {
                                ui.label(" ");
                                if ui
                                    .add_sized(vec2(90.0, 32.0), egui::Button::new("Default"))
                                    .clicked()
                                {
                                    draft.width = crate::interface::DEFAULT_CANVAS[0];
                                    draft.height = crate::interface::DEFAULT_CANVAS[1];
                                }
                            });
                        });
                    });
                ui.add_space(22.0);
                ui.separator();
                ui.add_space(12.0);
                (apply, cancel) = dialog_buttons(ui, 480.0, true);
            });
        if apply {
            self.edit(editing::Edit::Canvas(draft.width, draft.height), ctx);
        } else if !cancel && !(result.is_top_modal && ctx.input(|i| i.key_pressed(Key::Escape))) {
            self.interface.canvas_properties = Some(draft);
        }
    }
}
fn dialog_buttons(ui: &mut Ui, width: f32, valid: bool) -> (bool, bool) {
    ui.horizontal(|ui| {
        let width = (width - ui.spacing().item_spacing.x) * 0.5;
        let apply = ui
            .add_enabled(
                valid,
                egui::Button::new(egui::RichText::new("OK").color(Color32::BLACK))
                    .fill(ACCENT)
                    .min_size(vec2(width, 32.0)),
            )
            .clicked();
        let cancel = ui
            .add_sized(vec2(width, 32.0), egui::Button::new("Cancel"))
            .clicked();
        (apply, cancel)
    })
    .inner
}
pub(crate) fn color_dot(ui: &mut Ui, color: Color32, size: f32, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let radius = size * 0.34;
    ui.painter().circle_filled(rect.center(), radius, color);
    ui.painter().circle_stroke(
        rect.center(),
        radius,
        egui::Stroke::new(0.8, Color32::from_gray(130)),
    );
    if selected || response.hovered() {
        ui.painter().circle_stroke(
            rect.center(),
            radius + 3.0,
            egui::Stroke::new(
                1.0,
                if selected {
                    ACCENT
                } else {
                    Color32::LIGHT_GRAY
                },
            ),
        );
    }
    response
}
pub(crate) fn empty_dot(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().circle_stroke(
        rect.center(),
        size * 0.34,
        egui::Stroke::new(0.8, Color32::from_gray(90)),
    );
}
