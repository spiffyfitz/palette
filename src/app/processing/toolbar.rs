//! Paint-sized contextual processing bar. Drafts are presentation-only: the
//! canvas samples the worker's preview texture until Apply commits parameters.
use super::*;
use palette_core::document::Effect;

enum Intent {
    Close,
    Import,
    Add(Kind),
    Edit(Kind, u64),
    Action(Action),
    Apply,
    Cancel,
}

impl PaletteApp {
    pub(crate) fn processing_section_is(&self, section: Section) -> bool {
        self.processing_toolbar.as_ref().is_some_and(|t| {
            t.section == section
                && t.session == self.current().id
                && t.layer == self.current().document.active_layer
        })
    }

    pub(crate) fn open_processing_toolbar(&mut self, section: Section) {
        self.cancel_processing();
        if !self.close_pending_none() {
            return;
        }
        self.current_mut().end_gesture();
        self.stroke = None;
        self.processing_toolbar = Some(Toolbar {
            session: self.current().id,
            layer: self.current().document.active_layer,
            section,
            selected: None,
        });
    }

    pub(crate) fn toggle_processing_toolbar(&mut self, section: Section) {
        if self.processing_section_is(section) {
            self.cancel_processing();
            self.processing_toolbar = None;
        } else {
            self.open_processing_toolbar(section);
        }
    }

    /// Presentation never replaces the committed raster or the saved model.
    pub(crate) fn processing_preview(&self) -> Option<&TextureHandle> {
        self.processing_dialog
            .as_ref()
            .filter(|d| {
                d.preview
                    && d.session == self.current().id
                    && d.generation == self.current().generation
                    && d.layer == self.current().document.active_layer
            })
            .and_then(|d| d.image.as_ref())
    }

    pub(crate) fn processing_toolbar_window(&mut self, ctx: &Context) {
        self.processing_toolbar_rect = egui::Rect::NOTHING;
        let Some(mut toolbar) = self.processing_toolbar.take() else {
            return;
        };
        if toolbar.session != self.current().id
            || toolbar.layer != self.current().document.active_layer
        {
            self.cancel_processing();
            return;
        }
        if self
            .processing_dialog
            .as_ref()
            .is_some_and(|d| d.generation != self.current().generation)
        {
            self.cancel_processing();
        }
        let mut draft = self.processing_dialog.take();
        let shadow = draft
            .as_ref()
            .is_some_and(|d| matches!(d.operation.kind(), Kind::DropShadow | Kind::InnerShadow));
        let compact = self.canvas_rect.width() < if shadow { 1050. } else { 700. };
        let mut intent = None;
        let mut changed = false;
        if let Some(d) = &mut draft {
            match assets::poll(d) {
                Ok(value) => {
                    changed |= value;
                    if value && let Some(n) = &d.import_notice {
                        self.message = n.clone();
                    }
                }
                Err(e) => self.error = Some(format!("Could not load filter asset: {e}")),
            }
        }

        let popup_was_open = egui::Popup::is_any_open(ctx);
        let available = self.close_pending_none() && !self.current().document.active().locked;
        let bar = egui::Area::new(egui::Id::unique("processing-toolbar"))
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
                crate::interface::context_toolbar_frame(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().interact_size.y = crate::interface::CONTEXT_CONTROL_HEIGHT;
                    ui.spacing_mut().item_spacing.x = if compact { 4. } else { 6. };
                    ui.horizontal(|ui| {
                        ui.set_min_height(crate::interface::CONTEXT_CONTROL_HEIGHT);
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::splat(24.), Sense::hover());
                        if draft
                            .as_ref()
                            .is_some_and(|d| d.ready != Some(d.ticket) || d.import.is_some())
                        {
                            // Paint-only status keeps control IDs stable when a job
                            // completes during a pointer gesture or an open menu.
                            let phase = ctx.input(|i| i.time) as f32 * 4.;
                            let points = (0..16)
                                .map(|i| {
                                    let angle = phase + i as f32 * 0.25;
                                    rect.center() + vec2(angle.cos(), angle.sin()) * 7.
                                })
                                .collect();
                            ui.painter()
                                .add(egui::Shape::line(points, egui::Stroke::new(1.5, ACCENT)));
                            ctx.request_repaint_after(Duration::from_millis(16));
                        } else {
                            icons::draw(ui, rect.shrink(4.), toolbar.section.icon(), ACCENT);
                        }
                        response.on_hover_text(toolbar.section.label());
                        if let Some(d) = &mut draft {
                            changed |=
                                editor_controls(self, ui, d, available, compact, &mut intent);
                        } else {
                            ui.add_enabled_ui(available, |ui| {
                                stack_controls(self, ui, &mut toolbar, compact, &mut intent);
                            });
                            if ui
                                .small_button("×")
                                .on_hover_text("Close processing toolbar")
                                .clicked()
                            {
                                intent = Some(Intent::Close);
                            }
                        }
                    });
                });
            });
        self.processing_toolbar_rect = bar.response.rect;
        // Escape first dismisses a parameter/color/action popup. The same key
        // must not also cancel its editor after egui closes that popup.
        let escape = !popup_was_open
            && !egui::Popup::is_any_open(ctx)
            && ctx.input(|i| i.key_pressed(Key::Escape));
        if let Some(d) = &mut draft {
            if changed {
                d.ticket = self
                    .processing_worker
                    .ticket
                    .fetch_add(1, Ordering::Relaxed)
                    + 1;
            }
            if escape {
                intent = Some(Intent::Cancel);
            }
        } else if escape {
            intent = Some(Intent::Close);
        }
        self.processing_dialog = draft;
        self.processing_toolbar = Some(toolbar);
        if let Some(intent) = intent {
            match intent {
                Intent::Close => {
                    self.cancel_processing();
                    self.processing_toolbar = None;
                }
                Intent::Import => self.import_processing_asset(ctx),
                Intent::Cancel => self.cancel_processing(),
                Intent::Apply => self.apply_processing(),
                Intent::Add(kind) => self.open_processing(kind, None),
                Intent::Edit(kind, id) => self.open_processing(kind, Some(id)),
                Intent::Action(action) => {
                    self.cancel_processing();
                    let effects = self
                        .processing_toolbar
                        .as_ref()
                        .expect("toolbar")
                        .section
                        .effects();
                    self.processing_action(effects, action);
                }
            }
        }
    }
}

fn editor_controls(
    app: &PaletteApp,
    ui: &mut Ui,
    d: &mut Dialog,
    enabled: bool,
    compact: bool,
    intent: &mut Option<Intent>,
) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        if !compact {
            ui.label(d.operation.kind().descriptor().label);
        }
        let mut changed = parameter_control(ui, d, compact);
        let movable=!matches!(d.operation,Operation::Filter(Filter::LightingEffects {kind:palette_core::document::LightKind::Directional,..}));
        if matches!(&d.operation, Operation::Filter(f) if f.center().is_some())
            && icons::button(
                ui, Icon::Move,
                "Place effect origin: click or drag on the layer; Space or middle-drag pans. Origin X/Y accept exact values.",
                d.pick_origin, movable, Vec2::splat(24.),
            ).clicked()
        {
            d.pick_origin = !d.pick_origin;
            d.origin_drag = false;
        }
        let selected_parameter = d
            .operation
            .kind()
            .descriptor()
            .parameters
            .get(d.parameter)
            .map(|p| p.key);
        if selected_parameter != Some("rounded")
            && let Operation::Effect(effect)=&mut d.operation
            && let Effect::Stroke { rounded, .. }=effect.base_mut()
        {
            changed |= ui
                .checkbox(rounded, if compact { "Round" } else { "Rounded corners" })
                .changed();
        }
        if !compact && matches!(d.operation.kind(),Kind::DropShadow|Kind::InnerShadow)
        {
            let selected = d.operation.kind().descriptor().parameters[d.parameter].key;
            for (key, label, max, scale) in [
                ("angle", "Angle", 180., 1.),
                ("distance", "Distance", 256., 1.),
                ("spread", "Spread", 100., 100.),
            ] {
                if selected == key {
                    continue;
                }
                if key=="angle" && d.operation.value("global_light")==Some(Value::Boolean(true)){continue;}
                changed |= ui
                    .push_id(("shadow-quick", key), |ui| {
                        let Some(Value::Number(n)) = d.operation.value(key) else {
                            return false;
                        };
                        let mut value = n * scale;
                        ui.label(label);
                        let response = ui.add(
                            egui::DragValue::new(&mut value)
                                .range(if key == "angle" {
                                    -max..=max
                                } else {
                                    0. ..=max
                                })
                                .speed(if key == "angle" { 1. } else { 0.1 })
                                .max_decimals(if key == "angle" { 0 } else { 1 })
                                .suffix(if key == "spread" { "%" } else { "" }),
                        );
                        response.changed()
                            && d.operation
                                .set_value(key, Value::Number(value / scale))
                                .is_ok()
                    })
                    .inner;
            }
        }
        changed |= ui
            .push_id("processing-preview", |ui| {
                ui.checkbox(&mut d.preview, if compact { "" } else { "Preview" })
                    .on_hover_text("Preview changes on the canvas")
                    .changed()
            })
            .inner;
        let apply = ui
            .push_id("processing-apply", |ui| {
                ui.add_enabled(d.import.is_none(),
                    egui::Button::new(egui::RichText::new("Apply").color(Color32::BLACK))
                        .fill(ACCENT),
                )
            })
            .inner;
        if apply.clicked() {
            *intent = Some(Intent::Apply);
        }
        if ui
            .push_id("processing-cancel", |ui| ui.button("Cancel"))
            .inner
            .clicked()
        {
            *intent = Some(Intent::Cancel);
        }
        ui.push_id("processing-edit-actions", |ui| {
            let menu=ui.menu_button("…", |ui| {
                ui.label(d.operation.kind().descriptor().label);
                let (style_changed,load_pattern)=super::style_editors::controls(app,ui,d);
                changed|=style_changed;
                if load_pattern{*intent=Some(Intent::Import);}
                match &d.operation {
                    Operation::Filter(Filter::ColorLookup {lut:Some(l),..})=>{ui.label(format!("{} ({}³)",l.name,l.size));},
                    Operation::Filter(Filter::Displace {map:Some(m),map_name,..})=>{ui.label(format!("{} ({} × {})",map_name,m.width(),m.height()));},
                    _=>{},
                }
                if matches!(d.operation.kind(),Kind::ColorLookup|Kind::Displace) && ui.add_enabled(d.import.is_none(),egui::Button::new(match d.operation.kind(){Kind::ColorLookup=>"Load 3D LUT…",_=>"Load displacement map…"})).clicked() {*intent=Some(Intent::Import);ui.close();}
                if let Some(notice)=&d.import_notice {ui.label(notice);}
                if d.operation.kind()==Kind::Custom {let sub=ui.menu_button("Edit kernel…",|ui|{changed|=editors::kernel(ui,&mut d.operation);});app.capture_submenu("processing-custom-kernel",ui,&sub.response);}
                if d.operation.kind()==Kind::Shear {let sub=ui.menu_button("Edit shear curve…",|ui|{changed|=editors::curve(ui,&mut d.operation);});app.capture_submenu("processing-shear-curve",ui,&sub.response);}

                if matches!(&d.operation, Operation::Filter(f) if f.center().is_some()) && ui.button("Reset origin").clicked() {
                    let defaults = d.operation.kind().default_operation();
                    for key in ["center_x", "center_y"] { d.operation.set_value(key, defaults.value(key).expect("origin")).expect("default origin"); }
                    changed = true;
                    ui.close();
                }
                if ui
                    .add_enabled(d.instance.is_some(), egui::Button::new("Remove"))
                    .on_hover_text("Remove this filter/effect; source pixels are preserved")
                    .clicked()
                {
                    *intent = d.instance.map(|id| Intent::Action(Action::Delete(id)));
                    ui.close();
                }
            });
            app.capture_popup("processing-custom-kernel",&menu.response);
            app.capture_popup("processing-shear-curve",&menu.response);
            for scene in ["style-gradient-editor","style-gloss-editor","style-contour-editor"]{app.capture_popup(scene,&menu.response);}
        });
        changed
    })
    .inner
}

/// One parameter at a time keeps even six-parameter styles at the same toolbar
/// height as Text/Selection. Every setting remains available in the selector.
fn parameter_control(ui: &mut Ui, d: &mut Dialog, compact: bool) -> bool {
    let parameters = d.operation.kind().descriptor().parameters;
    if parameters.is_empty() {
        ui.label("No parameters");
        return false;
    }
    d.parameter = d.parameter.min(parameters.len() - 1);
    let selector = egui::ComboBox::from_id_salt("processing-parameter")
        .selected_text(parameters[d.parameter].label)
        .wrap_mode(if matches!(d.operation, Operation::Effect(_)) {
            egui::TextWrapMode::Truncate
        } else {
            ui.wrap_mode()
        })
        .width(if compact { 102. } else { 124. })
        .height(200.)
        .show_ui(ui, |ui| {
            for (index, p) in parameters.iter().enumerate() {
                ui.selectable_value(&mut d.parameter, index, p.label);
            }
        });
    selector.response.on_hover_text("Choose a setting to edit");
    let p = parameters[d.parameter];
    ui.push_id(p.key, |ui| {
        let stochastic = !(p.key == "seed"
            && matches!(
                d.operation,
                Operation::Filter(Filter::Diffuse {
                    mode: palette_core::document::DiffuseMode::Anisotropic,
                    ..
                })
            ));
        let channel_active = !matches!(
            d.operation,
            Operation::Filter(Filter::ChannelMixer {
                monochrome: true,
                ..
            })
        ) || !(p.key.starts_with("green_") || p.key.starts_with("blue_"));
        let context_active = match &d.operation {
            Operation::Effect(Effect::GradientOverlay {
                style: palette_core::document::GradientStyle::Angle,
                ..
            }) if p.key == "scale" => false,
            Operation::Effect(Effect::BevelEmboss { texture: false, .. })
                if matches!(p.key, "surface" | "texture_scale" | "texture_depth") =>
            {
                false
            }
            Operation::Filter(Filter::LightingEffects { kind, .. }) => match kind {
                palette_core::document::LightKind::Directional => {
                    !matches!(p.key, "radius" | "center_x" | "center_y")
                }
                palette_core::document::LightKind::Point => p.key != "angle",
                palette_core::document::LightKind::Spot => true,
            },
            Operation::Filter(Filter::LensBlur {
                depth_source: palette_core::document::DepthSource::Uniform,
                ..
            }) => !matches!(p.key, "focus" | "invert"),
            Operation::Filter(Filter::OilPaint {
                lighting: false, ..
            }) => !matches!(p.key, "angle" | "shine" | "bristles" | "scale"),
            Operation::Filter(Filter::Extrude { random: false, .. }) => p.key != "seed",
            Operation::Filter(Filter::Tiles { fill, .. }) => match p.key {
                "low" => *fill == palette_core::document::TileFill::Foreground,
                "high" => *fill == palette_core::document::TileFill::Background,
                _ => true,
            },
            _ => true,
        };
        let style_active = match &d.operation {
            Operation::Effect(e) => {
                palette_core::processing::style_options::parameter_enabled(e, p.key)
            }
            _ => true,
        };
        ui.add_enabled_ui(
            stochastic && channel_active && context_active && style_active,
            |ui| {
                match (
                    p.kind,
                    d.operation.value(p.key).expect("registered parameter"),
                ) {
                    (ParameterType::Number { min, max, step }, Value::Number(mut value)) => {
                        let (mut min, mut max) = (min, max);
                        if let Operation::Filter(Filter::Levels { black, white, .. }) = d.operation
                        {
                            if p.key == "black" {
                                max = max.min(f32::from(white) - 1.);
                            }
                            if p.key == "white" {
                                min = min.max(f32::from(black) + 1.);
                            }
                        }
                        let adjustment_percent = matches!(
                            d.operation,
                            Operation::Filter(
                                Filter::ColorBalance { .. }
                                    | Filter::ChannelMixer { .. }
                                    | Filter::SelectiveColor { .. }
                            )
                        ) || matches!(
                            p.key,
                            "density"
                                | "midpoint"
                                | "shadows"
                                | "highlights"
                                | "shadow_width"
                                | "highlight_width"
                        );
                        let painting_percent = matches!(
                            d.operation.kind(),
                            Kind::Chrome
                                | Kind::FieldBlur
                                | Kind::IrisBlur
                                | Kind::TiltShift
                                | Kind::PathBlur
                                | Kind::SpinBlur
                                | Kind::Flame
                                | Kind::PictureFrame
                                | Kind::Tree
                                | Kind::NtscColors
                                | Kind::Deinterlace
                                | Kind::HsbHsl
                                | Kind::ConteCrayon
                                | Kind::Plaster
                                | Kind::Reticulation
                                | Kind::TornEdges
                                | Kind::WaterPaper
                                | Kind::Fresco
                                | Kind::NeonGlow
                                | Kind::PaletteKnife
                                | Kind::PlasticWrap
                                | Kind::RoughPastels
                                | Kind::Sponge
                                | Kind::Underpainting
                                | Kind::Watercolor
                                | Kind::DiffuseGlow
                                | Kind::Glass
                                | Kind::OceanRipple
                                | Kind::Texturizer
                                | Kind::ShapeBlur
                                | Kind::SmartBlur
                                | Kind::SharpenEdges
                                | Kind::SmartSharpen
                                | Kind::Extrude
                                | Kind::OilPaint
                                | Kind::Tiles
                                | Kind::TraceContour
                                | Kind::Wind
                                | Kind::LensBlur
                                | Kind::LensFlare
                                | Kind::LightingEffects
                                | Kind::ColorLookup
                                | Kind::Displace
                                | Kind::Shear
                                | Kind::Custom
                                | Kind::SprayedStrokes
                                | Kind::SumiE
                                | Kind::BasRelief
                                | Kind::ChalkCharcoal
                                | Kind::Charcoal
                                | Kind::GraphicPen
                                | Kind::NotePaper
                                | Kind::Stamp
                                | Kind::ColoredPencil
                                | Kind::DryBrush
                                | Kind::PaintDaubs
                                | Kind::SmudgeStick
                                | Kind::AngledStrokes
                                | Kind::Crosshatch
                                | Kind::DarkStrokes
                                | Kind::Spatter
                        ) && matches!(
                            p.kind,
                            ParameterType::Number {
                                min: 0.,
                                max: 1.,
                                ..
                            }
                        );
                        let percent = matches!(
                            p.key,
                            "style_noise" | "style_jitter" | "style_range" | "gradient_smoothness"
                        ) || ((d.operation.kind() == Kind::SmartSharpen
                            && p.key == "amount")
                            || painting_percent
                            || adjustment_percent
                            || matches!(
                                p.key,
                                "center_x"
                                    | "center_y"
                                    | "opacity"
                                    | "spread"
                                    | "brightness"
                                    | "contrast"
                                    | "saturation"
                                    | "lightness"
                                    | "strength"
                                    | "preserve_details"
                                    | "color_noise"
                                    | "sharpen"
                                    | "grain"
                                    | "highlight"
                                    | "fidelity"
                                    | "dark"
                                    | "light"
                                    | "detail"
                                    | "darkness"
                                    | "depth"
                                    | "dot"
                            )
                            || (p.key == "intensity"
                                && matches!(
                                    d.operation,
                                    Operation::Filter(Filter::FilmGrain { .. })
                                )))
                            && !(d.operation.kind() == Kind::BevelEmboss && p.key == "depth")
                            || (d.operation.kind() == Kind::BevelEmboss
                                && matches!(
                                    p.key,
                                    "highlight_opacity" | "shadow_opacity" | "texture_depth"
                                ));
                        let scale = if percent { 100. } else { 1. };
                        value *= scale;
                        ui.spacing_mut().slider_width = if compact { 60. } else { 100. };
                        let display_step = step * scale;
                        let decimals = if display_step >= 1. {
                            0
                        } else if display_step >= 0.1 {
                            1
                        } else {
                            2
                        };
                        let mut slider = egui::Slider::new(&mut value, min * scale..=max * scale)
                            .step_by(f64::from(display_step))
                            .max_decimals(decimals);
                        if percent
                            || (d.operation.kind() == Kind::BevelEmboss && p.key == "depth")
                            || (d.operation.kind() == Kind::GradientOverlay && p.key == "scale")
                            || p.key == "material_scale"
                        {
                            slider = slider.suffix("%");
                        } else if matches!(
                            d.operation.kind(),
                            Kind::BevelEmboss
                                | Kind::Satin
                                | Kind::GradientOverlay
                                | Kind::PatternOverlay
                        ) && matches!(
                            p.key,
                            "angle"
                                | "altitude"
                                | "global_angle"
                                | "global_altitude"
                                | "material_angle"
                        ) {
                            slider = slider.suffix("°");
                        }
                        let response = ui.add(slider).on_hover_text(p.label);
                        let mut accepted = false;
                        if response.changed() {
                            // Integer model fields cannot receive fractional typed input.
                            let number = if step >= 1. {
                                (value / scale).round()
                            } else {
                                value / scale
                            };
                            let before = d.operation.value(p.key);
                            // Percentage display conversion can report a slider change
                            // on idle frames (e.g. 0.6 × 100). Only a real model change
                            // may invalidate the preview and cancel its worker ticket.
                            accepted = d.operation.set_value(p.key, Value::Number(number)).is_ok()
                                && d.operation.value(p.key) != before;
                        }
                        accepted
                    }
                    (ParameterType::Color, Value::Color(mut color)) => {
                        let changed = ui.color_edit_button_srgb(&mut color).changed();
                        if changed {
                            d.operation
                                .set_value(p.key, Value::Color(color))
                                .expect("color");
                        }
                        changed
                    }
                    (ParameterType::Choice(options), Value::Choice(mut selected)) => {
                        let before = selected;
                        egui::ComboBox::from_id_salt("processing-choice")
                            .width(88.)
                            .truncate()
                            .selected_text(if compact && p.key == "to_polar" {
                                if selected == 0 {
                                    "To Polar"
                                } else {
                                    "To Rectangular"
                                }
                            } else {
                                options[selected]
                            })
                            .show_ui(ui, |ui| {
                                for (index, label) in options.iter().enumerate() {
                                    ui.selectable_value(&mut selected, index, *label);
                                }
                            });
                        if selected != before {
                            d.operation
                                .set_value(p.key, Value::Choice(selected))
                                .expect("registered choice");
                        }
                        selected != before
                    }
                    (ParameterType::Boolean, Value::Boolean(mut value)) => {
                        let label = if d.operation.kind() == Kind::NtscColors {
                            "7.5 IRE"
                        } else if compact
                            && (matches!(d.operation, Operation::Effect(_))
                                || p.key == "preserve_luminosity")
                        {
                            "On"
                        } else {
                            p.label
                        };
                        let changed = ui
                            .checkbox(&mut value, label)
                            .on_hover_text(p.label)
                            .changed();
                        if changed {
                            d.operation
                                .set_value(p.key, Value::Boolean(value))
                                .expect("boolean");
                            if p.key == "global_light"
                                && value
                                && let Operation::Effect(e) = &mut d.operation
                            {
                                e.options_mut().global_light = d.original.global_light;
                            }
                        }
                        changed
                    }
                    _ => unreachable!("registered parameter type"),
                }
            },
        )
        .inner
    })
    .inner
}

fn stack_controls(
    app: &PaletteApp,
    ui: &mut Ui,
    toolbar: &mut Toolbar,
    compact: bool,
    intent: &mut Option<Intent>,
) {
    let layer = app.current().document.active();
    let nodes: Vec<_> = if toolbar.section.effects() {
        layer
            .effects
            .iter()
            .map(|e| (e.id, e.enabled, Operation::Effect(e.effect.clone()), None))
            .collect()
    } else {
        layer
            .filters
            .iter()
            .map(|f| {
                (
                    f.id,
                    f.enabled,
                    Operation::Filter(f.filter.clone()),
                    f.mask.as_ref().map(|m| m.enabled),
                )
            })
            .collect()
    };
    let selected = nodes
        .iter()
        .position(|n| Some(n.0) == toolbar.selected)
        .or_else(|| (!nodes.is_empty()).then_some(0));
    toolbar.selected = selected.map(|i| nodes[i].0);
    let name = selected.map_or_else(
        || format!("No {}", toolbar.section.label().to_lowercase()),
        |i| {
            format!(
                "{}. {}{}",
                i + 1,
                nodes[i].2.kind().descriptor().label,
                if nodes[i].1 { "" } else { " (off)" }
            )
        },
    );
    egui::ComboBox::from_id_salt("processing-instance")
        .selected_text(name)
        .width(if compact { 145. } else { 200. })
        .truncate()
        .show_ui(ui, |ui| {
            for (i, (id, enabled, operation, _)) in nodes.iter().enumerate() {
                ui.selectable_value(
                    &mut toolbar.selected,
                    Some(*id),
                    format!(
                        "{}. {}{}",
                        i + 1,
                        operation.kind().descriptor().label,
                        if *enabled { "" } else { " (off)" }
                    ),
                );
            }
        });
    ui.menu_button("Add", |ui| {
        egui::ScrollArea::vertical()
            .max_height((ui.ctx().content_rect().height() - 100.).clamp(128., 400.))
            .show(ui, |ui| {
                for descriptor in REGISTRY {
                    if matches!(descriptor.kind.default_operation(), Operation::Effect(_))
                        == toolbar.section.effects()
                        && ui.button(descriptor.label).clicked()
                    {
                        *intent = Some(Intent::Add(descriptor.kind));
                        ui.close();
                    }
                }
            });
    });
    if let Some(index) = nodes.iter().position(|n| Some(n.0) == toolbar.selected) {
        let (id, mut enabled, op, mask) = nodes[index].clone();
        if ui
            .checkbox(&mut enabled, "On")
            .on_hover_text("Enable or disable this instance")
            .changed()
        {
            *intent = Some(Intent::Action(Action::Toggle(id, enabled)));
        }
        if ui.button("Edit").clicked() {
            *intent = Some(Intent::Edit(op.kind(), id));
        }
        ui.menu_button("…", |ui| {
            for (label, action, allowed) in [
                ("Move up", Action::Move(id, -1), index > 0),
                ("Move down", Action::Move(id, 1), index + 1 < nodes.len()),
                ("Duplicate", Action::Duplicate(id), true),
                ("Remove", Action::Delete(id), true),
            ] {
                if ui.add_enabled(allowed, egui::Button::new(label)).clicked() {
                    *intent = Some(Intent::Action(action));
                    ui.close();
                }
            }
            if !toolbar.section.effects() {
                ui.separator();
                for (label, action, allowed) in [
                    (
                        "Mask from selection",
                        MaskAction::FromSelection,
                        app.current().document.selection.is_some(),
                    ),
                    (
                        if mask == Some(false) {
                            "Enable mask"
                        } else {
                            "Disable mask"
                        },
                        MaskAction::Toggle,
                        mask.is_some(),
                    ),
                    ("Remove mask", MaskAction::Remove, mask.is_some()),
                ] {
                    if ui.add_enabled(allowed, egui::Button::new(label)).clicked() {
                        *intent = Some(Intent::Action(Action::Mask(id, action)));
                        ui.close();
                    }
                }
            }
        });
    }
}
