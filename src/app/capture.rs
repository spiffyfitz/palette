//! Reproducible native UI capture scenes. Scene selection is a debug-build
//! command-line diagnostic and is never part of the editor's visible workflow.
use super::*;
#[derive(Default)]
pub(super) struct CaptureState {
    pub scene: Option<String>,
    pub settled_since: Option<std::time::Instant>,
}
impl PaletteApp {
    pub(crate) fn is_capturing(&self) -> bool {
        self.screenshot.is_some()
    }

    pub(super) fn prepare_capture(&mut self) {
        if self.screenshot.is_none() {
            return;
        }
        match self.capture.scene.as_deref() {
            Some(
                "processing-blur"
                | "processing-shadow"
                | "processing-glow"
                | "processing-stack"
                | "processing-effects-stack"
                | "processing-stroke"
                | "processing-inner-shadow"
                | "processing-inner-glow"
                | "processing-hue",
            ) => {
                use palette_core::processing::{Kind, Operation};
                self.current_mut().document =
                    Document::from_raster(Raster::new(819, 594, [0; 4]).expect("capture canvas"));
                self.current_mut().document.background =
                    if self.capture.scene.as_deref() == Some("processing-glow") {
                        [35, 35, 42, 255]
                    } else {
                        [255; 4]
                    };
                for y in 200..390 {
                    for x in 280..540 {
                        self.current_mut().document.active_mut().source.set(
                            x,
                            y,
                            [225, 70, 55, 255],
                        );
                    }
                }
                self.current_mut().zoom = 1.;
                self.interface.layer_properties = true;
                if matches!(
                    self.capture.scene.as_deref(),
                    Some("processing-stack" | "processing-effects-stack")
                ) {
                    let doc = &mut self.current_mut().document;
                    for kind in [Kind::GaussianBlur, Kind::DropShadow, Kind::OuterGlow] {
                        let id = doc.allocate_id();
                        match kind.default_operation() {
                            Operation::Filter(filter) => doc.active_mut().filters.push(
                                palette_core::document::FilterInstance {
                                    id,
                                    enabled: true,
                                    filter,
                                    mask: None,
                                },
                            ),
                            Operation::Effect(effect) => doc.active_mut().effects.push(
                                palette_core::document::EffectInstance {
                                    id,
                                    enabled: true,
                                    effect,
                                },
                            ),
                        }
                    }
                    self.open_processing_toolbar(
                        if self.capture.scene.as_deref() == Some("processing-stack") {
                            super::processing::Section::Filters
                        } else {
                            super::processing::Section::Effects
                        },
                    );
                } else {
                    let kind = match self.capture.scene.as_deref() {
                        Some("processing-shadow") => Kind::DropShadow,
                        Some("processing-glow") => Kind::OuterGlow,
                        Some("processing-stroke") => Kind::Stroke,
                        Some("processing-inner-shadow") => Kind::InnerShadow,
                        Some("processing-inner-glow") => Kind::InnerGlow,
                        Some("processing-hue") => Kind::HueSaturation,
                        _ => Kind::GaussianBlur,
                    };
                    self.open_processing(kind, None);
                    if let Some(d) = &mut self.processing_dialog {
                        if kind == Kind::Stroke {
                            d.operation
                                .set_value("width", palette_core::processing::Value::Number(8.))
                                .expect("capture width");
                            d.parameter = 3;
                        } else if kind == Kind::HueSaturation {
                            d.operation
                                .set_value("hue", palette_core::processing::Value::Number(120.))
                                .expect("capture hue");
                        }
                    }
                }
            }
            Some("layer-blending") => self.interface.layer_properties = true,
            Some("brush-selection" | "brush-selection-options") => {
                self.current_mut().document = Document::new(819, 594).expect("capture canvas");
                let raster = &mut self.current_mut().document.active_mut().source;
                for y in 110..470 {
                    for x in 140..680 {
                        raster.set(
                            x,
                            y,
                            if x < 510 {
                                [50, 105, 170, 255]
                            } else {
                                [225, 110, 60, 255]
                            },
                        );
                    }
                }
                self.current_mut().zoom = 1.;
                self.tool = Tool::SelectBrush;
                self.current_mut().selection_brush.diameter = 160.;
                self.current_mut().selection_brush.hardness = 0.8;
                self.current_mut().selection_feather = 4.;
                self.begin_brush_selection([290., 280.], palette_core::selection::Combine::Replace);
                self.current_mut()
                    .selection_brush_draft
                    .as_mut()
                    .expect("capture brush")
                    .points
                    .extend([[440., 260.], [540., 340.]]);
                if self.capture.scene.as_deref() == Some("brush-selection-options") {
                    self.current_mut().finish_brush_selection();
                }
            }
            Some("feather-outline") => {
                self.current_mut().document = Document::new(819, 594).expect("capture canvas");
                let doc = &mut self.current_mut().document;
                let raw = palette_core::editing::rectangle(
                    819,
                    594,
                    palette_core::editing::PixelRect {
                        x: 240,
                        y: 180,
                        width: 300,
                        height: 210,
                    },
                );
                palette_core::selection::apply_region(
                    doc,
                    &raw,
                    palette_core::selection::Combine::Replace,
                    20.,
                )
                .expect("capture feather");
                self.current_mut().zoom = 1.;
                self.current_mut().selection_feather = 20.;
                self.tool = Tool::Select;
            }
            Some("inverted-selection") => {
                let doc = &mut self.current_mut().document;
                doc.selection = Some(palette_core::editing::rectangle(
                    doc.width,
                    doc.height,
                    palette_core::editing::PixelRect {
                        x: 500,
                        y: 260,
                        width: 260,
                        height: 200,
                    },
                ));
                palette_core::selection::invert(doc);
                self.current_mut().zoom = 1.75;
                self.tool = Tool::Wand;
            }
            Some("canvas-scrollbars") => {
                self.current_mut().zoom = 2.;
                self.interface.layer_properties = true;
            }
            Some(
                "rectangle-toolbar" | "wand-toolbar" | "freeform-toolbar" | "freeform-preview",
            ) => {
                self.tool = match self.capture.scene.as_deref() {
                    Some("wand-toolbar") => Tool::Wand,
                    Some("freeform-toolbar" | "freeform-preview") => Tool::Lasso,
                    _ => Tool::Select,
                };
                self.current_mut().selection_feather = 4.;
                if self.capture.scene.as_deref() == Some("freeform-preview") {
                    self.current_mut().lasso_drag = Some(crate::session::LassoDrag {
                        points: vec![
                            [260., 240.],
                            [400., 180.],
                            [540., 280.],
                            [470., 390.],
                            [330., 370.],
                        ],
                        mode: palette_core::selection::Combine::Replace,
                    });
                }
            }
            Some("selection-options" | "wand-settings") => self.tool = Tool::Wand,
            Some("mask-layer") | Some("feather-selection") => {
                self.current_mut().document = Document::new(819, 594).expect("capture canvas");
                self.current_mut().command("Capture coverage", |doc| {
                    doc.add_layer();
                    for y in 120..470 {
                        for x in 180..680 {
                            doc.active_mut().source.set(
                                x,
                                y,
                                [40, (80 + (x - 180) / 3) as u8, 210, 255],
                            );
                        }
                    }
                    doc.selection = Some(palette_core::editing::rectangle(
                        doc.width,
                        doc.height,
                        palette_core::editing::PixelRect {
                            x: 280,
                            y: 180,
                            width: 300,
                            height: 220,
                        },
                    ));
                });
                self.tool = Tool::Select;
                self.current_mut().zoom = 1.;
                if self.capture.scene.as_deref() == Some("mask-layer") {
                    let doc = &mut self.current_mut().document;
                    doc.selection = Some(
                        palette_core::selection::feather(
                            doc.selection.as_ref().expect("capture selection"),
                            18.,
                        )
                        .expect("capture feather"),
                    );
                    palette_core::selection::mask_from_selection(doc).expect("capture mask");
                    doc.selection = None;
                    self.interface.layer_properties = true;
                } else {
                    self.show_feather(false);
                }
            }
            Some("close-dialog") => {
                self.current_mut()
                    .command("Capture unsaved drawing", |doc| {
                        doc.active_mut().source =
                            Raster::new(1280, 720, [138, 0, 23, 255]).expect("capture canvas");
                    });
                self.tool = Tool::Select;
                self.close_pending = Some(self.current().id);
            }
            Some("paint-project") | Some("paint-export") => {
                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("crates/palette-core/tests/fixtures/microsoft-paint.paint");
                self.current_mut().document = io::open(&path).expect("capture Paint specimen");
                self.current_mut().name = "Untitled".into();
                self.current_mut().zoom = 1.;
                self.current_mut().invalidate_preview();
                if self.capture.scene.as_deref() == Some("paint-export") {
                    self.paint_export = Some(super::files::PaintExport {
                        path: PathBuf::from("target/capture.paint"),
                        session: self.current().id,
                        revision: self.current().history.revision,
                        backup_complete: false,
                    });
                }
            }
            Some("avif-export") => {
                self.avif_export = Some(super::files::AvifExport {
                    path: PathBuf::from("target/capture.avif"),
                    options: io::AvifOptions::default(),
                })
            }
            Some("recovery") => {
                self.recovery_prompt = true;
                self.recoveries = vec![crate::persistence::Recovery {
                    name: "Recovered drawing".into(),
                    original: None,
                    project: "capture.palette".into(),
                    manifest: PathBuf::from("target/capture.json"),
                }];
            }
            Some("colors") => self.edit_color(crate::interface::ColorTarget::Foreground),
            Some("properties") => self.show_canvas_properties(),
            Some("text") => {
                self.tool = Tool::Text;
                self.start_text(palette_core::editing::PixelRect {
                    x: 100,
                    y: 100,
                    width: 520,
                    height: 150,
                });
                self.text_editor.as_mut().expect("capture draft").spec.text =
                    "Text is inline!".into();
            }
            Some("layers") => {
                self.current_mut().command("Capture layers", |doc| {
                    doc.add_layer();
                    doc.active_mut().name = "Red stroke".into();
                    let red = Brush {
                        diameter: 60.0,
                        opacity: 1.0,
                        hardness: 1.0,
                        color: [230, 45, 55, 220],
                        erase: false,
                    };
                    red.segment(&mut doc.active_mut().source, [150.0, 200.0], [600.0, 200.0]);
                    let middle = doc.active_layer;
                    doc.add_layer();
                    doc.active_mut().name = "Black stroke".into();
                    let black = Brush {
                        color: [20, 20, 25, 200],
                        ..red
                    };
                    black.segment(&mut doc.active_mut().source, [400.0, 100.0], [400.0, 450.0]);
                    doc.active_layer = middle;
                });
                self.interface.layer_properties = true;
            }
            Some("shapes") => {
                use palette_core::shapes::{self, Kind, Style};
                self.current_mut().command("Capture shapes", |doc| {
                    doc.add_layer();
                    for (i, kind) in [
                        Kind::RoundedRectangle,
                        Kind::Triangle,
                        Kind::Diamond,
                        Kind::Pentagon,
                        Kind::Hexagon,
                        Kind::Star,
                        Kind::RightArrow,
                        Kind::DownArrow,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let x = 180. + (i % 4) as f32 * 230.;
                        let y = 160. + (i / 4) as f32 * 230.;
                        shapes::paint(
                            doc,
                            kind,
                            [x, y],
                            [x + 150., y + 150.],
                            Style {
                                outline: Some([40, 50, 65, 255]),
                                fill: Some(if i % 2 == 0 {
                                    [35, 190, 210, 255]
                                } else {
                                    [250, 175, 45, 255]
                                }),
                                width: 5.,
                                opacity: 0.9,
                            },
                        )
                        .expect("capture shape");
                    }
                    doc.active_mut().offset = [-20, 10];
                });
                self.choose_tool(Tool::MoveLayer);
            }
            Some("selection") => {
                self.tool = Tool::Select;
                let doc = &mut self.current_mut().document;
                let mut mask = palette_core::editing::rectangle(
                    doc.width,
                    doc.height,
                    palette_core::editing::PixelRect {
                        x: 230,
                        y: 160,
                        width: 640,
                        height: 390,
                    },
                );
                let values = std::sync::Arc::make_mut(&mut mask.values);
                for y in 260..390 {
                    for x in 430..620 {
                        values[(y * doc.width + x) as usize] = 0;
                    }
                }
                doc.selection = Some(mask);
            }
            Some("catalog") => {
                use palette_core::shapes::{self, Kind, Style};
                self.current_mut().command("Capture catalog", |doc| {
                    doc.add_layer();
                    for (i, kind) in [
                        Kind::RightTriangle,
                        Kind::LeftArrow,
                        Kind::UpArrow,
                        Kind::FourPointStar,
                        Kind::SixPointStar,
                        Kind::RectangularCallout,
                        Kind::OvalCallout,
                        Kind::CloudCallout,
                        Kind::Heart,
                        Kind::Lightning,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let x = 200. + (i % 5) as f32 * 190.;
                        let y = 180. + (i / 5) as f32 * 230.;
                        shapes::paint(
                            doc,
                            kind,
                            [x, y],
                            [x + 130., y + 150.],
                            Style {
                                outline: Some([35, 45, 60, 255]),
                                fill: Some([35, 195, 210, 255]),
                                width: 4.,
                                opacity: 1.,
                            },
                        )
                        .expect("catalog shape");
                    }
                });
                self.tool = Tool::Select;
            }
            Some("rotate-custom")
            | Some("selected-shape")
            | Some("selection-resize-preview")
            | Some("selection-rotate-preview") => {
                self.current_mut()
                    .command("Capture selected triangle", |doc| {
                        doc.add_layer();
                        let path = palette_core::shapes::path(
                            palette_core::shapes::Kind::Triangle,
                            [300., 180.],
                            [800., 600.],
                        );
                        palette_core::shapes::paint_selected(
                            doc,
                            &path,
                            palette_core::shapes::Style {
                                outline: Some([20, 185, 65, 255]),
                                fill: None,
                                width: 4.,
                                opacity: 1.,
                            },
                        )
                        .expect("capture selected shape");
                    });
                self.tool = Tool::Select;
                if self.capture.scene.as_deref() == Some("rotate-custom") {
                    self.show_rotation();
                }
                if matches!(
                    self.capture.scene.as_deref(),
                    Some("selection-resize-preview" | "selection-rotate-preview")
                ) {
                    use palette_core::transform::SelectionTransform;
                    let resize = self.capture.scene.as_deref() == Some("selection-resize-preview");
                    self.current_mut().begin_selection_transform(
                        if resize {
                            crate::session::SelectionHandle::Resize([1, 1])
                        } else {
                            crate::session::SelectionHandle::Rotate
                        },
                        [800., 600.],
                    );
                    let bounds = self
                        .current_mut()
                        .selection_bounds()
                        .expect("capture selection");
                    self.current_mut()
                        .selection_transform
                        .as_mut()
                        .expect("capture gesture")
                        .desired = if resize {
                        SelectionTransform::Resize {
                            origin: [bounds.x as i32, bounds.y as i32],
                            size: [360, 240],
                        }
                    } else {
                        SelectionTransform::Rotate(25.)
                    };
                    self.current_mut().invalidate_preview();
                }
            }
            Some("polygon-preview") | Some("curve-preview") => {
                let polygon = self.capture.scene.as_deref() == Some("polygon-preview");
                self.tool = if polygon { Tool::Polygon } else { Tool::Curve };
                let tool = self.tool;
                self.current_mut().construction = Some(crate::session::Construction {
                    tool,
                    points: if polygon {
                        vec![[300., 250.], [500., 250.]]
                    } else {
                        vec![[250., 350.], [350., 150.], [600., 550.], [750., 350.]]
                    },
                    hover: [650., 450.],
                    dragging: false,
                    stage: if polygon { 1 } else { 2 },
                    style: palette_core::shapes::Style {
                        outline: Some([20, 110, 200, 255]),
                        fill: None,
                        width: 4.,
                        opacity: 1.,
                    },
                });
            }
            Some("width") => self.tool = Tool::Rectangle,
            _ => {}
        }
    }
    pub(crate) fn capture_popup(&self, name: &str, response: &egui::Response) {
        if self.screenshot.is_some() && self.capture.scene.as_deref() == Some(name) {
            egui::Popup::open_id(&response.ctx, egui::Popup::default_response_id(response));
        }
    }
    pub(crate) fn capture_submenu(&self, name: &str, ui: &Ui, response: &egui::Response) {
        if self.screenshot.is_some() && self.capture.scene.as_deref() == Some(name) {
            egui::containers::menu::MenuState::from_ui(ui, |state, _| {
                state.open_item = Some(egui::containers::menu::SubMenu::id_from_widget_id(
                    response.id,
                ));
            });
        }
    }
    pub(super) fn capture_is_settled(&mut self, ctx: &Context) -> bool {
        if self.screenshot.is_none() {
            return false;
        }
        if self.current().texture.is_none()
            || self.current().rendered_generation != Some(self.current().generation)
            || self.processing_worker.busy
            || self
                .processing_dialog
                .as_ref()
                .is_some_and(|d| d.ready != Some(d.ticket))
            || self.selection_brush_worker.busy
            || self
                .current()
                .selection_brush_draft
                .as_ref()
                .is_some_and(|d| d.frame.as_ref().is_none_or(|f| f.samples != d.points.len()))
            || self.feather_worker.busy
            || self
                .current()
                .feather_preview
                .as_ref()
                .is_some_and(|p| p.frame.as_ref().is_none_or(|f| f.radius != p.radius))
            || self.busy
            || self.edit_pending
            || self
                .current()
                .selection_transform
                .as_ref()
                .is_some_and(|g| g.applied != Some(g.desired))
        {
            self.capture.settled_since = None;
            return false;
        }
        let elapsed = self
            .capture
            .settled_since
            .get_or_insert_with(std::time::Instant::now)
            .elapsed();
        if elapsed < Duration::from_millis(350) {
            ctx.request_repaint_after(Duration::from_millis(16));
            false
        } else {
            true
        }
    }
}
