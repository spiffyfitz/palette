use super::*;

struct TabDrag(u64);
struct LayerDrag {
    session: u64,
    layer: u64,
}

impl PaletteApp {
    pub(super) fn menus(&mut self, ui: &mut Ui) {
        use crate::interface::{menu_item, menu_style};
        let ctx = ui.ctx().clone();
        egui::MenuBar::new()
            .config(egui::containers::menu::MenuConfig::new().style(menu_style))
            .ui(ui, |ui| {
                let file = ui.menu_button("File", |ui| {
                    if menu_item(ui, "New", "Ctrl+N", Some(Icon::New), true).clicked() {
                        self.new_document();
                        ui.close();
                    }
                    if menu_item(ui, "Open…", "Ctrl+O", Some(Icon::Open), true).clicked() {
                        self.open_dialog(&ctx);
                        ui.close();
                    }
                    if menu_item(
                        ui,
                        "Import to canvas…",
                        "",
                        Some(Icon::Layers),
                        !self.edit_pending,
                    )
                    .clicked()
                    {
                        self.import_image(None, &ctx);
                        ui.close();
                    }
                    self.recent_menu(ui);
                    if !self.recoveries.is_empty()
                        && menu_item(ui, "Recover unsaved documents…", "", None, true).clicked()
                    {
                        self.recovery_prompt = true;
                        ui.close();
                    }
                    ui.separator();
                    if menu_item(ui, "Save project", "Ctrl+S", Some(Icon::Save), !self.busy)
                        .clicked()
                    {
                        self.save(&ctx, false, false);
                        ui.close();
                    }
                    if menu_item(
                        ui,
                        "Save project as…",
                        "Ctrl+Shift+S",
                        Some(Icon::Save),
                        !self.busy,
                    )
                    .clicked()
                    {
                        self.save(&ctx, true, false);
                        ui.close();
                    }
                    if menu_item(ui, "Export image…", "", Some(Icon::Save), !self.busy).clicked()
                    {
                        self.save(&ctx, true, true);
                        ui.close();
                    }
                    if self.zoom_export.is_some()
                        && menu_item(ui, "Cancel Super Zoom export", "", None, true).clicked()
                    {
                        self.cancel_zoom_export();
                        ui.close();
                    }
                    ui.separator();
                    if menu_item(
                        ui,
                        "Image properties",
                        "Ctrl+E",
                        Some(Icon::Resize),
                        !self.edit_pending,
                    )
                    .clicked()
                    {
                        self.show_canvas_properties();
                        ui.close();
                    }
                    if menu_item(ui, "Close document", "Ctrl+W", None, true).clicked() {
                        self.request_close(self.current().id, &ctx);
                        ui.close();
                    }
                    if menu_item(ui, "Exit", "", None, true).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });
                self.capture_popup("file-menu", &file.response);
                ui.menu_button("Edit", |ui| {
                    let undo = self
                        .current()
                        .history
                        .undo_label()
                        .map(|s| format!("Undo {s}"));
                    let redo = self
                        .current()
                        .history
                        .redo_label()
                        .map(|s| format!("Redo {s}"));
                    if menu_item(
                        ui,
                        undo.as_deref().unwrap_or("Undo"),
                        "Ctrl+Z",
                        Some(Icon::Undo),
                        undo.is_some(),
                    )
                    .clicked()
                    {
                        self.undo_current();
                        ui.close();
                    }
                    if menu_item(
                        ui,
                        redo.as_deref().unwrap_or("Redo"),
                        "Ctrl+Y",
                        Some(Icon::Redo),
                        redo.is_some(),
                    )
                    .clicked()
                    {
                        self.redo_current();
                        ui.close();
                    }
                    ui.separator();
                    if menu_item(
                        ui,
                        "Cut",
                        "Ctrl+X",
                        None,
                        !self.current().document.active().locked && !self.edit_pending,
                    )
                    .clicked()
                    {
                        self.copy_selection(true, &ctx);
                        ui.close();
                    }
                    if menu_item(ui, "Copy", "Ctrl+C", None, true).clicked() {
                        self.copy_selection(false, &ctx);
                        ui.close();
                    }
                    if menu_item(ui, "Paste as layer", "Ctrl+V", None, !self.edit_pending).clicked()
                    {
                        self.paste_clipboard();
                        ui.close();
                    }
                    ui.separator();
                    if menu_item(ui, "Select all", "Ctrl+A", Some(Icon::Select), true).clicked() {
                        self.select_all();
                        ui.close();
                    }
                    if menu_item(ui, "Deselect", "Ctrl+D", None, true).clicked() {
                        self.deselect();
                        ui.close();
                    }
                    if menu_item(ui, "Invert selection", "", None, true).clicked() {
                        self.invert_selection();
                        ui.close();
                    }
                    if menu_item(
                        ui,
                        "Clear selection",
                        "Delete",
                        None,
                        !self.current().document.active().locked && !self.edit_pending,
                    )
                    .clicked()
                    {
                        self.edit(super::editing::Edit::Clear, &ctx);
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.set_min_width(230.0);
                    ui.menu_button("Zoom", |ui| {
                        for percent in [25, 50, 75, 100, 150, 200, 400, 800] {
                            if menu_item(ui, &format!("{percent}%"), "", None, true).clicked() {
                                self.current_mut().zoom = percent as f32 / 100.0;
                                self.current_mut().fit = false;
                                ui.close();
                            }
                        }
                        if menu_item(ui, "Fit to window", "", Some(Icon::Fit), true).clicked() {
                            self.current_mut().fit = true;
                            self.current_mut().pan = Vec2::ZERO;
                            ui.close();
                        }
                    });
                    menu_item(ui, "Rulers", "", None, false)
                        .on_hover_text("Rulers are not implemented yet");
                    ui.checkbox(&mut self.interface.grid, "Gridlines")
                        .on_hover_text("Pixel grid at 400% zoom and above");
                    ui.checkbox(&mut self.interface.status, "Status bar");
                    ui.separator();
                    ui.checkbox(&mut self.interface.ribbon_collapsed, "Collapse toolbar");
                    ui.checkbox(&mut self.checkerboard, "Transparency checkerboard");
                    ui.checkbox(&mut self.show_layers, "Layers");
                });
                let filter_menu = ui.menu_button("Filter", |ui| {
                    use palette_core::processing::{Kind, menu::CATEGORIES};
                    menu_item(ui, "Filter Gallery…", "", None, false).on_disabled_hover_text(
                        "Filter Gallery is planned; sample-image previews are not implemented yet",
                    );
                    ui.separator();
                    for category in CATEGORIES {
                        let response = ui.menu_button(category.label, |ui| {
                            // A popup Area starts with a 400-point default size.
                            // Let long conventional branches use the available
                            // window height, with a visible rail when necessary.
                            let height = (ctx.content_rect().height() - 48.).min(620.);
                            ui.set_max_height(height);
                            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
                            ui.style_mut().spacing.scroll.dormant_handle_opacity = 0.6;
                            egui::ScrollArea::vertical()
                                .max_height(height)
                                .show(ui, |ui| {
                                    for &name in category.entries {
                                        let descriptor = category.implemented(name);
                                        let label = category.caption(name);
                                        let response = menu_item(
                                            ui,
                                            &label,
                                            "",
                                            None,
                                            descriptor.is_some() && self.close_pending_none(),
                                        );
                                        if let Some(descriptor) = descriptor {
                                            if response.clicked() {
                                                if descriptor.kind == Kind::Invert {
                                                    self.add_filter(Filter::Invert);
                                                } else {
                                                    self.open_processing(descriptor.kind, None);
                                                }
                                                ui.close();
                                            }
                                        } else {
                                            response.on_disabled_hover_text("Not implemented yet");
                                        }
                                    }
                                });
                        });
                        let scene = match category.label {
                            "Neural Filters" => Some("neural-menu"),
                            "Adjustments" => Some("adjustments-menu"),
                            "Blur" => Some("blur-menu"),
                            "Noise" => Some("noise-menu"),
                            "Pixelate" => Some("pixelate-menu"),
                            "Other" => Some("other-menu"),
                            "Distort" => Some("distort-menu"),
                            "Sharpen" => Some("sharpen-menu"),
                            "Stylize" => Some("stylize-menu"),
                            "Artistic" => Some("artistic-menu"),
                            "Brush Strokes" => Some("brush-strokes-menu"),
                            "Sketch" => Some("sketch-menu"),
                            "Render" => Some("render-menu"),
                            "Blur Gallery" => Some("blur-gallery-menu"),
                            "Video" => Some("video-menu"),
                            "Texture" => Some("texture-menu"),
                            _ => None,
                        };
                        if let Some(scene) = scene {
                            self.capture_submenu(scene, ui, &response.response);
                        }
                    }
                });
                for scene in [
                    "neural-menu",
                    "filter-menu",
                    "adjustments-menu",
                    "noise-menu",
                    "blur-menu",
                    "pixelate-menu",
                    "other-menu",
                    "distort-menu",
                    "sharpen-menu",
                    "stylize-menu",
                    "artistic-menu",
                    "brush-strokes-menu",
                    "sketch-menu",
                    "render-menu",
                    "blur-gallery-menu",
                    "video-menu",
                    "texture-menu",
                ] {
                    self.capture_popup(scene, &filter_menu.response);
                }
                ui.separator();
                if icons::button(
                    ui,
                    Icon::Save,
                    "Save project (Ctrl+S)",
                    false,
                    !self.busy,
                    vec2(32.0, 28.0),
                )
                .clicked()
                {
                    self.save(&ctx, false, false);
                }
                if icons::button(
                    ui,
                    Icon::Undo,
                    "Undo (Ctrl+Z)",
                    false,
                    self.current().history.undo_label().is_some(),
                    vec2(32.0, 28.0),
                )
                .clicked()
                {
                    self.undo_current();
                }
                if icons::button(
                    ui,
                    Icon::Redo,
                    "Redo (Ctrl+Shift+Z)",
                    false,
                    self.current().history.redo_label().is_some(),
                    vec2(32.0, 28.0),
                )
                .clicked()
                {
                    self.redo_current();
                }
                if self.busy || self.edit_pending {
                    ui.spinner();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::interface::chevron(ui, vec2(26.0, 26.0), "Expand or collapse toolbar")
                        .clicked()
                    {
                        self.interface.ribbon_collapsed = !self.interface.ribbon_collapsed;
                    }
                });
            });
    }

    pub(super) fn tabs(&mut self, ui: &mut Ui) {
        let mut close = None;
        let mut reorder = None;
        let mut activate = None;
        egui::ScrollArea::horizontal()
            .id_salt("tabs-scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, session) in self.sessions.iter().enumerate() {
                        let label = format!(
                            "{}{}",
                            session.name,
                            if session.modified() { " •" } else { "" }
                        );
                        let (response, close_clicked) =
                            document_tab(ui, session.id, &label, index == self.active);
                        if response.clicked() && !close_clicked {
                            activate = Some(index);
                        }
                        response.dnd_set_drag_payload(TabDrag(session.id));
                        if let Some(source) = response.dnd_release_payload::<TabDrag>() {
                            reorder = Some((source.0, session.id));
                        }
                        if close_clicked {
                            close = Some(session.id);
                        }
                        ui.add_space(5.0);
                    }
                    if ui
                        .small_button("+")
                        .on_hover_text("New document (Ctrl+N)")
                        .clicked()
                    {
                        activate = Some(self.sessions.len());
                    }
                });
            });
        if let Some(index) = activate {
            if !self.finish_inline_text() {
                self.after_text = Some(if index == self.sessions.len() {
                    text::AfterText::New
                } else {
                    text::AfterText::Tab(self.sessions[index].id)
                });
                return;
            }
            self.current_mut().end_gesture();
            self.stroke = None;
            if index == self.sessions.len() {
                self.new_document();
            } else {
                self.active = index;
            }
        }
        if let Some((source, target)) = reorder {
            let active_id = self.current().id;
            if let (Some(from), Some(to)) = (
                self.sessions.iter().position(|s| s.id == source),
                self.sessions.iter().position(|s| s.id == target),
            ) {
                let session = self.sessions.remove(from);
                self.sessions.insert(to, session);
                self.active = self
                    .sessions
                    .iter()
                    .position(|s| s.id == active_id)
                    .expect("active tab");
            }
        }
        if let Some(id) = close {
            self.request_close(id, ui.ctx());
        }
    }

    pub fn add_filter(&mut self, filter: Filter) {
        if !self.finish_inline_text() {
            self.after_text = Some(text::AfterText::Filter(filter));
            return;
        }
        self.current_mut().end_gesture();
        let mut doc = self.current().document.clone();
        if doc.active().locked {
            self.error = Some("Unlock the layer before editing filters".into());
            return;
        }
        let id = doc.allocate_id();
        doc.active_mut().filters.push(FilterInstance {
            id,
            enabled: true,
            filter,
            mask: None,
        });
        match doc.validate() {
            Ok(()) => self.current_mut().command("Add live filter", |d| *d = doc),
            Err(error) => self.error = Some(error),
        }
    }

    pub fn layers(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 30.0) * 0.5);
            let response = ui.add_enabled(
                self.current().document.layers.len() < 256,
                egui::Button::new(" ")
                    .frame(false)
                    .min_size(vec2(30.0, 30.0)),
            );
            let c = response.rect.center();
            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
            ui.painter().circle_stroke(c, 6.0, stroke);
            ui.painter()
                .line_segment([c - vec2(3.0, 0.0), c + vec2(3.0, 0.0)], stroke);
            ui.painter()
                .line_segment([c - vec2(0.0, 3.0), c + vec2(0.0, 3.0)], stroke);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Add layer"));
            if response.on_hover_text("Add transparent layer").clicked()
                && self.finish_inline_text()
            {
                self.current_mut().command("Add layer", Document::add_layer);
            }
        });
        ui.separator();
        let layers: Vec<_> = self
            .current()
            .document
            .layers
            .iter()
            .rev()
            .map(|l| (l.id, l.name.clone(), l.visible, l.locked))
            .collect();
        let mut action = None;
        let mut reorder = None;
        let mut activate = None;
        let active = self.current().document.active_layer;
        for (id, name, visible, locked) in layers {
            let selected = active == id;
            let key = (self.current().id, id);
            let generation = self.current().generation;
            if self.thumbnails.get(&key).is_none_or(|(g, _)| {
                *g != generation
                    && !palette_core::processing::has_neighborhood(
                        self.current()
                            .document
                            .layers
                            .iter()
                            .find(|l| l.id == id)
                            .expect("layer"),
                    )
            }) {
                let layer = self
                    .current()
                    .document
                    .layers
                    .iter()
                    .find(|l| l.id == id)
                    .expect("layer");
                // Spatial thumbnails arrive with the background canvas render.
                // Until then (or for a hidden layer), show the original source.
                let spatial = palette_core::processing::has_neighborhood(layer);
                let mut pixels = vec![];
                for y in 0..36 {
                    for x in 0..48 {
                        let p = if spatial {
                            layer.source.get(
                                x * layer.source.width() / 48,
                                y * layer.source.height() / 36,
                            )
                        } else {
                            composite::layer_pixel(
                                layer,
                                x * layer.source.width() / 48,
                                y * layer.source.height() / 36,
                            )
                        };
                        let gray = if (x / 6 + y / 6) % 2 == 0 { 185 } else { 225 };
                        pixels.extend_from_slice(&composite::over(
                            [gray, gray, gray, 255],
                            p,
                            layer.opacity,
                            BlendMode::Normal,
                        ));
                    }
                }
                let image = ColorImage::from_rgba_unmultiplied([48, 36], &pixels);
                let texture = ui.ctx().load_texture(
                    format!("thumb-{}-{id}", key.0),
                    image,
                    TextureOptions::NEAREST,
                );
                self.thumbnails.insert(key, (generation, texture));
            }

            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), 86.0), Sense::click_and_drag());
            let thumb = egui::Rect::from_center_size(rect.center(), vec2(82.0, 78.0));
            ui.painter()
                .rect_filled(thumb, 4, Color32::from_rgb(34, 34, 38));
            ui.painter().rect_stroke(
                thumb,
                4,
                egui::Stroke::new(
                    1.0,
                    if selected {
                        ACCENT
                    } else {
                        Color32::from_gray(55)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let layer = self
                .current()
                .document
                .layers
                .iter()
                .find(|l| l.id == id)
                .expect("layer");
            let ratio = layer.source.width() as f32 / layer.source.height() as f32;
            let size = if ratio >= 1.0 {
                vec2(74.0, 74.0 / ratio)
            } else {
                vec2(66.0 * ratio, 66.0)
            };
            if let Some((_, texture)) = self.thumbnails.get(&(self.current().id, id)) {
                ui.painter().image(
                    texture.id(),
                    egui::Rect::from_center_size(thumb.center(), size),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    if visible {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(110)
                    },
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), selected, &name)
            });
            let mut child_clicked = false;
            if response.contains_pointer() || !visible {
                let eye = egui::Rect::from_min_size(thumb.min + vec2(3.0, 3.0), vec2(20.0, 18.0));
                let response = ui
                    .interact(eye, ui.scope_id().with((id, "visibility")), Sense::click())
                    .on_hover_text("Toggle layer visibility");
                ui.painter()
                    .rect_filled(eye, 3, Color32::from_black_alpha(170));
                icons::draw(ui, eye.shrink(4.0), Icon::Ellipse, Color32::WHITE);
                ui.painter()
                    .circle_filled(eye.center(), 1.5, Color32::WHITE);
                if !visible {
                    ui.painter().line_segment(
                        [eye.left_bottom(), eye.right_top()],
                        egui::Stroke::new(1.0, Color32::WHITE),
                    );
                }
                if response.clicked() {
                    action = Some((id, 0));
                    child_clicked = true;
                }
            }
            if response.contains_pointer() {
                let more = egui::Rect::from_min_size(
                    thumb.right_bottom() - vec2(24.0, 20.0),
                    vec2(22.0, 18.0),
                );
                let more_response =
                    ui.interact(more, ui.scope_id().with((id, "properties")), Sense::click());
                ui.painter()
                    .rect_filled(more, 3, Color32::from_black_alpha(180));
                for x in [-4.0, 0.0, 4.0] {
                    ui.painter()
                        .circle_filled(more.center() + vec2(x, 0.0), 1.0, Color32::WHITE);
                }
                if more_response.on_hover_text("Layer properties").clicked() {
                    activate = Some(id);
                    self.interface.layer_properties = !self.interface.layer_properties;
                    child_clicked = true;
                }
            }
            if locked {
                icons::draw(
                    ui,
                    egui::Rect::from_min_size(
                        thumb.left_bottom() + vec2(4.0, -18.0),
                        vec2(14.0, 14.0),
                    ),
                    Icon::Lock,
                    Color32::WHITE,
                );
            }
            if response.clicked() && !child_clicked {
                activate = Some(id);
            }
            response.dnd_set_drag_payload(LayerDrag {
                session: self.current().id,
                layer: id,
            });
            if let Some(source) = response.dnd_release_payload::<LayerDrag>()
                && source.session == self.current().id
            {
                reorder = Some((source.layer, id));
            }
            response.context_menu(|ui| {
                if ui.button("Properties").clicked() {
                    activate = Some(id);
                    self.interface.layer_properties = true;
                    ui.close();
                }
                for (label, code) in [
                    ("Move layer", 4),
                    ("Duplicate", 1),
                    (if locked { "Unlock" } else { "Lock" }, 2),
                    ("Delete", 3),
                ] {
                    if ui
                        .add_enabled(
                            code != 3 || self.current().document.layers.len() > 1,
                            egui::Button::new(label),
                        )
                        .clicked()
                    {
                        action = Some((id, code));
                        ui.close();
                    }
                }
            });
            response.on_hover_text(if palette_core::processing::has_neighborhood(self.current().document.layers.iter().find(|l|l.id==id).expect("layer")) {format!("{name} · Spatial preview updates with the canvas; hidden layers show source content")} else {name});
            if selected && self.interface.layer_properties {
                ui.push_id((self.current().id, id), |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(34, 34, 40))
                        .corner_radius(4)
                        .inner_margin(4)
                        .show(ui, |ui| {
                            ui.add_enabled_ui(self.text_editor.is_none(), |ui| {
                                self.layer_properties(ui);
                            });
                        });
                });
            }
        }
        if let Some(id) = activate
            && self.finish_inline_text()
        {
            self.current_mut().end_gesture();
            self.stroke = None;
            self.current_mut().document.active_layer = id;
        }
        if let Some((id, code)) = action
            && self.finish_inline_text()
        {
            self.current_mut().end_gesture();
            self.current_mut().document.active_layer = id;
            if code == 4 {
                self.choose_tool(Tool::MoveLayer);
            } else {
                self.current_mut()
                    .command("Layer operation", |doc| match code {
                        0 => doc.active_mut().visible = !doc.active().visible,
                        1 => doc.duplicate_layer(),
                        2 => doc.active_mut().locked = !doc.active().locked,
                        _ => doc.delete_layer(),
                    });
            }
        }
        if let Some((source, target)) = reorder
            && source != target
            && self.finish_inline_text()
        {
            self.current_mut().command("Reorder layer", |doc| {
                let from = doc
                    .layers
                    .iter()
                    .position(|l| l.id == source)
                    .expect("source");
                let to = doc
                    .layers
                    .iter()
                    .position(|l| l.id == target)
                    .expect("target");
                let layer = doc.layers.remove(from);
                doc.layers.insert(to, layer);
            });
        }
    }

    fn layer_properties(&mut self, ui: &mut Ui) {
        ui.spacing_mut().slider_width = 70.0;
        let mut name = self.current().document.active().name.clone();
        if ui
            .text_edit_singleline(&mut name)
            .on_hover_text("Rename active layer")
            .changed()
        {
            self.current_mut()
                .command("Rename layer", |doc| doc.active_mut().name = name);
        }
        let mut opacity = self.current().document.active().opacity;
        ui.label("Opacity");
        let response = ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0));
        if response.changed() {
            self.current_mut()
                .command("Layer opacity", |doc| doc.active_mut().opacity = opacity);
        }
        ui.horizontal(|ui| {
            let size = vec2(((ui.available_width() - 12.) / 3.).clamp(28., 50.), 30.);
            for (index, section, icon, name) in [
                (
                    0,
                    super::processing::Section::Filters,
                    Icon::Filters,
                    "Filters",
                ),
                (
                    1,
                    super::processing::Section::Effects,
                    Icon::Effects,
                    "Effects",
                ),
            ] {
                let response = icons::button(
                    ui,
                    icon,
                    name,
                    self.processing_section_is(section),
                    true,
                    size,
                );
                self.layer_processing_buttons[index] = response.rect;
                if response.clicked() {
                    self.toggle_processing_toolbar(section);
                }
            }
            let mut blend = self.current().document.active().blend;
            let response = icons::button(
                ui,
                Icon::Blend,
                &format!("Blending mode: {}", blend.label()),
                blend != BlendMode::Normal,
                true,
                size,
            );
            self.layer_processing_buttons[2] = response.rect;
            self.capture_popup("layer-blending", &response);
            egui::Popup::menu(&response)
                .style(crate::interface::menu_style)
                .show(|ui| {
                    ui.label("Blending mode");
                    egui::ScrollArea::vertical()
                        .max_height(360.)
                        .show(ui, |ui| {
                            for mode in BlendMode::ALL {
                                if ui
                                    .selectable_value(&mut blend, mode, mode.label())
                                    .clicked()
                                {
                                    ui.close();
                                }
                            }
                        });
                });
            if blend != self.current().document.active().blend {
                self.cancel_processing();
                self.current_mut()
                    .command("Blend mode", |doc| doc.active_mut().blend = blend);
            }
        });
        self.mask_properties(ui);
    }

    pub(super) fn dialogs(&mut self, ctx: &Context) {
        self.feather_window(ctx);
        if let Some(id) = self.close_pending
            && self.error.is_none()
        {
            self.close_dialog(ctx, id);
        }
        if let Some(error) = self.error.clone() {
            egui::Modal::new(egui::Id::unique("file-error")).show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.heading("File operation failed");
                ui.label(error);
                if ui.button("OK").clicked() {
                    self.error = None;
                }
            });
        }
    }

    pub(super) fn status(&mut self, ui: &mut Ui) {
        let selection = self.current_mut().selection_bounds();
        ui.horizontal_centered(|ui| {
            ui.label(
                self.cursor
                    .map(|p| format!("{}, {} px", p[0], p[1]))
                    .unwrap_or_else(|| self.tool.label().into()),
            );
            ui.separator();
            ui.label(format!(
                "{} × {} px",
                self.current().document.width,
                self.current().document.height
            ));
            ui.separator();
            if let Some(rect) = selection {
                ui.label(format!("Selection {} × {} px", rect.width, rect.height));
                ui.separator();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+").on_hover_text("Zoom in").clicked() {
                    self.current_mut().zoom = (self.current().zoom * 1.25).min(32.0);
                    self.current_mut().fit = false;
                }
                let previous_zoom = self.current().zoom;
                let mut zoom = previous_zoom;
                let response = ui
                    .add(
                        egui::Slider::new(&mut zoom, 0.05..=32.0)
                            .logarithmic(true)
                            .show_value(false),
                    )
                    .on_hover_text("Canvas zoom — snaps to 100%");
                if response.changed() {
                    // A small magnetic range makes actual size easy to hit on
                    // the logarithmic rail. Keyboard steps can still leave it.
                    if (zoom - 1.0).abs() <= 0.05
                        && (response.dragged() || response.clicked() || previous_zoom != 1.0)
                    {
                        zoom = 1.0;
                    }
                    self.current_mut().zoom = zoom;
                    self.current_mut().fit = false;
                }
                if ui.small_button("−").on_hover_text("Zoom out").clicked() {
                    self.current_mut().zoom = (self.current().zoom / 1.25).max(0.05);
                    self.current_mut().fit = false;
                }
                egui::ComboBox::from_id_salt("status-zoom")
                    .width(78.0)
                    .selected_text(format!("{:.0}%", self.current().zoom * 100.0))
                    .show_ui(ui, |ui| {
                        for zoom in [0.125, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 4.0, 8.0, 16.0, 32.0] {
                            if ui
                                .selectable_label(
                                    self.current().zoom == zoom,
                                    format!("{}%", zoom * 100.0),
                                )
                                .clicked()
                            {
                                self.current_mut().zoom = zoom;
                                self.current_mut().fit = false;
                            }
                        }
                    });
                if icons::button(
                    ui,
                    Icon::Fit,
                    "Fit to window",
                    false,
                    true,
                    vec2(27.0, 25.0),
                )
                .clicked()
                {
                    self.current_mut().fit = true;
                    self.current_mut().pan = Vec2::ZERO;
                }
                let session = self.current();
                if session.requested == Some(session.generation)
                    && session.rendered_generation != Some(session.generation)
                {
                    ui.add(egui::Spinner::new().size(12.));
                    ui.add(egui::Label::new("Rendering…").truncate());
                } else {
                    ui.add(egui::Label::new(&self.message).truncate());
                }
            });
        });
    }
}

/// Keep a permanent close-button slot inside each tab so hover never shifts
/// labels or neighboring tabs. The close hit target only exists while hovered.
fn document_tab(ui: &mut Ui, id: u64, label: &str, selected: bool) -> (egui::Response, bool) {
    ui.push_id(id, |ui| {
        let font = egui::TextStyle::Button.resolve(ui.style());
        let label_width = ui
            .painter()
            .layout_no_wrap(label.to_owned(), font, ui.visuals().text_color())
            .size()
            .x
            .min(200.0);
        let mut close_rect = egui::Rect::NOTHING;
        let response = egui::Frame::new()
            .fill(if selected {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().widgets.inactive.weak_bg_fill
            })
            .corner_radius(5)
            .inner_margin(egui::Margin::symmetric(8, 1))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.horizontal(|ui| {
                    ui.add_sized(vec2(label_width, 20.0), egui::Label::new(label).truncate());
                    close_rect = ui.allocate_exact_size(vec2(18.0, 20.0), Sense::hover()).0;
                });
            })
            .response
            .interact(Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::Role::Tab, ui.is_enabled(), selected, label)
        });
        let close_clicked = if response.contains_pointer() {
            let close = ui.interact(
                close_rect,
                egui::Id::unique(("tab-close", id)),
                Sense::click(),
            );
            let center = close_rect.center();
            let stroke = egui::Stroke::new(1.0, Color32::LIGHT_GRAY);
            ui.painter()
                .line_segment([center + vec2(-3.0, -3.0), center + vec2(3.0, 3.0)], stroke);
            ui.painter()
                .line_segment([center + vec2(-3.0, 3.0), center + vec2(3.0, -3.0)], stroke);
            close.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Close document")
            });
            close.on_hover_text("Close document").clicked()
        } else {
            false
        };
        (response, close_clicked)
    })
    .inner
}
