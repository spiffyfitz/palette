//! Paint's compact save prompt: no backdrop dimming, a two-tone body/footer,
//! fixed button geometry, native Windows typography and keyboard navigation.
use super::*;

#[derive(Clone, Copy)]
enum Choice {
    Save,
    DontSave,
    Cancel,
}

impl PaletteApp {
    pub(super) fn close_dialog(&mut self, ctx: &Context, id: u64) {
        let Some(session) = self.sessions.iter().find(|s| s.id == id) else {
            self.close_pending = None;
            return;
        };
        let name = session.name.clone();
        let mut choice = None;
        let result = egui::Modal::new(egui::Id::unique("save-changes"))
            .area(
                egui::Modal::default_area(egui::Id::unique("save-changes"))
                    .default_size(vec2(340., 188.)),
            )
            .frame(egui::Frame::NONE)
            .backdrop_color(Color32::TRANSPARENT)
            .show(ctx, |ui| {
                ui.set_width(340.);
                let (rect, _) = ui.allocate_exact_size(vec2(340., 188.), Sense::hover());
                let p = ui.painter();
                p.rect_filled(rect, 8, Color32::from_gray(43));
                p.rect_filled(
                    egui::Rect::from_min_max(rect.min + vec2(0., 106.), rect.max),
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 8,
                        se: 8,
                    },
                    Color32::from_gray(32),
                );
                p.rect_stroke(
                    rect,
                    8,
                    egui::Stroke::new(1., Color32::from_gray(65)),
                    egui::StrokeKind::Inside,
                );
                p.text(
                    rect.min + vec2(24., 24.),
                    egui::Align2::LEFT_TOP,
                    "Do you want to save your work?",
                    egui::FontId::new(20., egui::FontFamily::Name("Dialog Semibold".into())),
                    Color32::from_gray(245),
                );
                let body = format!("There are unsaved changes in “{name}”.");
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(egui::Rect::from_min_size(
                            rect.min + vec2(24., 64.),
                            vec2(292., 20.),
                        ))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                    |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&body).font(egui::FontId::new(
                                14.,
                                egui::FontFamily::Name("Dialog".into()),
                            )))
                            .truncate(),
                        )
                    },
                )
                .inner
                .on_hover_text(body);
                let ids = ["save", "dont-save", "cancel"]
                    .map(|slot| egui::Id::unique(("close-button", id, slot)));
                let focused = ctx.memory(|m| m.focused());
                for (index, (label, action)) in [
                    ("Save", Choice::Save),
                    ("Don’t save", Choice::DontSave),
                    ("Cancel", Choice::Cancel),
                ]
                .into_iter()
                .enumerate()
                {
                    let button = egui::Rect::from_min_size(
                        rect.min + vec2(24. + index as f32 * 100., 130.),
                        vec2(92., 32.),
                    );
                    let response = ui.interact(button, ids[index], Sense::click());
                    response
                        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                    if index == 0 && focused.is_none_or(|f| !ids.contains(&f)) {
                        response.request_focus();
                    }
                    let fill = if index == 0 {
                        Color32::from_rgb(41, 247, 255)
                    } else {
                        Color32::from_gray(45)
                    };
                    let fill = if response.is_pointer_button_down_on() {
                        fill.gamma_multiply(0.8)
                    } else if response.hovered() && index != 0 {
                        Color32::from_gray(58)
                    } else {
                        fill
                    };
                    ui.painter().rect_filled(button, 4, fill);
                    if response.has_focus() && index != 0 {
                        ui.painter().rect_stroke(
                            button.shrink(2.),
                            3,
                            egui::Stroke::new(1., Color32::WHITE),
                            egui::StrokeKind::Inside,
                        );
                    }
                    ui.painter().text(
                        button.center(),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::new(14., egui::FontFamily::Name("Dialog".into())),
                        if index == 0 {
                            Color32::BLACK
                        } else {
                            Color32::from_gray(245)
                        },
                    );
                    if response.clicked()
                        || (response.has_focus()
                            && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space)))
                    {
                        choice = Some(action);
                    }
                }
            });
        if result.is_top_modal
            && !result.any_popup_open
            && ctx.input(|i| i.key_pressed(Key::Escape))
        {
            choice = Some(Choice::Cancel);
        }
        match choice {
            Some(Choice::Save) => {
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
            Some(Choice::DontSave) => self.remove_tab(id, ctx),
            Some(Choice::Cancel) => {
                self.close_pending = None;
                self.quitting = false;
            }
            None => {}
        }
    }
}
