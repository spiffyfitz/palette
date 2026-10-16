//! Centered, transactional Paint-style image and color dialogs.
use super::*;
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
        let color = match target {
            ColorTarget::Foreground => self.brush.color,
            ColorTarget::Background => self.background.to_srgba_unmultiplied(),
        };
        self.interface.color_editor =
            Some(ColorEditor::new(target, [color[0], color[1], color[2]]));
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
    fn colors_window(&mut self, ctx: &Context) {
        let Some(mut draft) = self.interface.color_editor.take() else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        let result = egui::Modal::new(egui::Id::unique("edit-colors"))
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(24))
            .backdrop_color(Color32::from_black_alpha(18))
            .show(ctx, |ui| {
                ui.set_width(592.0);
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 150.0).max(200.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.heading("Edit colors");
                        ui.add_space(16.0);
                        if draft.field.is_none() {
                            let pixels = (0..128)
                                .flat_map(|y| {
                                    (0..128).map(move |x| {
                                        Color32::from(egui::ecolor::Hsva::new(
                                            x as f32 / 127.0,
                                            1.0 - y as f32 / 127.0,
                                            1.0,
                                            1.0,
                                        ))
                                    })
                                })
                                .collect();
                            draft.field = Some(ctx.load_texture(
                                "color-spectrum",
                                ColorImage::new([128, 128], pixels),
                                TextureOptions::LINEAR,
                            ));
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;
                            color_field(ui, &mut draft);
                            ui.vertical(|ui| {
                                ui.set_width(150.0);
                                let response = ui.add_sized(
                                    vec2(125.0, 32.0),
                                    egui::TextEdit::singleline(&mut draft.hex).char_limit(7),
                                );
                                if response.changed() {
                                    if let Some(rgb) = parse_hex(&draft.hex) {
                                        draft.rgb = rgb;
                                        draft.hsv =
                                            Color32::from_rgb(rgb[0], rgb[1], rgb[2]).into();
                                    }
                                } else if !response.has_focus() {
                                    draft.hex = hex(draft.rgb);
                                }
                                ui.add_space(10.0);
                                ui.label("RGB");
                                for (index, label) in
                                    ["Red", "Green", "Blue"].into_iter().enumerate()
                                {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_sized(
                                                vec2(90.0, 32.0),
                                                egui::DragValue::new(&mut draft.rgb[index])
                                                    .range(0..=255),
                                            )
                                            .changed()
                                        {
                                            draft.hsv = Color32::from_rgb(
                                                draft.rgb[0],
                                                draft.rgb[1],
                                                draft.rgb[2],
                                            )
                                            .into();
                                            draft.hex = hex(draft.rgb);
                                        }
                                        ui.label(label);
                                    });
                                }
                            });
                        });
                        ui.add_space(26.0);
                        ui.horizontal(|ui| {
                            ui.label("Basic colors");
                            ui.add_space(260.0);
                            ui.label("Custom colors");
                            if ui
                                .small_button("+")
                                .on_hover_text("Add current color to custom colors")
                                .clicked()
                            {
                                if self.interface.custom_colors.len() == 20 {
                                    self.interface.custom_colors.remove(0);
                                }
                                self.interface.custom_colors.push(Color32::from_rgb(
                                    draft.rgb[0],
                                    draft.rgb[1],
                                    draft.rgb[2],
                                ));
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                for row in 0..4 {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().interact_size.y = 20.0;
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        for col in 0..12 {
                                            let color = if col == 11 {
                                                Color32::from_gray([0, 96, 192, 255][row])
                                            } else {
                                                Color32::from(egui::ecolor::Hsva::new(
                                                    col as f32 / 11.0,
                                                    [0.4, 0.65, 1.0, 1.0][row],
                                                    [1.0, 1.0, 1.0, 0.5][row],
                                                    1.0,
                                                ))
                                            };
                                            if color_dot(ui, color, 24.0, false).clicked() {
                                                set_color(&mut draft, color);
                                            }
                                        }
                                    });
                                }
                            });
                            ui.add_space(24.0);
                            ui.vertical(|ui| {
                                for row in 0..4 {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().interact_size.y = 20.0;
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        for col in 0..5 {
                                            if let Some(color) = self
                                                .interface
                                                .custom_colors
                                                .get(row * 5 + col)
                                                .copied()
                                            {
                                                if color_dot(ui, color, 24.0, false).clicked() {
                                                    set_color(&mut draft, color);
                                                }
                                            } else {
                                                empty_dot(ui, 24.0);
                                            }
                                        }
                                    });
                                }
                            });
                        });
                    });
                ui.add_space(22.0);
                ui.separator();
                ui.add_space(12.0);
                (apply, cancel) = dialog_buttons(ui, 592.0, parse_hex(&draft.hex).is_some());
            });
        if apply {
            match draft.target {
                ColorTarget::Foreground => {
                    self.brush.color = [draft.rgb[0], draft.rgb[1], draft.rgb[2], 255]
                }
                ColorTarget::Background => {
                    self.background = Color32::from_rgb(draft.rgb[0], draft.rgb[1], draft.rgb[2])
                }
            }
        } else if !cancel && !(result.is_top_modal && ctx.input(|i| i.key_pressed(Key::Escape))) {
            self.interface.color_editor = Some(draft);
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
fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 {
        return None;
    }
    u32::from_str_radix(text, 16)
        .ok()
        .map(|n| [(n >> 16) as u8, (n >> 8) as u8, n as u8])
}
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
fn set_color(draft: &mut ColorEditor, color: Color32) {
    draft.rgb = [color.r(), color.g(), color.b()];
    draft.hsv = color.into();
    draft.hex = hex(draft.rgb);
}
fn color_field(ui: &mut Ui, draft: &mut ColorEditor) {
    let (rect, response) = ui.allocate_exact_size(vec2(256.0, 256.0), Sense::click_and_drag());
    ui.painter().image(
        draft.field.as_ref().expect("spectrum").id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    let before = draft.hsv;
    if (response.clicked() || response.dragged())
        && let Some(point) = response.interact_pointer_pos()
    {
        draft.hsv.h = ((point.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        draft.hsv.s = (1.0 - (point.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    }
    let point = rect.min
        + vec2(
            draft.hsv.h * rect.width(),
            (1.0 - draft.hsv.s) * rect.height(),
        );
    ui.painter()
        .circle_stroke(point, 5.0, egui::Stroke::new(2.0, Color32::BLACK));
    ui.painter()
        .circle_stroke(point, 6.5, egui::Stroke::new(1.0, Color32::WHITE));
    let (preview, _) = ui.allocate_exact_size(vec2(24.0, 256.0), Sense::hover());
    ui.painter()
        .rect_filled(preview, 4, Color32::from(draft.hsv));
    let (bar, response) = ui.allocate_exact_size(vec2(14.0, 256.0), Sense::click_and_drag());
    for step in 0..64 {
        let v = 1.0 - step as f32 / 63.0;
        let color = Color32::from(egui::ecolor::Hsva::new(draft.hsv.h, draft.hsv.s, v, 1.0));
        ui.painter().rect_filled(
            egui::Rect::from_min_size(bar.min + vec2(0.0, step as f32 * 4.0), vec2(14.0, 4.0)),
            0,
            color,
        );
    }
    if (response.clicked() || response.dragged())
        && let Some(point) = response.interact_pointer_pos()
    {
        draft.hsv.v = (1.0 - (point.y - bar.top()) / bar.height()).clamp(0.0, 1.0);
    }
    let y = bar.top() + (1.0 - draft.hsv.v) * bar.height();
    ui.painter().circle_stroke(
        egui::pos2(bar.center().x, y),
        5.0,
        egui::Stroke::new(2.0, Color32::WHITE),
    );
    response.on_hover_text("Brightness; RGB and hexadecimal fields allow exact keyboard entry");
    if before != draft.hsv {
        let color = Color32::from(draft.hsv);
        draft.rgb = [color.r(), color.g(), color.b()];
        draft.hex = hex(draft.rgb);
    }
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
