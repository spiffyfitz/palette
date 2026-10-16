use super::*;

struct TabDrag(u64);
struct LayerDrag {
    session: u64,
    layer: u64,
}

impl PaletteApp {
    pub(super) fn menus(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New                         Ctrl+N").clicked() {
                    self.new_document();
                    ui.close();
                }
                if ui.button("Open…                     Ctrl+O").clicked() {
                    self.open_dialog(&ctx);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        !self.busy,
                        egui::Button::new("Save project             Ctrl+S"),
                    )
                    .clicked()
                {
                    self.save(&ctx, false, false);
                    ui.close();
                }
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Save project as…"))
                    .clicked()
                {
                    self.save(&ctx, true, false);
                    ui.close();
                }
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Export flattened image…"))
                    .clicked()
                {
                    self.save(&ctx, true, true);
                    ui.close();
                }
                ui.separator();
                if ui.button("Close document         Ctrl+W").clicked() {
                    self.request_close(self.current().id, &ctx);
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo = self
                    .current()
                    .history
                    .undo_label()
                    .map(|s| format!("Undo {s}"));
                if ui
                    .add_enabled(
                        undo.is_some(),
                        egui::Button::new(undo.unwrap_or("Undo".into())),
                    )
                    .clicked()
                {
                    self.undo_current();
                    ui.close();
                }
                let redo = self
                    .current()
                    .history
                    .redo_label()
                    .map(|s| format!("Redo {s}"));
                if ui
                    .add_enabled(
                        redo.is_some(),
                        egui::Button::new(redo.unwrap_or("Redo".into())),
                    )
                    .clicked()
                {
                    self.redo_current();
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        !self.current().document.active().locked && !self.edit_pending,
                        egui::Button::new("Cut image selection         Ctrl+X"),
                    )
                    .clicked()
                {
                    self.copy_selection(true, &ctx);
                    ui.close();
                }
                if ui.button("Copy image selection       Ctrl+C").clicked() {
                    self.copy_selection(false, &ctx);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !self.edit_pending,
                        egui::Button::new("Paste image as layer         Ctrl+V"),
                    )
                    .clicked()
                {
                    self.paste_clipboard();
                    ui.close();
                }
                ui.separator();
                if ui
                    .button("Select all                           Ctrl+A")
                    .clicked()
                {
                    self.select_all();
                    ui.close();
                }
                if ui
                    .button("Deselect                            Ctrl+D")
                    .clicked()
                {
                    self.deselect();
                    ui.close();
                }
                if ui.button("Invert selection").clicked() {
                    self.invert_selection();
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !self.current().document.active().locked && !self.edit_pending,
                        egui::Button::new("Clear selected pixels       Delete"),
                    )
                    .clicked()
                {
                    self.edit(super::editing::Edit::Clear, &ctx);
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Actual size (100%)").clicked() {
                    self.current_mut().zoom = 1.0;
                    self.current_mut().fit = false;
                    ui.close();
                }
                if ui.button("Fit to window").clicked() {
                    self.current_mut().fit = true;
                    self.current_mut().pan = Vec2::ZERO;
                    ui.close();
                }
                ui.checkbox(&mut self.checkerboard, "Transparency checkerboard");
                ui.checkbox(&mut self.show_layers, "Layers");
            });
            ui.menu_button("Filter", |ui| {
                ui.menu_button("Adjustments", |ui| {
                    if ui.button("Invert").clicked() {
                        self.add_filter(Filter::Invert);
                        ui.close();
                    }
                    if ui.button("Brightness…").clicked() {
                        self.add_filter(Filter::Brightness { amount: 0.1 });
                        self.show_filters = true;
                        ui.close();
                    }
                });
                for category in [
                    "Artistic",
                    "Blur",
                    "Brush Strokes",
                    "Distort",
                    "Noise",
                    "Pixelate",
                    "Render",
                    "Sketch",
                    "Stylize",
                    "Texture",
                    "Other",
                    "Camera Distortion Correction",
                ] {
                    ui.add_enabled(false, egui::Button::new(category))
                        .on_hover_text("Not implemented in this milestone");
                }
                ui.separator();
                if ui.button("Edit live filter stack…").clicked() {
                    self.show_filters = true;
                    ui.close();
                }
            });
            ui.separator();
            if icons::button(
                ui,
                Icon::Save,
                "Save project (Ctrl+S)",
                false,
                !self.busy,
                vec2(28.0, 26.0),
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
                vec2(28.0, 26.0),
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
                vec2(28.0, 26.0),
            )
            .clicked()
            {
                self.redo_current();
            }
            if self.busy || self.edit_pending {
                ui.spinner();
            }
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
        self.current_mut().command("Add live filter", |doc| {
            let id = doc.allocate_id();
            doc.active_mut().filters.push(FilterInstance {
                id,
                enabled: true,
                filter,
                mask: None,
            });
        });
    }

    pub fn layers(&mut self, ui: &mut Ui) {
        ui.spacing_mut().slider_width = 82.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Layers").strong());
            if ui
                .small_button("+")
                .on_hover_text("Add transparent layer")
                .clicked()
            {
                self.current_mut().command("Add layer", Document::add_layer);
            }
            if ui
                .small_button("×")
                .on_hover_text("Hide layers panel")
                .clicked()
            {
                self.show_layers = false;
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
        let active_layer = self.current().document.active_layer;
        for (id, name, visible, locked) in layers {
            let selected = active_layer == id;
            let response = egui::Frame::new()
                .fill(if selected {
                    Color32::from_rgb(52, 66, 72)
                } else {
                    PANEL
                })
                .inner_margin(5)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut visible = visible;
                        if ui
                            .checkbox(&mut visible, "")
                            .on_hover_text("Layer visibility")
                            .changed()
                        {
                            action = Some((id, 0));
                        }
                        let key = (self.current().id, id);
                        let generation = self.current().generation;
                        if self
                            .thumbnails
                            .get(&key)
                            .is_none_or(|(g, _)| *g != generation)
                        {
                            let layer = self
                                .current()
                                .document
                                .layers
                                .iter()
                                .find(|l| l.id == id)
                                .expect("layer");
                            let mut pixels = vec![];
                            for y in 0..36 {
                                for x in 0..48 {
                                    let p = composite::layer_pixel(
                                        layer,
                                        x * layer.source.width() / 48,
                                        y * layer.source.height() / 36,
                                    );
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
                        if let Some((_, texture)) = self.thumbnails.get(&key) {
                            ui.image((texture.id(), vec2(48.0, 36.0)));
                        }
                        ui.label(if locked { "▣" } else { "" });
                    });
                    ui.add(egui::Label::new(&name).truncate());
                })
                .response
                .interact(Sense::click_and_drag());
            if response.clicked() {
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
                for (label, code) in [
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
            if selected {
                ui.push_id((self.current().id, id), |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(34, 34, 42))
                        .corner_radius(4)
                        .inner_margin(4)
                        .show(ui, |ui| self.layer_properties(ui));
                });
            }
        }
        if let Some(id) = activate {
            self.current_mut().end_gesture();
            self.stroke = None;
            self.current_mut().document.active_layer = id;
        }
        if let Some((id, code)) = action {
            self.current_mut().document.active_layer = id;
            self.current_mut()
                .command("Layer operation", |doc| match code {
                    0 => doc.active_mut().visible = !doc.active().visible,
                    1 => doc.duplicate_layer(),
                    2 => doc.active_mut().locked = !doc.active().locked,
                    _ => doc.delete_layer(),
                });
        }
        if let Some((source, target)) = reorder
            && source != target
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
        let mut blend = self.current().document.active().blend;
        egui::ComboBox::from_id_salt("blend")
            .selected_text(format!("{blend:?}"))
            .width(120.0)
            .show_ui(ui, |ui| {
                for mode in [BlendMode::Normal, BlendMode::Multiply, BlendMode::Screen] {
                    ui.selectable_value(&mut blend, mode, format!("{mode:?}"));
                }
            });
        if blend != self.current().document.active().blend {
            self.current_mut()
                .command("Blend mode", |doc| doc.active_mut().blend = blend);
        }
        if ui.button("Live filters…").clicked() {
            self.show_filters = true;
        }
    }

    fn filter_window(&mut self, ctx: &Context) {
        let mut open = self.show_filters;
        egui::Window::new("Live filters — active layer")
            .open(&mut open)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.label("Source pixels are preserved. Changes can be undone.");
                ui.horizontal(|ui| {
                    if ui.button("+ Invert").clicked() {
                        self.add_filter(Filter::Invert);
                    }
                    if ui.button("+ Brightness").clicked() {
                        self.add_filter(Filter::Brightness { amount: 0.0 });
                    }
                });
                ui.separator();
                let mut filters = self.current().document.active().filters.clone();
                let mut changed = false;
                let mut remove = None;
                let mut swap = None;
                let mut duplicate = None;
                for (index, filter) in filters.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        changed |= ui.checkbox(&mut filter.enabled, "").changed();
                        ui.label(match filter.filter {
                            Filter::Invert => "Invert",
                            Filter::Brightness { .. } => "Brightness",
                        });
                        if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                            swap = Some(index);
                        }
                        if ui
                            .small_button("Copy")
                            .on_hover_text("Duplicate filter")
                            .clicked()
                        {
                            duplicate = Some(index);
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text("Remove filter")
                            .clicked()
                        {
                            remove = Some(index);
                        }
                    });
                    if let Filter::Brightness { amount } = &mut filter.filter {
                        changed |= ui
                            .add(egui::Slider::new(amount, -1.0..=1.0).text("Amount"))
                            .changed();
                    }
                }
                if let Some(index) = swap {
                    filters.swap(index, index - 1);
                    changed = true;
                }
                if let Some(index) = remove {
                    filters.remove(index);
                    changed = true;
                }
                if let Some(index) = duplicate {
                    let mut filter = filters[index].clone();
                    filter.id = self.current_mut().document.allocate_id();
                    filters.insert(index + 1, filter);
                    changed = true;
                }
                if changed {
                    self.current_mut().command("Edit live filter stack", |doc| {
                        doc.active_mut().filters = filters
                    });
                }
                if filters_empty(self) {
                    ui.label("No filters on this layer.");
                }
            });
        self.show_filters = open;
    }

    pub(super) fn dialogs(&mut self, ctx: &Context) {
        if let Some(id) = self.close_pending {
            let name = self
                .sessions
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            egui::Modal::new(egui::Id::unique("save-changes")).show(ctx, |ui| {
                ui.label(format!("Save changes to {name} before closing?"));
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.active = self
                            .sessions
                            .iter()
                            .position(|s| s.id == id)
                            .expect("pending tab");
                        self.save(ctx, false, false);
                        if self.busy {
                            self.close_pending = None;
                            self.close_after_save = Some(id);
                        }
                    }
                    if ui.button("Discard").clicked() {
                        self.remove_tab(id, ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        self.close_pending = None;
                        self.quitting = false;
                    }
                });
            });
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
        if self.show_filters {
            self.filter_window(ctx);
        }
    }

    pub(super) fn status(&mut self, ui: &mut Ui) {
        let selection = self.current_mut().selection_bounds();
        ui.horizontal_centered(|ui| {
            ui.label(
                self.cursor
                    .map(|p| format!("↖  {}, {} px", p[0], p[1]))
                    .unwrap_or_else(|| self.tool.label().into()),
            );
            ui.separator();
            ui.label(format!(
                "▢  {} × {} px",
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
                ui.label(format!("{:.0}%", self.current().zoom * 100.0));
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
                ui.add(egui::Label::new(&self.message).truncate());
            });
        });
    }
}

fn filters_empty(app: &PaletteApp) -> bool {
    app.current().document.active().filters.is_empty()
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
        let close_clicked = response.contains_pointer()
            && ui
                .put(close_rect, egui::Button::new("×").frame(false))
                .on_hover_text("Close document")
                .clicked();
        (response, close_clicked)
    })
    .inner
}
