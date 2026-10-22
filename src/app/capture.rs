//! Reproducible native UI capture scenes. Scene selection is a debug-build
//! command-line diagnostic and is never part of the editor's visible workflow.
use super::*;
mod brush_scenes;
mod gpu_probe;
mod layer_scenes;
mod reorder_scenes;
mod text_scenes;

#[derive(Default)]
pub(super) struct CaptureState {
    pub scene: Option<String>,
    pub settled_since: Option<std::time::Instant>,
    gpu_probe: gpu_probe::Probe,
    reorder_probe: reorder_scenes::Probe,
    text_probe: u8,
}
impl PaletteApp {
    pub(crate) fn is_capturing(&self) -> bool {
        self.screenshot.is_some()
    }

    pub(super) fn prepare_capture(&mut self) {
        if self.screenshot.is_none() {
            return;
        }
        if let Some(scene) = self.capture.scene.clone()
            && (self.prepare_reorder_capture(&scene) || self.prepare_layer_capture(&scene))
        {
            return;
        }
        match self.capture.scene.as_deref() {
            Some("brush-styles") => self.prepare_brush_capture(),
            Some("brush-size-preview") => {
                self.choose_brush(palette_core::brush::Style::Oil);
                self.brush.diameter = 96.;
                self.brush_size_preview_until = Some(f64::INFINITY);
            }
            Some("tabs-overflow") => {
                self.current_mut().name = "Mountain study 01.png".into();
                for number in 2..=16 {
                    self.new_document();
                    self.current_mut().name = format!("Mountain study {number:02}.png");
                }
                self.active = 8;
                self.current_mut().command("Capture dirty tab", |doc| {
                    doc.active_mut().source.set(0, 0, [255, 0, 0, 255]);
                });
            }
            Some(
                "processing-blur"
                | "processing-shadow"
                | "processing-glow"
                | "processing-stack"
                | "processing-effects-stack"
                | "processing-stroke"
                | "processing-inner-shadow"
                | "processing-inner-glow"
                | "processing-hue"
                | "processing-levels"
                | "processing-curves"
                | "processing-motion"
                | "processing-noise"
                | "processing-mosaic"
                | "processing-high-pass"
                | "processing-edges"
                | "processing-emboss"
                | "processing-unsharp"
                | "processing-offset"
                | "processing-twirl"
                | "processing-wave"
                | "processing-surface"
                | "processing-radial"
                | "processing-clouds"
                | "processing-difference-clouds"
                | "processing-poster-edges"
                | "processing-grain"
                | "processing-fragment"
                | "processing-despeckle"
                | "processing-dust"
                | "processing-reduce-noise"
                | "processing-pinch"
                | "processing-spherize"
                | "processing-ripple"
                | "processing-sharpen"
                | "processing-sharpen-more"
                | "processing-polar"
                | "processing-zigzag"
                | "processing-fibers"
                | "processing-glowing-edges"
                | "processing-origin"
                | "processing-cutout"
                | "processing-film-grain"
                | "processing-accented-edges"
                | "processing-ink-outlines"
                | "processing-photocopy"
                | "processing-halftone"
                | "processing-patchwork"
                | "processing-craquelure"
                | "processing-color-halftone"
                | "processing-crystallize"
                | "processing-facet"
                | "processing-mezzotint"
                | "processing-pointillize"
                | "processing-stained-glass"
                | "processing-mosaic-tiles"
                | "processing-diffuse"
                | "processing-color-balance"
                | "processing-photo-filter"
                | "processing-channel-mixer"
                | "processing-gradient-map"
                | "processing-selective-color"
                | "processing-shadows-highlights"
                | "processing-fixed-blur"
                | "processing-blur-more"
                | "processing-colored-pencil"
                | "processing-dry-brush"
                | "processing-paint-daubs"
                | "processing-smudge-stick"
                | "processing-angled-strokes"
                | "processing-crosshatch"
                | "processing-dark-strokes"
                | "processing-spatter"
                | "processing-sprayed-strokes"
                | "processing-sumi-e"
                | "processing-bas-relief"
                | "processing-chalk-charcoal"
                | "processing-charcoal"
                | "processing-graphic-pen"
                | "processing-note-paper"
                | "processing-stamp"
                | "processing-chrome"
                | "processing-conte-crayon"
                | "processing-plaster"
                | "processing-reticulation"
                | "processing-torn-edges"
                | "processing-water-paper"
                | "processing-fresco"
                | "processing-neon-glow"
                | "processing-palette-knife"
                | "processing-plastic-wrap"
                | "processing-rough-pastels"
                | "processing-sponge"
                | "processing-underpainting"
                | "processing-watercolor"
                | "processing-diffuse-glow"
                | "processing-glass"
                | "processing-ocean-ripple"
                | "processing-texturizer"
                | "processing-shape-blur"
                | "processing-smart-blur"
                | "processing-sharpen-edges"
                | "processing-smart-sharpen"
                | "processing-extrude"
                | "processing-oil-paint"
                | "processing-tiles"
                | "processing-trace-contour"
                | "processing-wind"
                | "processing-lens-blur"
                | "processing-lens-flare"
                | "processing-lighting-effects"
                | "processing-color-lookup"
                | "processing-displace"
                | "processing-shear"
                | "processing-custom"
                | "processing-custom-kernel"
                | "processing-field-blur"
                | "processing-iris-blur"
                | "processing-tilt-shift"
                | "processing-path-blur"
                | "processing-spin-blur"
                | "processing-flame"
                | "processing-picture-frame"
                | "processing-tree"
                | "processing-ntsc"
                | "processing-deinterlace"
                | "processing-hsb-hsl"
                | "processing-shear-curve",
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
                let contrast_pattern = matches!(
                    self.capture.scene.as_deref(),
                    Some(
                        "processing-edges"
                            | "processing-emboss"
                            | "processing-unsharp"
                            | "processing-offset"
                            | "processing-twirl"
                            | "processing-wave"
                            | "processing-surface"
                            | "processing-radial"
                            | "processing-poster-edges"
                            | "processing-grain"
                            | "processing-fragment"
                            | "processing-despeckle"
                            | "processing-dust"
                            | "processing-reduce-noise"
                            | "processing-pinch"
                            | "processing-spherize"
                            | "processing-ripple"
                            | "processing-sharpen"
                            | "processing-sharpen-more"
                            | "processing-polar"
                            | "processing-zigzag"
                            | "processing-glowing-edges"
                            | "processing-origin"
                    )
                );
                let cleanup_pattern = matches!(
                    self.capture.scene.as_deref(),
                    Some("processing-despeckle" | "processing-dust" | "processing-reduce-noise")
                );
                let artistic_pattern = matches!(
                    self.capture.scene.as_deref(),
                    Some(
                        "processing-cutout"
                            | "processing-film-grain"
                            | "processing-accented-edges"
                            | "processing-ink-outlines"
                            | "processing-photocopy"
                            | "processing-halftone"
                            | "processing-patchwork"
                            | "processing-craquelure"
                            | "processing-color-halftone"
                            | "processing-crystallize"
                            | "processing-facet"
                            | "processing-mezzotint"
                            | "processing-pointillize"
                            | "processing-stained-glass"
                            | "processing-mosaic-tiles"
                            | "processing-diffuse"
                            | "processing-color-balance"
                            | "processing-photo-filter"
                            | "processing-channel-mixer"
                            | "processing-gradient-map"
                            | "processing-selective-color"
                            | "processing-shadows-highlights"
                            | "processing-fixed-blur"
                            | "processing-blur-more"
                            | "processing-colored-pencil"
                            | "processing-dry-brush"
                            | "processing-paint-daubs"
                            | "processing-smudge-stick"
                            | "processing-angled-strokes"
                            | "processing-crosshatch"
                            | "processing-dark-strokes"
                            | "processing-spatter"
                            | "processing-sprayed-strokes"
                            | "processing-sumi-e"
                            | "processing-bas-relief"
                            | "processing-chalk-charcoal"
                            | "processing-charcoal"
                            | "processing-graphic-pen"
                            | "processing-note-paper"
                            | "processing-stamp"
                            | "processing-chrome"
                            | "processing-conte-crayon"
                            | "processing-plaster"
                            | "processing-reticulation"
                            | "processing-torn-edges"
                            | "processing-water-paper"
                            | "processing-fresco"
                            | "processing-neon-glow"
                            | "processing-palette-knife"
                            | "processing-plastic-wrap"
                            | "processing-rough-pastels"
                            | "processing-sponge"
                            | "processing-underpainting"
                            | "processing-watercolor"
                            | "processing-diffuse-glow"
                            | "processing-glass"
                            | "processing-ocean-ripple"
                            | "processing-texturizer"
                            | "processing-shape-blur"
                            | "processing-smart-blur"
                            | "processing-sharpen-edges"
                            | "processing-smart-sharpen"
                            | "processing-extrude"
                            | "processing-oil-paint"
                            | "processing-tiles"
                            | "processing-trace-contour"
                            | "processing-wind"
                            | "processing-lens-blur"
                            | "processing-lens-flare"
                            | "processing-lighting-effects"
                            | "processing-color-lookup"
                            | "processing-displace"
                            | "processing-shear"
                            | "processing-custom"
                            | "processing-custom-kernel"
                            | "processing-field-blur"
                            | "processing-iris-blur"
                            | "processing-tilt-shift"
                            | "processing-path-blur"
                            | "processing-spin-blur"
                            | "processing-flame"
                            | "processing-picture-frame"
                            | "processing-tree"
                            | "processing-ntsc"
                            | "processing-deinterlace"
                            | "processing-hsb-hsl"
                            | "processing-shear-curve"
                    )
                );
                for y in 200..390 {
                    for x in 280..540 {
                        let mut color = if contrast_pattern && x > 400 && y > 240 && y < 350 {
                            [35u8, 140, 210, 255]
                        } else {
                            [225u8, 70, 55, 255]
                        };
                        if artistic_pattern {
                            color = [
                                (80 + (x - 280) * 145 / 260) as u8,
                                (40 + (y - 200) * 150 / 190) as u8,
                                55,
                                255,
                            ];
                            if x > 400 && y > 240 && y < 350 {
                                color = [35, 140, (80 + (y - 240) * 130 / 110) as u8, 255];
                            }
                            if x > 320 && x < 380 && y > 255 && y < 335 {
                                color = [225, 185, 55, 255];
                            }
                        }
                        if cleanup_pattern {
                            let delta = ((x * 17 + y * 31) % 25) as i16 - 12;
                            for c in &mut color[..3] {
                                *c = (i16::from(*c) + delta).clamp(0, 255) as u8;
                            }
                            if x % 31 == 0 && y % 29 == 0 {
                                color = [255; 4];
                            }
                        }
                        self.current_mut()
                            .document
                            .active_mut()
                            .source
                            .set(x, y, color);
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
                        Some("processing-levels") => Kind::Levels,
                        Some("processing-curves") => Kind::Curves,
                        Some("processing-motion") => Kind::MotionBlur,
                        Some("processing-noise") => Kind::AddNoise,
                        Some("processing-mosaic") => Kind::Mosaic,
                        Some("processing-high-pass") => Kind::HighPass,
                        Some("processing-edges") => Kind::FindEdges,
                        Some("processing-emboss") => Kind::Emboss,
                        Some("processing-unsharp") => Kind::UnsharpMask,
                        Some("processing-offset") => Kind::Offset,
                        Some("processing-twirl") => Kind::Twirl,
                        Some("processing-wave") => Kind::Wave,
                        Some("processing-surface") => Kind::SurfaceBlur,
                        Some("processing-radial") => Kind::RadialBlur,
                        Some("processing-clouds") => Kind::Clouds,
                        Some("processing-difference-clouds") => Kind::DifferenceClouds,
                        Some("processing-poster-edges") => Kind::PosterEdges,
                        Some("processing-grain") => Kind::Grain,
                        Some("processing-fragment") => Kind::Fragment,
                        Some("processing-despeckle") => Kind::Despeckle,
                        Some("processing-dust") => Kind::DustScratches,
                        Some("processing-reduce-noise") => Kind::ReduceNoise,
                        Some("processing-pinch") => Kind::Pinch,
                        Some("processing-spherize") => Kind::Spherize,
                        Some("processing-ripple") => Kind::Ripple,
                        Some("processing-sharpen") => Kind::Sharpen,
                        Some("processing-sharpen-more") => Kind::SharpenMore,
                        Some("processing-polar") => Kind::PolarCoordinates,
                        Some("processing-zigzag") => Kind::ZigZag,
                        Some("processing-fibers") => Kind::Fibers,
                        Some("processing-glowing-edges") => Kind::GlowingEdges,
                        Some("processing-origin") => Kind::Twirl,
                        Some("processing-cutout") => Kind::Cutout,
                        Some("processing-film-grain") => Kind::FilmGrain,
                        Some("processing-accented-edges") => Kind::AccentedEdges,
                        Some("processing-ink-outlines") => Kind::InkOutlines,
                        Some("processing-photocopy") => Kind::Photocopy,
                        Some("processing-halftone") => Kind::HalftonePattern,
                        Some("processing-patchwork") => Kind::Patchwork,
                        Some("processing-craquelure") => Kind::Craquelure,
                        Some("processing-color-halftone") => Kind::ColorHalftone,
                        Some("processing-crystallize") => Kind::Crystallize,
                        Some("processing-facet") => Kind::Facet,
                        Some("processing-mezzotint") => Kind::Mezzotint,
                        Some("processing-pointillize") => Kind::Pointillize,
                        Some("processing-stained-glass") => Kind::StainedGlass,
                        Some("processing-mosaic-tiles") => Kind::MosaicTiles,
                        Some("processing-diffuse") => Kind::Diffuse,
                        Some("processing-color-balance") => Kind::ColorBalance,
                        Some("processing-photo-filter") => Kind::PhotoFilter,
                        Some("processing-channel-mixer") => Kind::ChannelMixer,
                        Some("processing-gradient-map") => Kind::GradientMap,
                        Some("processing-selective-color") => Kind::SelectiveColor,
                        Some("processing-shadows-highlights") => Kind::ShadowsHighlights,
                        Some("processing-fixed-blur") => Kind::Blur,
                        Some("processing-blur-more") => Kind::BlurMore,
                        Some("processing-colored-pencil") => Kind::ColoredPencil,
                        Some("processing-dry-brush") => Kind::DryBrush,
                        Some("processing-paint-daubs") => Kind::PaintDaubs,
                        Some("processing-smudge-stick") => Kind::SmudgeStick,
                        Some("processing-angled-strokes") => Kind::AngledStrokes,
                        Some("processing-crosshatch") => Kind::Crosshatch,
                        Some("processing-dark-strokes") => Kind::DarkStrokes,
                        Some("processing-spatter") => Kind::Spatter,
                        Some("processing-sprayed-strokes") => Kind::SprayedStrokes,
                        Some("processing-sumi-e") => Kind::SumiE,
                        Some("processing-bas-relief") => Kind::BasRelief,
                        Some("processing-chalk-charcoal") => Kind::ChalkCharcoal,
                        Some("processing-charcoal") => Kind::Charcoal,
                        Some("processing-graphic-pen") => Kind::GraphicPen,
                        Some("processing-note-paper") => Kind::NotePaper,
                        Some("processing-stamp") => Kind::Stamp,
                        Some("processing-chrome") => Kind::Chrome,
                        Some("processing-conte-crayon") => Kind::ConteCrayon,
                        Some("processing-plaster") => Kind::Plaster,
                        Some("processing-reticulation") => Kind::Reticulation,
                        Some("processing-torn-edges") => Kind::TornEdges,
                        Some("processing-water-paper") => Kind::WaterPaper,
                        Some("processing-fresco") => Kind::Fresco,
                        Some("processing-neon-glow") => Kind::NeonGlow,
                        Some("processing-palette-knife") => Kind::PaletteKnife,
                        Some("processing-plastic-wrap") => Kind::PlasticWrap,
                        Some("processing-rough-pastels") => Kind::RoughPastels,
                        Some("processing-sponge") => Kind::Sponge,
                        Some("processing-underpainting") => Kind::Underpainting,
                        Some("processing-watercolor") => Kind::Watercolor,
                        Some("processing-diffuse-glow") => Kind::DiffuseGlow,
                        Some("processing-glass") => Kind::Glass,
                        Some("processing-ocean-ripple") => Kind::OceanRipple,
                        Some("processing-texturizer") => Kind::Texturizer,
                        Some("processing-shape-blur") => Kind::ShapeBlur,
                        Some("processing-smart-blur") => Kind::SmartBlur,
                        Some("processing-sharpen-edges") => Kind::SharpenEdges,
                        Some("processing-smart-sharpen") => Kind::SmartSharpen,
                        Some("processing-extrude") => Kind::Extrude,
                        Some("processing-oil-paint") => Kind::OilPaint,
                        Some("processing-tiles") => Kind::Tiles,
                        Some("processing-trace-contour") => Kind::TraceContour,
                        Some("processing-wind") => Kind::Wind,
                        Some("processing-lens-blur") => Kind::LensBlur,
                        Some("processing-lens-flare") => Kind::LensFlare,
                        Some("processing-lighting-effects") => Kind::LightingEffects,
                        Some("processing-color-lookup") => Kind::ColorLookup,
                        Some("processing-displace") => Kind::Displace,
                        Some("processing-shear") => Kind::Shear,
                        Some("processing-custom" | "processing-custom-kernel") => Kind::Custom,
                        Some("processing-shear-curve") => Kind::Shear,
                        Some("processing-field-blur") => Kind::FieldBlur,
                        Some("processing-iris-blur") => Kind::IrisBlur,
                        Some("processing-tilt-shift") => Kind::TiltShift,
                        Some("processing-path-blur") => Kind::PathBlur,
                        Some("processing-spin-blur") => Kind::SpinBlur,
                        Some("processing-flame") => Kind::Flame,
                        Some("processing-picture-frame") => Kind::PictureFrame,
                        Some("processing-tree") => Kind::Tree,
                        Some("processing-ntsc") => Kind::NtscColors,
                        Some("processing-deinterlace") => Kind::Deinterlace,
                        Some("processing-hsb-hsl") => Kind::HsbHsl,
                        _ => Kind::GaussianBlur,
                    };

                    self.open_processing(kind, None);
                    if let Some(d) = &mut self.processing_dialog {
                        let edit = match kind {
                            Kind::ColorBalance => Some(("midtone_red", 0.6)),
                            Kind::ChannelMixer => Some(("red_green", 0.6)),
                            Kind::SelectiveColor => Some(("yellows_magenta", 0.6)),
                            _ => None,
                        };
                        if let Some((key, number)) = edit {
                            d.operation
                                .set_value(key, palette_core::processing::Value::Number(number))
                                .expect("capture adjustment");
                            d.parameter = kind
                                .descriptor()
                                .parameters
                                .iter()
                                .position(|p| p.key == key)
                                .unwrap();
                        }

                        if self.capture.scene.as_deref() == Some("processing-origin") {
                            d.operation
                                .set_value(
                                    "center_x",
                                    palette_core::processing::Value::Number(0.38),
                                )
                                .expect("origin x");
                            d.operation
                                .set_value(
                                    "center_y",
                                    palette_core::processing::Value::Number(0.55),
                                )
                                .expect("origin y");
                            d.parameter = kind
                                .descriptor()
                                .parameters
                                .iter()
                                .position(|p| p.key == "center_x")
                                .expect("origin parameter");
                        }
                        if kind == Kind::Stroke {
                            d.operation
                                .set_value("width", palette_core::processing::Value::Number(8.))
                                .expect("capture width");
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
            Some("polygon-selection-preview" | "polygon-selection-result") => {
                self.tool = Tool::PolygonSelect;
                self.current_mut().zoom = 0.75;
                self.current_mut().selection_feather = 3.;
                let points = vec![[300., 220.], [560., 210.], [600., 380.], [380., 450.]];
                if self.capture.scene.as_deref() == Some("polygon-selection-preview") {
                    self.current_mut().polygon_selection = Some(crate::session::PolygonSelection {
                        points,
                        hover: [260., 360.],
                        mode: palette_core::selection::Combine::Replace,
                        click_armed: false,
                    });
                } else {
                    let doc = &mut self.current_mut().document;
                    let region = palette_core::selection::freeform(doc.width, doc.height, &points)
                        .expect("capture polygon");
                    palette_core::selection::apply_region(
                        doc,
                        &region,
                        palette_core::selection::Combine::Replace,
                        3.,
                    )
                    .expect("capture selection");
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
            Some("text" | "text-fonts" | "text-overflow" | "text-rotated") => {
                self.tool = Tool::Text;
                self.start_text(palette_core::editing::PixelRect {
                    x: 100,
                    y: 100,
                    width: 520,
                    height: 150,
                });
                self.text_editor.as_mut().expect("capture draft").spec.text =
                    "Text is inline!".into();
                if self.capture.scene.as_deref() != Some("text") {
                    let editor = self.text_editor.as_mut().unwrap();
                    editor.origin = [310, 220];
                    editor.spec.width = 260;
                    editor.spec.height = 125;
                    editor.spec.text =
                        "First line\nSecond line\nThird line\nFourth line\nFifth line".into();
                    editor.angle = if self.capture.scene.as_deref() == Some("text-rotated") {
                        25.
                    } else {
                        0.
                    };
                }
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
                self.choose_tool(Tool::Move);
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
            Some(
                "floating-shape-original"
                | "floating-shape-moving"
                | "floating-shape-moved"
                | "floating-shape-rotated"
                | "floating-shape-committed",
            ) => {
                let scene = self.capture.scene.as_deref().unwrap().to_owned();
                let mut pixels = Raster::new(640, 480, [0; 4]).expect("capture dimensions");
                for y in 0..480 {
                    for x in 0..640 {
                        let color = if x % 80 < 2 || y % 80 < 2 {
                            [45, 60, 80, 255]
                        } else {
                            [
                                (40 + x * 180 / 640) as u8,
                                (45 + y * 170 / 480) as u8,
                                120,
                                255,
                            ]
                        };
                        pixels.set(x, y, color);
                    }
                }
                let mut doc = Document::from_raster(pixels);
                palette_core::floating::create(
                    &mut doc,
                    &palette_core::shapes::path(
                        palette_core::shapes::Kind::Star,
                        [160., 100.],
                        [360., 300.],
                    ),
                    palette_core::shapes::Style {
                        outline: Some([0, 0, 0, 255]),
                        fill: None,
                        width: 8.,
                        opacity: 0.8,
                    },
                )
                .expect("capture isolated shape");
                if matches!(
                    scene.as_str(),
                    "floating-shape-moved" | "floating-shape-rotated" | "floating-shape-committed"
                ) {
                    palette_core::transform::SelectionTransform::Move([100, 50])
                        .apply(&mut doc)
                        .expect("move shape");
                }
                if matches!(
                    scene.as_str(),
                    "floating-shape-rotated" | "floating-shape-committed"
                ) {
                    palette_core::transform::rotate(&mut doc, 32.).expect("rotate shape");
                }
                if scene == "floating-shape-committed" {
                    palette_core::floating::commit(&mut doc);
                }
                self.current_mut().document = doc;
                self.current_mut().zoom = 1.25;
                self.current_mut().invalidate();
                self.tool = Tool::Select;
                if scene == "floating-shape-moving" {
                    self.current_mut()
                        .begin_selection_transform(crate::session::SelectionHandle::Move, [0., 0.]);
                    self.current_mut()
                        .selection_transform
                        .as_mut()
                        .unwrap()
                        .desired = palette_core::transform::SelectionTransform::Move([100, 50]);
                }
            }
            Some("rotate-custom")
            | Some("selected-shape")
            | Some("selection-resize-preview")
            | Some("selection-rotate-preview") => {
                let floating = self.capture.scene.as_deref() == Some("selected-shape");
                self.current_mut()
                    .command("Capture selected triangle", |doc| {
                        doc.add_layer();
                        let path = palette_core::shapes::path(
                            palette_core::shapes::Kind::Triangle,
                            [300., 180.],
                            [800., 600.],
                        );
                        let paint = if floating {
                            palette_core::floating::create
                        } else {
                            palette_core::shapes::paint_selected
                        };
                        paint(
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
        if self.capture.scene.as_deref() == Some("text-fonts") && self.capture.text_probe < 4 {
            return false;
        }
        if matches!(
            self.capture.scene.as_deref(),
            Some("gpu-layer-move" | "gpu-selection-move" | "gpu-floating-shape-move")
        ) && self.capture.gpu_probe.step < 6
        {
            return false;
        }
        if self
            .capture
            .scene
            .as_deref()
            .is_some_and(|s| matches!(s, "reorder-filters" | "reorder-effects" | "reorder-layers"))
            && self.capture.reorder_probe.step < 4
        {
            return false;
        }
        if self.current().texture.is_none()
            || !self.display_is_current()
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
                .text_editor
                .as_ref()
                .is_some_and(|editor| !editor.ready())
            || self
                .current()
                .selection_transform
                .as_ref()
                .is_some_and(|g| {
                    g.applied != Some(g.desired) && self.gpu_selection_move().is_none()
                })
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
