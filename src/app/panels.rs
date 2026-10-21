use super::*;

struct TabDrag(u64);

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
                    // Allocate only the remaining width: overflowing tabs may
                    // scroll, but must never push menus or the ribbon toggle away.
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| self.tabs(ui),
                    );
                });
            });
    }

    pub(super) fn tabs(&mut self, ui: &mut Ui) {
        let mut close = None;
        let mut reorder = None;
        let mut activate = None;
        let active_id = self.current().id;
        let reveal_active = self.tab_in_view.is_none_or(|(id, _, _)| id != active_id);
        let mut active_rect = egui::Rect::NOTHING;
        let edge_scroll_id = egui::Id::unique("document-tabs-edge-scroll");
        let edge_offset = ui.ctx().data_mut(|d| d.remove_temp::<f32>(edge_scroll_id));
        let mut area = egui::ScrollArea::horizontal()
            .id_salt("tabs-scroll")
            .max_height(28.0)
            .min_scrolled_height(28.0)
            .auto_shrink([false, false])
            .scroll_source(egui::scroll_area::ScrollSource {
                drag: egui::scroll_area::DragScroll::Never,
                ..Default::default()
            });
        if let Some(offset) = edge_offset {
            area = area.horizontal_scroll_offset(offset);
        }
        let scroll = area.show(ui, |ui| {
            // A floating rail overlays the bottom of the fixed-height row
            // instead of creating a second strip when documents overflow.
            ui.horizontal_centered(|ui| {
                for (index, session) in self.sessions.iter().enumerate() {
                    let label = format!(
                        "{}{}",
                        session.name,
                        if session.modified() { " •" } else { "" }
                    );
                    let (response, close_clicked) =
                        document_tab(ui, session.id, &label, index == self.active);
                    if session.id == active_id {
                        active_rect = response.rect;
                    }
                    if reveal_active && session.id == active_id {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
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
        // Font/DPI changes and egui's sizing pass can settle after activation.
        // Recheck when layout changes, but never fight manual scrolling while
        // the tab list and viewport retain the same size.
        let content_width = scroll.content_size.x;
        let viewport_width = scroll.inner_rect.width();
        if self.tab_in_view.is_some_and(|(_, content, viewport)| {
            (content - content_width).abs() > 1.0 || (viewport - viewport_width).abs() > 1.0
        }) {
            self.tab_in_view = None;
            ui.ctx().request_repaint();
        } else if reveal_active && scroll.inner_rect.contains(active_rect.center()) {
            self.tab_in_view = Some((active_id, content_width, viewport_width));
        }
        ui.interact(
            scroll.inner_rect,
            egui::Id::unique("document-tabs-viewport"),
            Sense::hover(),
        );
        // Keep offscreen destinations reachable while reordering a long list.
        // Only the tab viewport scrolls; menus and window chrome stay fixed.
        if egui::DragAndDrop::payload::<TabDrag>(ui.ctx()).is_some()
            && let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
            && scroll.inner_rect.contains(pointer)
        {
            const EDGE_WIDTH: f32 = 24.0;
            const SCROLL_SPEED: f32 = 360.0;
            let direction = if pointer.x < scroll.inner_rect.left() + EDGE_WIDTH {
                -1.0
            } else if pointer.x > scroll.inner_rect.right() - EDGE_WIDTH {
                1.0
            } else {
                0.0
            };
            let current_offset = scroll.state.clamped_offset().x;
            let max_offset = (scroll.content_size.x - scroll.inner_rect.width()).max(0.0);
            let offset = (current_offset
                + direction * SCROLL_SPEED * ui.input(|i| i.stable_dt.min(0.05)))
            .clamp(0.0, max_offset);
            if offset != current_offset {
                ui.ctx().data_mut(|d| d.insert_temp(edge_scroll_id, offset));
                ui.ctx().request_repaint();
            }
        }
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

    pub(super) fn dialogs(&mut self, ctx: &Context) {
        self.merge_layers_window(ctx);
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
                if session.requested == Some(session.generation) && !self.display_is_current() {
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
        let frame = egui::Frame::new()
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
            .response;
        let response = ui.interact(
            frame.rect,
            egui::Id::unique(("document-tab", id)),
            Sense::click_and_drag(),
        );
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
