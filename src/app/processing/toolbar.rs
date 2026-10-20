//! Paint-sized contextual processing bar. Drafts are presentation-only: the
//! canvas samples the worker's preview texture until Apply commits parameters.
use super::*;
use palette_core::document::Effect;

enum Intent {
    ExportZoom,
    CancelZoomExport,
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
        // Reference-photo filters also reserve space for their direct photo
        // picker; switch before the full bar reaches the canvas boundaries.
        let reference_photo = draft.as_ref().is_some_and(|d| {
            matches!(
                d.operation.kind(),
                Kind::ColorTransfer
                    | Kind::MakeupTransfer
                    | Kind::Harmonization
                    | Kind::LandscapeMixer
            )
        });
        let compact = self.canvas_rect.width()
            < if shadow {
                1050.
            } else if reference_photo {
                840.
            } else {
                700.
            };
        let mut intent = None;
        let mut changed = false;
        if let Some(d) = &mut draft {
            if let Err(e) = landscape_brush::update(d, ctx, &self.processing_worker.ticket) {
                self.error = Some(format!("Could not refine Landscape Mixer: {e}"));
            }
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
                    ui.spacing_mut().item_spacing.x = if compact
                        && draft
                            .as_ref()
                            .is_some_and(|d| d.operation.kind() == Kind::LandscapeMixer)
                    {
                        2.
                    } else if compact {
                        4.
                    } else {
                        6.
                    };
                    ui.horizontal(|ui| {
                        ui.set_min_height(crate::interface::CONTEXT_CONTROL_HEIGHT);
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::splat(24.), Sense::hover());
                        if draft.as_ref().is_some_and(|d| {
                            d.ready != Some(d.ticket)
                                || d.import.is_some()
                                || d.protection.as_ref().is_some_and(|p| p.awaiting_samples())
                        }) {
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
            changed |=
                landscape::reference_window(d, ctx, self.canvas_rect, self.processing_toolbar_rect);
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
                Intent::ExportZoom => self.export_whole_zoom(ctx),
                Intent::CancelZoomExport => self.cancel_zoom_export(),
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
        if d.protection.as_ref().is_some_and(|p| p.editing) {
            return landscape_brush::controls(ui,d,compact);
        }
        if !compact {
            ui.label(d.operation.kind().descriptor().label);
        }
        let mut changed = parameter_control(ui, d, compact);
        let reference = match &d.operation {
            Operation::Filter(Filter::LandscapeMixer {reference,reference_name,..}) => Some((reference_name,reference.as_ref().map_or(0,Raster::width),reference.as_ref().map_or(0,Raster::height))),
            Operation::Filter(Filter::ColorTransfer {reference,..}) => Some((&reference.name,reference.width,reference.height)),
            Operation::Filter(Filter::MakeupTransfer {reference,..}) => Some((&reference.name,reference.width,reference.height)),
            _=>None,
        };
        if let Some((name,width,height)) = reference {
            let tooltip = if d.operation.kind() == Kind::LandscapeMixer {
                format!("Blend a reference landscape; align Source horizon and Reference horizon\nCurrent: {} ({} × {})",if name.is_empty(){"None — season controls only"}else{name},width,height)
            } else {format!(
                "Load a reference photo to use its color grade\nCurrent reference: {} ({} × {})",
                name, width, height
            )};
            let response = ui
                .push_id("color-transfer-reference-photo", |ui| {
                    if compact {
                        icons::button(
                            ui,
                            Icon::Open,
                            &tooltip,
                            false,
                            d.import.is_none(),
                            Vec2::splat(24.),
                        )
                    } else {
                        ui.add_enabled(d.import.is_none(), egui::Button::new("Reference photo…"))
                            .on_hover_text(&tooltip)
                    }
                })
                .inner;
            if response.clicked() {
                *intent = Some(Intent::Import);
            }
        }
        let movable=!matches!(d.operation,Operation::Filter(Filter::LightingEffects {kind:palette_core::document::LightKind::Directional,..} | Filter::DepthBlur {pick_focus:false,..}));
        if matches!(&d.operation, Operation::Filter(f) if f.center().is_some() || matches!(f, Filter::LandscapeMixer { .. }))
            && icons::button(
                ui, Icon::Move,
                if d.operation.kind() == Kind::LandscapeMixer {"Place source horizon: click or drag vertically on the layer; Space or middle-drag pans. Use Reference horizon to align the reference photo."} else {"Place effect origin: click or drag on the layer; Space or middle-drag pans. Origin X/Y accept exact values."},
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
            && let Operation::Effect(palette_core::document::Effect::Stroke { rounded, .. }) =
                &mut d.operation
        {
            changed |= ui
                .checkbox(rounded, if compact { "Round" } else { "Rounded corners" })
                .changed();
        }
        if !compact
            && matches!(
                d.operation,
                Operation::Effect(
                    palette_core::document::Effect::DropShadow { .. }
                        | palette_core::document::Effect::InnerShadow { .. }
                )
            )
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
                ui.add_enabled(d.import.is_none() && d.reference_ready() && d.protection.as_ref().is_none_or(|p| !p.pending()),
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
                if let Operation::Filter(Filter::LandscapeMixer{reference,reference_name,..}) = &d.operation {
                    ui.set_max_width(380.);
                    ui.label("Edge-aware sky replacement · terrain colour transfer");
                    ui.small("Edge-aware mode follows detected sky around mountain ridges and vegetation. Reference terrain transfers a luminance-ranked palette while retaining the source's geometry; Detail retains source luminance. Softness affects the sky matte instead of a horizontal band. Sky detection adjusts colour/texture sensitivity. Horizons are search hints; use the separate reference preview to align its sky.");
                    ui.small("Seasons and Sunset grade both inputs before blending, including the reference preview. Refine mix lets you protect source areas the detector misclassified. New references retain up to 2048 × 2048; old Horizontal projects keep their old rendering until a new reference is loaded.");
                    if let Some(r)=reference {ui.label(format!("Reference: {} ({} × {})",reference_name,r.width(),r.height()));}
                }
                if d.operation.kind()==Kind::LandscapeMixer {changed |= ui.checkbox(&mut d.show_reference,"Reference preview").changed();}
                if let Some(p) = &mut d.protection && ui.button("Refine mix…").clicked() {
                    p.editing=true;
                    changed |= !d.preview;
                    d.preview=true;
                    d.pick_origin=false;
                    d.origin_drag=false;
                    ui.close();
                }
                if matches!(&d.operation,Operation::Filter(Filter::LandscapeMixer{reference:Some(_),..})) && ui.button("Clear reference").clicked() {
                    if let Operation::Filter(Filter::LandscapeMixer{reference,reference_name,..})=&mut d.operation { *reference=None;reference_name.clear(); }
                    changed=true;
                    ui.close();
                }
                if d.operation.kind()==Kind::SmartPortrait {
                    ui.set_max_width(380.);
                    ui.label("Local face expressions and directional shading");
                    ui.label("Happiness: left frowns, right smiles, with stronger endpoints. High positive values separate the lips and reveal teeth. Surprise opens the eyes/mouth; Anger adjusts the brows. Lighting strength enables shading; Light direction is 0° right, −90° above. Turn All faces off to edit only the largest detected face.");
                    ui.small("Visible teeth reuse source texture; closed mouths use a shaded procedural dental arch, not recovered anatomy. Strong changes can stretch texture. Age/hair/gaze/new head poses remain unavailable. Shading is conventional. No detected face leaves the image unchanged. Existing projects keep legacy rendering until an expression is edited.");
                    ui.collapsing("Model license and attribution",|ui| {
                        egui::ScrollArea::vertical().max_height(180.).show(ui,|ui| {
                            ui.add(egui::Label::new(palette_core::processing::SKIN_NOTICE).wrap());
                            ui.add(egui::Label::new(palette_core::processing::SKIN_LICENSE).wrap());
                        });
                    });
                }
                if d.operation.kind()==Kind::PhotoRestoration {
                    ui.set_max_width(380.);
                    ui.label(if d.operation.value("method")==Some(Value::Choice(0)){"Conventional local restoration · thin scratches (legacy)"}else{"Conventional local restoration · multiscale cracks"});
                    ui.small("Protect facial details uses the existing local face landmarks to exclude eyes, brows and lips from crack repair. Disable it for damage crossing those details; it cannot identify every face in a damaged scan.");
                    ui.collapsing("Face model license and attribution",|ui| { egui::ScrollArea::vertical().max_height(160.).show(ui,|ui| {ui.add(egui::Label::new(palette_core::processing::SKIN_NOTICE).wrap());ui.add(egui::Label::new(palette_core::processing::SKIN_LICENSE).wrap());});});
                    ui.small("Strength 0–100% mixes the result; 100–200% increases crack sensitivity and search reach without extrapolating colours. Repair radius accepts 1–64 source pixels (up to 96 effective at boosted Strength). Multiscale cracks links curved/interrupted damage, then interpolates from clean boundaries. Bright cracks avoids dark facial detail; choose Dark scratches or Both when needed. Show detected damage previews the mask. Inspect it: real lines/highlights can resemble damage. Legacy projects retain Thin scratches until you change Method or exceed their old range. This does not reconstruct missing faces or large torn regions.");
                }
                if d.operation.kind()==Kind::Harmonization {
                    ui.set_max_width(380.);
                    ui.label("Choose a reference layer using the parameter selector.");
                    ui.small("Live Lab colour/tone matching follows reference edits, masks and effects. Missing, fully masked or zero-opacity references leave the layer unchanged. Reference visibility is ignored; its own evaluated colours are sampled, without background blending or outer effect padding. This is statistical matching, not learned relighting.");
                }
                if let Operation::Filter(Filter::MakeupTransfer{reference,..})=&d.operation {
                    ui.set_max_width(380.);
                    ui.add(egui::Label::new(format!("Reference: {} ({} × {})",reference.name,reference.width,reference.height)).wrap());
                    ui.label("Local face landmarks · spatial makeup detail");
                    ui.label("Use Reference photo to copy colors from the largest detected face. Clear, front-facing portraits work best.");
                    ui.small("Photo references retain gradients, liner and highlights. Lips, Eyes and Cheeks control each region. Luminance must be enabled to transfer dark liner and highlights. Strength 100% uses the sampled reference colour inside the mask.");
                    ui.small("Protect occlusions can mirror the clearer eye when the other is covered by dark hair; disable for asymmetric makeup. This is a geometric warp, not semantic makeup isolation. Lighting/skin texture may transfer; masks may need refinement. The original Rose and plum preset supplies colour only.");
                    ui.collapsing("Model license and attribution",|ui| {
                        egui::ScrollArea::vertical().max_height(180.).show(ui,|ui| {
                            ui.add(egui::Label::new(palette_core::processing::SKIN_NOTICE).wrap());
                            ui.add(egui::Label::new(palette_core::processing::SKIN_LICENSE).wrap());
                        });
                    });
                }
                if let Operation::Filter(Filter::ColorTransfer {reference,..})=&d.operation {
                    ui.set_max_width(380.);
                    ui.add(egui::Label::new(format!("Reference profile: {} ({} × {})",reference.name,reference.width,reference.height)).wrap());
                    ui.label("Global Lab statistics · conventional processing · runs locally");
                    ui.label("Strength controls the transfer; Luminance transfers brightness; Contrast matches channel variation; Saturation adjusts transferred color.");
                    ui.small("The built-in warm palette is an original default. Load an image to use its colors. Alpha and source pixels stay intact; all reference statistics are saved in the project, so the reference file is not needed later.");
                    ui.small("Global matching has no semantic regions or learned appearance model. Luminance is retained in Lab before gamut clipping; extreme colors can still change displayed brightness.");
                }
                if d.operation.kind() == Kind::DepthBlur {
                    ui.set_max_width(380.);
                    ui.label("Learned relative depth · runs locally");
                    ui.label("Click or drag on the image to focus. Disable Click to focus to set Focal depth manually.");
                    ui.label("Focal depth: 0 = far, 100% = near. Focal range sets the sharp depth band. Blur radius uses image pixels.");
                    ui.small("Show depth map previews the inferred depth. This is relative depth with a Gaussian blur, not measured distance or an occlusion-aware lens simulation. Fine edges and unusual scenes can be misestimated.");
                    ui.small(palette_core::processing::DEPTH_ID);
                    ui.collapsing("Model license and attribution", |ui| {
                        egui::ScrollArea::vertical().max_height(200.).show(ui,|ui| {
                            ui.add(egui::Label::new(palette_core::processing::DEPTH_NOTICE).wrap());
                        });
                    });
                }
                if d.operation.kind() == Kind::SuperZoom {
                    ui.set_max_width(420.);
                    egui::ScrollArea::vertical()
                        .max_height((ui.ctx().content_rect().height() - 180.).max(100.))
                        .show(ui, |ui| {
                    ui.label("Local learned magnification · cropped output");
                    ui.label("Keeps canvas dimensions; this does not enlarge the document");
                    ui.label("Origin selects the source point placed at the canvas center");
                    ui.label("Enhancement adjusts learned luminance; color and alpha are resampled");
                    ui.small("Fixed 3× network; 1–3× magnification. Face-specific restoration and denoising controls remain unavailable.");
                    ui.separator();
                    ui.label("Whole-image export uses Zoom and Enhancement above; origin is ignored.");
                    ui.small("Exports all visible layers as one PNG, replacing this crop with full-image enlargement. Other committed processing stays active. The canvas preview shows the crop, not this export. Save .palette separately to preserve editing.");
                    if let Some(Value::Number(scale)) = d.operation.value("zoom") {
                        match palette_core::processing::super_zoom_dimensions(d.original.width, d.original.height, scale) {
                            Ok([w,h]) => {
                                ui.label(format!("Output: {w} × {h} pixels"));
                                if ui.add_enabled(!app.busy, egui::Button::new("Export enlarged whole image as PNG…")).clicked() {
                                    *intent = Some(Intent::ExportZoom);
                                    ui.close();
                                }
                            }
                            Err(error) => { ui.label(error); }
                        }
                    }
                    if app.zoom_export.is_some() && ui.button("Cancel Super Zoom export").clicked() {
                        *intent = Some(Intent::CancelZoomExport);
                        ui.close();
                    }
                    ui.small(palette_core::processing::ZOOM_ID);
                    ui.collapsing("Model license and attribution", |ui| {
                        egui::ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                            ui.add(egui::Label::new(palette_core::processing::ZOOM_NOTICE).wrap());
                            ui.add(egui::Label::new(palette_core::processing::SKIN_LICENSE).wrap());
                            ui.add(egui::Label::new(palette_core::processing::MOSAIC_LICENSE).wrap());
                        });
                    });
                    });
                }
                if d.operation.kind() == Kind::SkinSmoothing {
                    ui.set_max_width(420.);
                    ui.label("Face-guided Skin Smoothing · runs locally");
                    ui.label("Learned face geometry; conventional edge-preserving smoothing");
                    ui.label("Protects eyes, lips and brows; up to eight detected faces");
                    ui.label("Blur controls face-relative radius; Smoothness controls edge tolerance");
                    ui.small("No detected face: source unchanged. Facial hair and occlusions may need a filter mask.");
                    ui.small(palette_core::processing::SKIN_ID);
                    ui.collapsing("Model license and attribution", |ui| {
                        ui.set_max_width(380.);
                        egui::ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                            ui.add(egui::Label::new(palette_core::processing::SKIN_NOTICE).wrap());
                            ui.add(egui::Label::new(palette_core::processing::SKIN_LICENSE).wrap());
                        });
                    });
                }
                if d.operation.kind() == Kind::Colorize {
                    ui.label("ECCV16 · bundled model · runs locally");
                    ui.label("256 px color inference; source detail and alpha retained");
                    ui.label("Predicted colors may differ from the original scene");
                    ui.small(palette_core::processing::COLORIZE_ID);
                    ui.collapsing("Model license", |ui| {
                        ui.set_max_width(380.);
                        egui::ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                            ui.add(egui::Label::new(palette_core::processing::COLORIZE_LICENSE).wrap());
                        });
                    });
                }
                if d.operation.kind() == Kind::JpegArtifactsRemoval {
                    ui.label("DnCNN-3 · bundled model · runs locally");
                    ui.label("Full-resolution luminance restoration; alpha retained");
                    ui.label("Color compression artifacts are not reconstructed");
                    ui.small(palette_core::processing::JPEG_ID);
                    ui.collapsing("Model license", |ui| {
                        ui.set_max_width(380.);
                        egui::ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                            ui.add(egui::Label::new(palette_core::processing::JPEG_LICENSE).wrap());
                        });
                    });
                }
                if d.operation.kind() == Kind::StyleTransfer {
                    ui.label("Mosaic · bundled model · runs locally");
                    ui.label("224 px working image; source size and alpha preserved");
                    ui.small(palette_core::processing::MOSAIC_ID);
                    ui.collapsing("Model license", |ui| {
                        ui.set_max_width(380.);
                        egui::ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                            ui.add(egui::Label::new(palette_core::processing::MOSAIC_LICENSE).wrap());
                        });
                    });
                }
                match &d.operation {
                    Operation::Filter(Filter::ColorLookup {lut:Some(l),..})=>{ui.label(format!("{} ({}³)",l.name,l.size));},
                    Operation::Filter(Filter::Displace {map:Some(m),map_name,..})=>{ui.label(format!("{} ({} × {})",map_name,m.width(),m.height()));},
                    _=>{},
                }
                if matches!(d.operation.kind(),Kind::ColorLookup|Kind::Displace|Kind::ColorTransfer|Kind::MakeupTransfer|Kind::LandscapeMixer) && ui.add_enabled(d.import.is_none(),egui::Button::new(match d.operation.kind(){Kind::ColorLookup=>"Load 3D LUT…",Kind::ColorTransfer|Kind::MakeupTransfer|Kind::LandscapeMixer=>"Load reference image…",_=>"Load displacement map…"})).clicked() {*intent=Some(Intent::Import);ui.close();}
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
            app.capture_popup("processing-super-zoom-options",&menu.response);
            app.capture_popup("processing-color-transfer-reference",&menu.response);
            app.capture_popup("processing-makeup-reference",&menu.response);
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
    // Method is a compatibility tag, not a user-facing Landscape setting.
    // Keep legacy saved filters readable without exposing an obsolete switch.
    if d.operation.kind() == Kind::LandscapeMixer && parameters[d.parameter].key == "method" {
        d.parameter = 0;
    }
    let selector = egui::ComboBox::from_id_salt("processing-parameter")
        .selected_text(
            if compact
                && matches!(d.operation.kind(), Kind::SuperZoom | Kind::DepthBlur)
                && parameters[d.parameter].key == "model"
            {
                "Model"
            } else if compact && parameters[d.parameter].key == "protect_occlusions" {
                "Occlusions"
            } else if compact && d.operation.kind() == Kind::LandscapeMixer {
                match parameters[d.parameter].key {
                    "sky" => "Sky",
                    "terrain" => "Terrain",
                    "detail" => "Detail",
                    "horizon" => "Horizon",
                    "reference_horizon" => "Ref. horizon",
                    "feather" => "Softness",
                    "edge_sensitivity" => "Sky detect",
                    _ => parameters[d.parameter].label,
                }
            } else if compact && d.operation.kind() == Kind::PhotoRestoration {
                match parameters[d.parameter].key {
                    "enhancement" => "Enhancement",
                    "scratches" => "Scratches",
                    "radius" => "Radius",
                    "noise" => "Noise",
                    "color_noise" => "Color noise",
                    "show_damage" => "Damage",
                    "damage_type" => "Damage type",
                    "protect_faces" => "Faces",
                    _ => parameters[d.parameter].label,
                }
            } else if compact && parameters[d.parameter].key == "reference_layer" {
                "Reference"
            } else {
                parameters[d.parameter].label
            },
        )
        .width(if compact { 102. } else { 124. })
        .height(if d.operation.kind() == Kind::LandscapeMixer {
            520.
        } else if d.operation.kind() == Kind::PhotoRestoration {
            360.
        } else if d.operation.kind() == Kind::MakeupTransfer {
            300.
        } else {
            200.
        })
        .show_ui(ui, |ui| {
            for (index, p) in parameters.iter().enumerate() {
                if d.operation.kind() == Kind::LandscapeMixer && p.key == "method" {
                    continue;
                }
                ui.selectable_value(&mut d.parameter, index, p.label);
            }
        });
    selector.response.on_hover_text("Choose a setting to edit");
    let p = parameters[d.parameter];
    if matches!(p.kind, ParameterType::Layer) {
        let Operation::Filter(Filter::Harmonization {
            reference_layer, ..
        }) = &mut d.operation
        else {
            return false;
        };
        let name = reference_layer
            .and_then(|id| d.original.layers.iter().find(|l| l.id == id))
            .map_or("Choose layer…", |l| l.name.as_str());
        let full = name.to_owned();
        let short = if name.chars().count() > if compact { 12 } else { 24 } {
            format!(
                "{}…",
                name.chars()
                    .take(if compact { 11 } else { 23 })
                    .collect::<String>()
            )
        } else {
            name.to_owned()
        };
        let mut changed = false;
        egui::ComboBox::from_id_salt("processing-reference-layer")
            .width(if compact { 112. } else { 180. })
            .selected_text(short)
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(reference_layer, None, "Choose layer…")
                    .changed();
                for layer in &d.original.layers {
                    if layer.id == d.layer {
                        continue;
                    }
                    let available = palette_core::processing::harmonization::can_reference(
                        &d.original,
                        d.layer,
                        layer.id,
                    );
                    ui.add_enabled_ui(available, |ui| {
                        changed |= ui
                            .selectable_value(reference_layer, Some(layer.id), &layer.name)
                            .changed();
                    });
                }
            })
            .response
            .on_hover_text(full);
        return changed;
    }
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
            Operation::Effect(Effect::GradientOverlay { style: palette_core::document::GradientStyle::Angle, .. }) if p.key=="scale" => false,
            Operation::Effect(Effect::BevelEmboss {texture:false,..}) if matches!(p.key,"surface"|"texture_scale"|"texture_depth")=>false,
            Operation::Filter(Filter::LandscapeMixer {
                reference: None, ..
            }) if matches!(p.key, "sky" | "terrain" | "detail" | "reference_horizon") => false,
            Operation::Filter(Filter::MakeupTransfer {
                model, reference, ..
            }) if matches!(p.key, "cheeks" | "protect_occlusions") => {
                *model == palette_core::document::MakeupModel::FaceMeshSpatialV2
                    && reference.texture.is_some()
            }
            Operation::Filter(Filter::DepthBlur { pick_focus, .. }) => {
                if p.key == "focus" {
                    !*pick_focus
                } else if matches!(p.key, "center_x" | "center_y") {
                    *pick_focus
                } else {
                    true
                }
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
        ui.add_enabled_ui(stochastic && channel_active && context_active, |ui| {
            match (
                p.kind,
                d.operation.value(p.key).expect("registered parameter"),
            ) {
                (ParameterType::Number { min, max, step }, Value::Number(mut value)) => {
                    let (mut min, mut max) = (min, max);
                    if let Operation::Filter(Filter::Levels { black, white, .. }) = d.operation {
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
                        Kind::Harmonization
                            | Kind::MakeupTransfer
                            | Kind::ColorTransfer
                            | Kind::LandscapeMixer
                            | Kind::DepthBlur
                            | Kind::SuperZoom
                            | Kind::SkinSmoothing
                            | Kind::Colorize
                            | Kind::JpegArtifactsRemoval
                            | Kind::StyleTransfer
                            | Kind::Chrome
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
                    let percent = ((d.operation.kind() == Kind::SmartSharpen && p.key == "amount")
                        || (d.operation.kind()==Kind::SmartPortrait && matches!(p.key,"happiness"|"surprise"|"anger"|"lighting"))
                        || (d.operation.kind() == Kind::PhotoRestoration
                            && matches!(
                                p.key,
                                "strength" | "enhancement" | "scratches" | "noise" | "color_noise"
                            ))
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
                            && matches!(d.operation, Operation::Filter(Filter::FilmGrain { .. }))))
                        && !(d.operation.kind()==Kind::BevelEmboss && p.key=="depth")
                        || (d.operation.kind()==Kind::BevelEmboss && matches!(p.key,"highlight_opacity"|"shadow_opacity"|"texture_depth"));
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
                    if percent || (d.operation.kind()==Kind::BevelEmboss && p.key=="depth") || (d.operation.kind()==Kind::GradientOverlay && p.key=="scale") {
                        slider=slider.suffix("%");
                    } else if matches!(d.operation.kind(),Kind::BevelEmboss|Kind::Satin|Kind::GradientOverlay|Kind::PatternOverlay) && matches!(p.key,"angle"|"altitude") {
                        slider=slider.suffix("°");
                    } else if d.operation.kind() == Kind::SuperZoom && p.key == "zoom" {
                        slider = slider.suffix("×");
                    } else if d.operation.kind()==Kind::SmartPortrait && p.key=="light_direction" {
                        slider=slider.suffix("°");
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
                        .selected_text(
                            if d.operation.kind() == Kind::LandscapeMixer && p.key == "method" {
                                if selected==1 {"Edge-aware"} else {"Horizontal"}
                            } else if d.operation.kind() == Kind::MakeupTransfer && p.key == "model" {
                                if matches!(d.operation.value("model"), Some(Value::Choice(1))) {
                                    "Spatial detail"
                                } else {
                                    "Feature colour"
                                }
                            } else if compact && d.operation.kind() == Kind::SuperZoom {
                                "ESPCN 3×"
                            } else if compact && d.operation.kind() == Kind::DepthBlur {
                                "MiDaS"
                            } else if compact && p.key == "to_polar" {
                                if selected == 0 {
                                    "To Polar"
                                } else {
                                    "To Rectangular"
                                }
                            } else {
                                options[selected]
                            },
                        )
                        .show_ui(ui, |ui| {
                            for (index, label) in options.iter().enumerate() {
                                let enabled=!(d.operation.kind()==Kind::LandscapeMixer && p.key=="method" && index==0 && matches!(&d.operation,Operation::Filter(Filter::LandscapeMixer{reference:Some(r),..}) if r.width()>512 || r.height()>512));
                                ui.add_enabled_ui(enabled,|ui|ui.selectable_value(&mut selected,index,*label));
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
                        && (matches!(d.operation,Operation::Effect(_)) || p.key == "preserve_luminosity"
                            || matches!(
                                d.operation.kind(),
                                Kind::DepthBlur | Kind::PhotoRestoration
                            ))
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
                    }
                    changed
                }
                _ => unreachable!("registered parameter type"),
            }
        })
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
