use crate::{
    app::PaletteApp,
    icons::{self, Icon},
    session::Tool,
};
use eframe::egui::{self, Color32, RichText, Sense, Stroke, Ui, vec2};

fn group(ui: &mut Ui, width: f32, title: &str, contents: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.allocate_ui(vec2(width, 76.0), contents);
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(title)
                    .size(11.0)
                    .color(Color32::from_gray(207)),
            );
        });
    });
    ui.separator();
}

impl PaletteApp {
    fn transform_menu(&mut self, ui: &mut Ui) {
        use palette_core::transform::Orthogonal;
        ui.menu_button("v", |ui| {
            for (name, operation) in [
                ("Rotate 90° clockwise", Orthogonal::Clockwise),
                ("Rotate 90° counterclockwise", Orthogonal::CounterClockwise),
                ("Rotate 180°", Orthogonal::HalfTurn),
            ] {
                if ui
                    .add_enabled(!self.edit_pending, egui::Button::new(name))
                    .clicked()
                {
                    self.edit(crate::app::editing::Edit::Transform(operation), ui.ctx());
                    ui.close();
                }
            }
        });
        ui.add_space(4.0);
        ui.menu_button("v", |ui| {
            for (name, operation) in [
                ("Flip horizontal", Orthogonal::FlipHorizontal),
                ("Flip vertical", Orthogonal::FlipVertical),
            ] {
                if ui
                    .add_enabled(!self.edit_pending, egui::Button::new(name))
                    .clicked()
                {
                    self.edit(crate::app::editing::Edit::Transform(operation), ui.ctx());
                    ui.close();
                }
            }
        });
    }
    fn tool_button(&mut self, ui: &mut Ui, tool: Tool, icon: Icon, size: egui::Vec2) {
        if icons::button(ui, icon, tool.label(), self.tool == tool, true, size).clicked() {
            self.tool = tool;
        }
    }

    pub fn ribbon(&mut self, ui: &mut Ui) {
        egui::ScrollArea::horizontal()
            .id_salt("ribbon-scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    group(ui, 64.0, "Selection", |ui| {
                        ui.vertical_centered(|ui| {
                            self.tool_button(ui, Tool::Select, Icon::Select, vec2(40.0, 49.0));
                            ui.menu_button("v", |ui| {
                                if ui.button("Select all").clicked() {
                                    self.select_all();
                                    ui.close();
                                }
                                if ui.button("Deselect").clicked() {
                                    self.deselect();
                                    ui.close();
                                }
                                if ui.button("Invert selection").clicked() {
                                    self.invert_selection();
                                    ui.close();
                                }
                            });
                        });
                    });
                    group(ui, 138.0, "Image", |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                if icons::button(
                                    ui,
                                    Icon::Crop,
                                    "Crop canvas to selection",
                                    false,
                                    self.current().document.selection.is_some()
                                        && !self.edit_pending,
                                    vec2(32.0, 31.0),
                                )
                                .clicked()
                                {
                                    self.edit(crate::app::editing::Edit::Crop, ui.ctx());
                                }
                                if icons::button(
                                    ui,
                                    Icon::Resize,
                                    "Resize image…",
                                    false,
                                    !self.edit_pending,
                                    vec2(32.0, 31.0),
                                )
                                .clicked()
                                {
                                    self.show_resize();
                                }
                            });
                            ui.vertical(|ui| {
                                if icons::button(
                                    ui,
                                    Icon::Rotate,
                                    "Rotate image 90° clockwise",
                                    false,
                                    !self.edit_pending,
                                    vec2(32.0, 31.0),
                                )
                                .clicked()
                                {
                                    self.edit(
                                        crate::app::editing::Edit::Transform(
                                            palette_core::transform::Orthogonal::Clockwise,
                                        ),
                                        ui.ctx(),
                                    );
                                }
                                if icons::button(
                                    ui,
                                    Icon::Flip,
                                    "Flip image horizontally",
                                    false,
                                    !self.edit_pending,
                                    vec2(32.0, 31.0),
                                )
                                .clicked()
                                {
                                    self.edit(
                                        crate::app::editing::Edit::Transform(
                                            palette_core::transform::Orthogonal::FlipHorizontal,
                                        ),
                                        ui.ctx(),
                                    );
                                }
                            });
                            ui.vertical(|ui| {
                                self.transform_menu(ui);
                            });
                            if icons::button(
                                ui,
                                Icon::Resize,
                                "Resize image… (drag canvas corner for bounds only)",
                                false,
                                !self.edit_pending,
                                vec2(32.0, 42.0),
                            )
                            .clicked()
                            {
                                self.show_resize();
                            }
                        });
                    });
                    group(ui, 112.0, "Tools", |ui| {
                        ui.horizontal(|ui| {
                            self.tool_button(ui, Tool::Pencil, Icon::Pencil, vec2(30.0, 31.0));
                            self.tool_button(ui, Tool::Fill, Icon::Fill, vec2(30.0, 31.0));
                            icons::button(
                                ui,
                                Icon::Text,
                                "Text — planned for Phase 2",
                                false,
                                false,
                                vec2(30.0, 31.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            self.tool_button(ui, Tool::Eraser, Icon::Eraser, vec2(30.0, 31.0));
                            self.tool_button(ui, Tool::Picker, Icon::Picker, vec2(30.0, 31.0));
                            self.tool_button(ui, Tool::Magnifier, Icon::Zoom, vec2(30.0, 31.0));
                        });
                    });
                    group(ui, 58.0, "Brushes", |ui| {
                        ui.vertical_centered(|ui| {
                            self.tool_button(
                                ui,
                                Tool::Brush,
                                Icon::Brush,
                                vec2(42.0, if self.tool == Tool::Fill { 40.0 } else { 58.0 }),
                            );
                            if self.tool == Tool::Fill {
                                ui.add(
                                    egui::DragValue::new(&mut self.fill_tolerance).range(0..=255),
                                )
                                .on_hover_text("Fill tolerance: 0 = exact match, 255 = any color");
                            }
                        });
                    });
                    group(ui, 244.0, "Shapes", |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 1.0;
                            self.tool_button(ui, Tool::Line, Icon::Line, vec2(27.0, 27.0));
                            self.tool_button(
                                ui,
                                Tool::Rectangle,
                                Icon::Rectangle,
                                vec2(27.0, 27.0),
                            );
                            self.tool_button(ui, Tool::Ellipse, Icon::Ellipse, vec2(27.0, 27.0));
                            for icon in
                                [Icon::Curve, Icon::RoundRect, Icon::Pentagon, Icon::Hexagon]
                            {
                                icons::button(
                                    ui,
                                    icon,
                                    "Additional shape planned for Phase 2",
                                    false,
                                    false,
                                    vec2(27.0, 27.0),
                                );
                            }
                        });
                        ui.horizontal(|ui| {
                            for icon in [
                                Icon::Triangle,
                                Icon::Diamond,
                                Icon::Star,
                                Icon::ArrowRight,
                                Icon::ArrowDown,
                                Icon::Ellipse,
                                Icon::RoundRect,
                            ] {
                                icons::button(
                                    ui,
                                    icon,
                                    "Additional shape planned for Phase 2",
                                    false,
                                    false,
                                    vec2(27.0, 23.0),
                                );
                            }
                        });
                        ui.add_enabled_ui(self.tool.shape(), |ui| {
                            ui.checkbox(&mut self.shape_fill, "Fill shape");
                        });
                    });
                    group(ui, 324.0, "Colors", |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                let mut color = Color32::from_rgba_unmultiplied(
                                    self.brush.color[0],
                                    self.brush.color[1],
                                    self.brush.color[2],
                                    self.brush.color[3],
                                );
                                if ui
                                    .color_edit_button_srgba(&mut color)
                                    .on_hover_text("Foreground color")
                                    .changed()
                                {
                                    self.brush.color = color.to_srgba_unmultiplied();
                                }
                                ui.color_edit_button_srgba(&mut self.background)
                                    .on_hover_text(
                                        "Background color (right-click a swatch to set)",
                                    );
                            });
                            ui.vertical(|ui| {
                                let colors: [[u8; 3]; 24] = [
                                    [0, 0, 0],
                                    [128, 128, 128],
                                    [136, 0, 21],
                                    [237, 28, 36],
                                    [255, 127, 39],
                                    [255, 242, 0],
                                    [34, 177, 76],
                                    [0, 162, 232],
                                    [63, 72, 204],
                                    [163, 73, 164],
                                    [255, 255, 255],
                                    [195, 195, 195],
                                    [185, 122, 87],
                                    [255, 174, 201],
                                    [255, 201, 14],
                                    [239, 228, 176],
                                    [181, 230, 29],
                                    [153, 217, 234],
                                    [112, 146, 190],
                                    [200, 191, 231],
                                    [32, 32, 32],
                                    [64, 64, 64],
                                    [96, 96, 96],
                                    [160, 160, 160],
                                ];
                                for row in colors.chunks(12) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 3.0;
                                        for rgb in row {
                                            let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                            let (rect, response) = ui.allocate_exact_size(
                                                vec2(19.0, 22.0),
                                                Sense::click(),
                                            );
                                            ui.painter().circle_filled(rect.center(), 7.5, color);
                                            ui.painter().circle_stroke(
                                                rect.center(),
                                                7.5,
                                                Stroke::new(0.8, Color32::from_gray(130)),
                                            );
                                            if self.brush.color[..3] == rgb[..] {
                                                ui.painter().circle_stroke(
                                                    rect.center(),
                                                    9.0,
                                                    Stroke::new(1.0, crate::app::ACCENT),
                                                );
                                            }
                                            if response.clicked() {
                                                self.brush.color = [rgb[0], rgb[1], rgb[2], 255];
                                            }
                                            if response.secondary_clicked() {
                                                self.background = color;
                                            }
                                            response.on_hover_text(format!(
                                                "RGB {}, {}, {}",
                                                rgb[0], rgb[1], rgb[2]
                                            ));
                                        }
                                    });
                                }
                                ui.horizontal(|ui| {
                                    for _ in 0..10 {
                                        let (rect, _) = ui
                                            .allocate_exact_size(vec2(19.0, 16.0), Sense::hover());
                                        ui.painter().circle_stroke(
                                            rect.center(),
                                            7.0,
                                            Stroke::new(0.7, Color32::from_gray(100)),
                                        );
                                    }
                                });
                            });
                        });
                    });
                    group(ui, 59.0, "Effects", |ui| {
                        if icons::button(
                            ui,
                            Icon::Effects,
                            "Edit nondestructive filters",
                            self.show_filters,
                            true,
                            vec2(51.0, 62.0),
                        )
                        .clicked()
                        {
                            self.show_filters = !self.show_filters;
                        }
                    });
                    ui.vertical(|ui| {
                        if icons::button(
                            ui,
                            Icon::Layers,
                            "Show or hide layers",
                            self.show_layers,
                            true,
                            vec2(55.0, 76.0),
                        )
                        .clicked()
                        {
                            self.show_layers = !self.show_layers;
                        }
                        ui.label(RichText::new("Layers").size(11.0));
                    });
                });
            });
    }
}
