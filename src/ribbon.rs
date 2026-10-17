//! Compact Paint ribbon with control-anchored flyouts.
use crate::{
    app::PaletteApp,
    icons::{self, Icon},
    interface::{self, ColorTarget, PALETTE},
    session::Tool,
};
use eframe::egui::{self, Color32, RichText, Sense, Stroke, Ui, vec2};
use palette_core::shapes::Kind;
/// Paint reference order: three complete seven-column rows, then heart/lightning.
/// Every entry now has a functional raster or multi-stage construction tool.
pub(crate) const SHAPE_GALLERY: [(Icon, Option<Tool>); 23] = [
    (Icon::Line, Some(Tool::Line)),
    (Icon::Curve, Some(Tool::Curve)),
    (Icon::Ellipse, Some(Tool::Ellipse)),
    (Icon::Rectangle, Some(Tool::Rectangle)),
    (
        Icon::Shape(Kind::RoundedRectangle),
        Some(Tool::Shape(Kind::RoundedRectangle)),
    ),
    (Icon::Polygon, Some(Tool::Polygon)),
    (
        Icon::Shape(Kind::Triangle),
        Some(Tool::Shape(Kind::Triangle)),
    ),
    (
        Icon::Shape(Kind::RightTriangle),
        Some(Tool::Shape(Kind::RightTriangle)),
    ),
    (Icon::Shape(Kind::Diamond), Some(Tool::Shape(Kind::Diamond))),
    (
        Icon::Shape(Kind::Pentagon),
        Some(Tool::Shape(Kind::Pentagon)),
    ),
    (Icon::Shape(Kind::Hexagon), Some(Tool::Shape(Kind::Hexagon))),
    (
        Icon::Shape(Kind::RightArrow),
        Some(Tool::Shape(Kind::RightArrow)),
    ),
    (
        Icon::Shape(Kind::LeftArrow),
        Some(Tool::Shape(Kind::LeftArrow)),
    ),
    (Icon::Shape(Kind::UpArrow), Some(Tool::Shape(Kind::UpArrow))),
    (
        Icon::Shape(Kind::DownArrow),
        Some(Tool::Shape(Kind::DownArrow)),
    ),
    (
        Icon::Shape(Kind::FourPointStar),
        Some(Tool::Shape(Kind::FourPointStar)),
    ),
    (Icon::Shape(Kind::Star), Some(Tool::Shape(Kind::Star))),
    (
        Icon::Shape(Kind::SixPointStar),
        Some(Tool::Shape(Kind::SixPointStar)),
    ),
    (
        Icon::Shape(Kind::RectangularCallout),
        Some(Tool::Shape(Kind::RectangularCallout)),
    ),
    (
        Icon::Shape(Kind::OvalCallout),
        Some(Tool::Shape(Kind::OvalCallout)),
    ),
    (
        Icon::Shape(Kind::CloudCallout),
        Some(Tool::Shape(Kind::CloudCallout)),
    ),
    (Icon::Shape(Kind::Heart), Some(Tool::Shape(Kind::Heart))),
    (
        Icon::Shape(Kind::Lightning),
        Some(Tool::Shape(Kind::Lightning)),
    ),
];
fn group(ui: &mut Ui, width: f32, title: &str, contents: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.allocate_ui(vec2(width, 76.0), |ui| {
            ui.set_min_height(76.0);
            contents(ui);
        });
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
    fn tool_button(&mut self, ui: &mut Ui, tool: Tool, icon: Icon, size: egui::Vec2) {
        if icons::button(ui, icon, tool.label(), self.tool == tool, true, size).clicked() {
            self.choose_tool(tool);
        }
    }
    fn selection_control(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            let tool = if matches!(
                self.tool,
                Tool::MoveLayer | Tool::Wand | Tool::Lasso | Tool::SelectBrush
            ) {
                self.tool
            } else {
                Tool::Select
            };
            self.tool_button(
                ui,
                tool,
                if tool == Tool::MoveLayer {
                    Icon::Move
                } else if tool == Tool::Wand {
                    Icon::Wand
                } else if tool == Tool::Lasso {
                    Icon::Lasso
                } else if tool == Tool::SelectBrush {
                    Icon::SelectBrush
                } else {
                    Icon::Select
                },
                vec2(40.0, 36.0),
            );
            let response = interface::chevron(ui, vec2(40.0, 18.0), "Selection options");
            egui::Popup::menu(&response)
                .style(interface::menu_style)
                .show(|ui| {
                    ui.set_min_width(230.);
                    if interface::menu_item(
                        ui,
                        "Rectangular selection",
                        "",
                        Some(Icon::Select),
                        true,
                    )
                    .clicked()
                    {
                        self.choose_tool(Tool::Select);
                        ui.close();
                    }
                    if interface::menu_item(ui, "Move active layer", "", Some(Icon::Move), true)
                        .clicked()
                    {
                        self.choose_tool(Tool::MoveLayer);
                        ui.close();
                    }
                    if interface::menu_item(ui, "Freeform selection", "", Some(Icon::Lasso), true)
                        .clicked()
                    {
                        self.choose_tool(Tool::Lasso);
                        ui.close();
                    }
                    if interface::menu_item(ui, "Magic Wand", "", Some(Icon::Wand), true).clicked()
                    {
                        self.choose_tool(Tool::Wand);
                        ui.close();
                    }
                    if interface::menu_item(
                        ui,
                        "Selection brush",
                        "",
                        Some(Icon::SelectBrush),
                        true,
                    )
                    .clicked()
                    {
                        self.choose_tool(Tool::SelectBrush);
                        ui.close();
                    }
                    self.selection_options(ui);
                    ui.separator();
                    if interface::menu_item(ui, "Select all", "Ctrl+A", None, true).clicked() {
                        self.select_all();
                        ui.close();
                    }
                    if interface::menu_item(ui, "Deselect", "Ctrl+D", None, true).clicked() {
                        self.deselect();
                        ui.close();
                    }
                    if interface::menu_item(ui, "Invert selection", "", None, true).clicked() {
                        self.invert_selection();
                        ui.close();
                    }
                    if interface::menu_item(
                        ui,
                        "Feather selection…",
                        "",
                        None,
                        self.current().document.selection.is_some()
                            && self.close_pending_none()
                            && !self.edit_pending,
                    )
                    .clicked()
                    {
                        self.show_feather(false);
                        ui.close();
                    }
                    if interface::menu_item(
                        ui,
                        "Mask from selection",
                        "",
                        None,
                        self.current().document.selection.is_some()
                            && self.current().document.active().mask.is_none()
                            && !self.current().document.active().locked
                            && !self.edit_pending,
                    )
                    .clicked()
                    {
                        self.edit(crate::app::editing::Edit::MaskFromSelection, ui.ctx());
                        ui.close();
                    }
                });
            self.capture_popup("selection-options", &response);
            self.capture_popup("wand-settings", &response);
        });
    }
    fn transform_control(&mut self, ui: &mut Ui, flip: bool) {
        use palette_core::transform::Orthogonal as O;
        ui.horizontal(|ui| {
            let icon = if flip { Icon::Flip } else { Icon::Rotate };
            let tooltip = if flip {
                "Flip horizontally"
            } else {
                "Rotate 90° clockwise"
            };
            if icons::button(
                ui,
                icon,
                tooltip,
                false,
                !self.edit_pending,
                vec2(30.0, 30.0),
            )
            .clicked()
            {
                self.edit(
                    crate::app::editing::Edit::Transform(if flip {
                        O::FlipHorizontal
                    } else {
                        O::Clockwise
                    }),
                    ui.ctx(),
                );
            }
            let response = interface::chevron(
                ui,
                vec2(18.0, 30.0),
                if flip {
                    "Flip options"
                } else {
                    "Rotate options"
                },
            );
            egui::Popup::menu(&response)
                .style(interface::menu_style)
                .show(|ui| {
                    let operations: &[(&str, O)] = if flip {
                        &[
                            ("Flip horizontal", O::FlipHorizontal),
                            ("Flip vertical", O::FlipVertical),
                        ]
                    } else {
                        &[
                            ("Rotate right 90°", O::Clockwise),
                            ("Rotate left 90°", O::CounterClockwise),
                            ("Rotate 180°", O::HalfTurn),
                        ]
                    };
                    for &(name, operation) in operations {
                        if interface::menu_item(ui, name, "", Some(icon), !self.edit_pending)
                            .clicked()
                        {
                            self.edit(crate::app::editing::Edit::Transform(operation), ui.ctx());
                            ui.close();
                        }
                    }
                    if !flip
                        && interface::menu_item(
                            ui,
                            "Rotate custom…",
                            "",
                            Some(icon),
                            !self.edit_pending,
                        )
                        .clicked()
                    {
                        self.show_rotation();
                        ui.close();
                    }
                });
        });
    }
    fn brushes_control(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            self.tool_button(ui, Tool::Brush, Icon::Brush, vec2(40.0, 40.0));
            let response = interface::chevron(ui, vec2(40.0, 18.0), "Brushes");
            self.capture_popup("brushes", &response);
            egui::Popup::menu(&response)
                .style(interface::menu_style)
                .show(|ui| {
                    ui.set_width(236.0);
                    for (index, name) in [
                        "Brush",
                        "Calligraphy brush",
                        "Calligraphy pen",
                        "Airbrush",
                        "Oil brush",
                        "Crayon",
                        "Marker",
                        "Natural pencil",
                        "Watercolor brush",
                    ]
                    .iter()
                    .enumerate()
                    {
                        let enabled = index == 0;
                        let row = ui.add_enabled(
                            enabled,
                            egui::Button::new(*name)
                                .min_size(vec2(236.0, 42.0))
                                .selected(enabled && self.tool == Tool::Brush),
                        );
                        let sample = egui::Rect::from_center_size(
                            row.rect.right_center() - vec2(46.0, 0.0),
                            vec2(66.0, 16.0),
                        );
                        interface::stroke_sample(
                            ui,
                            sample,
                            if index == 6 { 7.0 } else { 3.0 },
                            Color32::from_gray(if enabled { 235 } else { 100 }),
                        );
                        if row.clicked() {
                            self.choose_tool(Tool::Brush);
                            ui.close();
                        }
                        if !enabled {
                            row.on_disabled_hover_text("This brush engine is not implemented yet");
                        }
                    }
                });
            if self.tool == Tool::Fill {
                ui.add(egui::DragValue::new(&mut self.fill_tolerance).range(0..=255))
                    .on_hover_text("Fill tolerance");
            }
        });
    }
    fn shape_style(&mut self, ui: &mut Ui, fill: bool) {
        let response = icons::button(
            ui,
            if fill { Icon::Fill } else { Icon::Pencil },
            if fill { "Shape fill" } else { "Shape outline" },
            false,
            self.tool.shape()
                && (!fill
                    || (self.tool != Tool::Curve
                        && self.tool.shape_kind() != Some(palette_core::shapes::Kind::Line))),
            vec2(42.0, 32.0),
        );
        // Use one native egui popup chain, so switching flyouts dismisses the
        // previous one and Escape/outside clicks cannot leave orphan windows.
        egui::Popup::menu(&response)
            .style(interface::menu_style)
            .show(|ui| {
                for (name, enabled_value) in if fill {
                    [("No fill", false), ("Solid color", true)]
                } else {
                    [("No outline", false), ("Solid color", true)]
                } {
                    if ui
                        .add(
                            egui::Button::new(name)
                                .selected(if fill {
                                    self.shape_fill == enabled_value
                                } else {
                                    self.shape_outline == enabled_value
                                })
                                .min_size(vec2(210.0, 32.0)),
                        )
                        .clicked()
                    {
                        if fill {
                            self.shape_fill = enabled_value;
                        } else {
                            self.shape_outline = enabled_value;
                        }
                        ui.close();
                    }
                }
                for name in ["Crayon", "Marker", "Oil", "Natural pencil", "Watercolor"] {
                    ui.add_enabled(false, egui::Button::new(name).min_size(vec2(210.0, 36.0)))
                        .on_hover_text("Textured shape styles are not implemented yet");
                }
            });
    }
    fn width_control(&mut self, ui: &mut Ui) {
        let response = icons::button(
            ui,
            Icon::Width,
            "Stroke width",
            false,
            self.tool.paints() || self.tool.shape(),
            vec2(36.0, 64.0),
        );
        self.capture_popup("width", &response);
        egui::Popup::menu(&response)
            .style(interface::menu_style)
            .show(|ui| {
                for width in [1.0, 3.0, 5.0, 8.0] {
                    let row = ui.add(
                        egui::Button::new(format!("{width:.0} px"))
                            .min_size(vec2(160.0, 34.0))
                            .selected(self.brush.diameter == width),
                    );
                    let rect = egui::Rect::from_center_size(
                        row.rect.right_center() - vec2(48.0, 0.0),
                        vec2(70.0, 18.0),
                    );
                    ui.painter().line_segment(
                        [rect.left_center(), rect.right_center()],
                        Stroke::new(width, Color32::WHITE),
                    );
                    if row.clicked() {
                        self.brush.diameter = width;
                        ui.close();
                    }
                }
            });
    }
    fn shape_gallery(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            egui::Frame::new()
                .stroke(Stroke::new(1.0, Color32::from_gray(55)))
                .corner_radius(4)
                .inner_margin(2)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().interact_size.y = 18.0;
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for row in SHAPE_GALLERY.chunks(7) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 1.;
                                for &(icon, tool) in row {
                                    if let Some(tool) = tool {
                                        self.tool_button(ui, tool, icon, vec2(22., 18.));
                                    } else {
                                        icons::button(
                                            ui,
                                            icon,
                                            if matches!(icon, Icon::Curve) {
                                                "Curve — not implemented yet"
                                            } else {
                                                "Free polygon — not implemented yet"
                                            },
                                            false,
                                            false,
                                            vec2(22., 18.),
                                        );
                                    }
                                }
                            });
                        }
                    });
                });
            ui.vertical(|ui| {
                self.shape_style(ui, false);
                self.shape_style(ui, true);
            });
            self.width_control(ui);
        });
    }
    fn color_controls(&mut self, ui: &mut Ui) {
        use crate::app::appearance::{color_dot, empty_dot};
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                for target in [ColorTarget::Foreground, ColorTarget::Background] {
                    let color = if target == ColorTarget::Foreground {
                        Color32::from_rgba_unmultiplied(
                            self.brush.color[0],
                            self.brush.color[1],
                            self.brush.color[2],
                            self.brush.color[3],
                        )
                    } else {
                        self.background
                    };
                    let response =
                        color_dot(ui, color, 32.0, self.interface.color_target == target);
                    if response.clicked() {
                        self.interface.color_target = target;
                    }
                    if response.double_clicked() {
                        self.edit_color(target);
                    }
                    response.on_hover_text(if target == ColorTarget::Foreground {
                        "Foreground color; double-click to edit"
                    } else {
                        "Background color; double-click to edit"
                    });
                }
            });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.spacing_mut().interact_size.y = 20.0;
                for row in PALETTE.chunks(10) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for rgb in row {
                            let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            let response = color_dot(ui, color, 22.0, false);
                            if response.clicked() {
                                match self.interface.color_target {
                                    ColorTarget::Foreground => {
                                        self.brush.color = [rgb[0], rgb[1], rgb[2], 255]
                                    }
                                    ColorTarget::Background => self.background = color,
                                }
                            }
                            if response.secondary_clicked() {
                                self.background = color;
                            }
                            response
                                .on_hover_text(format!("RGB {}, {}, {}", rgb[0], rgb[1], rgb[2]));
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for index in 0..10 {
                        if let Some(color) = self
                            .interface
                            .custom_colors
                            .get(self.interface.custom_colors.len().saturating_sub(10) + index)
                            .copied()
                        {
                            if color_dot(ui, color, 22.0, false).clicked() {
                                match self.interface.color_target {
                                    ColorTarget::Foreground => {
                                        self.brush.color = color.to_srgba_unmultiplied()
                                    }
                                    ColorTarget::Background => self.background = color,
                                }
                            }
                        } else {
                            empty_dot(ui, 22.0);
                        }
                    }
                });
            });
            let (rect, response) = ui.allocate_exact_size(vec2(30.0, 64.0), Sense::click());
            let c = rect.center();
            let mut mesh = egui::epaint::Mesh::default();
            mesh.colored_vertex(c, Color32::WHITE);
            for i in 0..=36 {
                let a = i as f32 / 36.0 * std::f32::consts::TAU;
                mesh.colored_vertex(
                    c + vec2(a.cos(), a.sin()) * 12.0,
                    Color32::from(egui::ecolor::Hsva::new(i as f32 / 36.0, 1.0, 1.0, 1.0)),
                );
                if i > 0 {
                    mesh.add_triangle(0, i, i + 1);
                }
            }
            ui.painter().add(egui::Shape::mesh(mesh));
            if response.clicked() {
                self.edit_color(self.interface.color_target);
            }
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Edit colors"));
            response.on_hover_text("Edit colors");
        });
    }
    pub fn ribbon(&mut self, ui: &mut Ui) {
        egui::ScrollArea::horizontal()
            .id_salt("ribbon-scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 7.0;
                    group(ui, 62.0, "Selection", |ui| self.selection_control(ui));
                    group(ui, 144.0, "Image", |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                if icons::button(
                                    ui,
                                    Icon::Crop,
                                    "Crop to selection",
                                    false,
                                    self.current().document.selection.is_some()
                                        && !self.edit_pending,
                                    vec2(30.0, 30.0),
                                )
                                .clicked()
                                {
                                    self.edit(crate::app::editing::Edit::Crop, ui.ctx());
                                }
                                icons::button(
                                    ui,
                                    Icon::Effects,
                                    "Remove background is not implemented yet",
                                    false,
                                    false,
                                    vec2(30.0, 30.0),
                                );
                            });
                            ui.vertical(|ui| {
                                self.transform_control(ui, false);
                                self.transform_control(ui, true);
                            });
                            if icons::button(
                                ui,
                                Icon::Resize,
                                "Resize image",
                                false,
                                !self.edit_pending,
                                vec2(36.0, 64.0),
                            )
                            .clicked()
                            {
                                self.show_resize();
                            }
                        });
                    });
                    group(ui, 124.0, "Tools", |ui| {
                        ui.horizontal(|ui| {
                            self.tool_button(ui, Tool::Pencil, Icon::Pencil, vec2(32.0, 30.0));
                            self.tool_button(ui, Tool::Fill, Icon::Fill, vec2(32.0, 30.0));
                            self.tool_button(ui, Tool::Text, Icon::Text, vec2(32.0, 30.0));
                        });
                        ui.horizontal(|ui| {
                            self.tool_button(ui, Tool::Eraser, Icon::Eraser, vec2(32.0, 30.0));
                            self.tool_button(ui, Tool::Picker, Icon::Picker, vec2(32.0, 30.0));
                            self.tool_button(ui, Tool::Magnifier, Icon::Zoom, vec2(32.0, 30.0));
                        });
                    });
                    group(ui, 58.0, "Brushes", |ui| self.brushes_control(ui));
                    group(ui, 280.0, "Shapes", |ui| self.shape_gallery(ui));
                    group(ui, 330.0, "Colors", |ui| self.color_controls(ui));
                    group(ui, 60.0, "Effects", |ui| {
                        if icons::button(
                            ui,
                            Icon::Effects,
                            "Live effects",
                            self.processing_section_is(crate::app::processing::Section::Effects),
                            true,
                            vec2(48.0, 62.0),
                        )
                        .clicked()
                        {
                            self.toggle_processing_toolbar(
                                crate::app::processing::Section::Effects,
                            );
                        }
                    });
                    ui.vertical(|ui| {
                        if icons::button(
                            ui,
                            Icon::Layers,
                            "Show or hide layers",
                            self.show_layers,
                            true,
                            vec2(42.0, 62.0),
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
