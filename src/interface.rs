//! Paint-style interface metrics and reusable interaction treatments.
//! Dimensions are logical points; egui applies monitor DPI exactly once.
use crate::icons::{self, Icon};
use eframe::egui::{self, Color32, Context, Response, Stroke, Ui, Vec2, vec2};

pub const DEFAULT_CANVAS: [u32; 2] = [1280, 720];

pub const HOVER_SECONDS: f32 = 0.10;
pub const POPUP_SECONDS: f32 = 0.16;
pub const PANEL_SECONDS: f32 = 0.18;
pub const MENU_ROW_HEIGHT: f32 = 32.0;
pub const LAYER_TRAY_WIDTH: f32 = 112.0;
pub const CONTEXT_CONTROL_HEIGHT: f32 = 28.0;

/// Shared popup geometry keeps text and selection controls on the same baseline.
pub fn context_toolbar_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::popup(style).inner_margin(8)
}

/// Transient presentation state never enters the serialized document.
pub struct InterfaceState {
    pub layer_properties: bool,
    pub grid: bool,
    pub popup_was_open: bool,
    pub status: bool,
    pub ribbon_collapsed: bool,
    pub color_editor: Option<ColorEditor>,
    pub custom_colors: Vec<Color32>,
    pub color_target: ColorTarget,
    pub canvas_properties: Option<CanvasProperties>,
}
impl Default for InterfaceState {
    fn default() -> Self {
        Self {
            layer_properties: false,
            grid: false,
            popup_was_open: false,
            status: true,
            ribbon_collapsed: false,
            color_editor: None,
            custom_colors: vec![],
            canvas_properties: None,
            color_target: ColorTarget::Foreground,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ColorTarget {
    #[default]
    Foreground,
    Background,
}
pub struct ColorEditor {
    pub target: ColorTarget,
    pub rgb: [u8; 3],
    pub hex: String,
    pub hsv: egui::ecolor::Hsva,
    pub field: Option<egui::TextureHandle>,
}
impl ColorEditor {
    pub fn new(target: ColorTarget, rgb: [u8; 3]) -> Self {
        Self {
            target,
            rgb,
            hsv: Color32::from_rgb(rgb[0], rgb[1], rgb[2]).into(),
            hex: format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]),
            field: None,
        }
    }
}

pub fn configure(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        ("Palette Sans", epaint_default_fonts::UBUNTU_LIGHT),
        ("Palette Mono", epaint_default_fonts::HACK_REGULAR),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), egui::FontData::from_static(bytes).into());
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), vec![name.to_owned()]);
    }
    // Use installed Windows typography for the Paint-style close dialog only.
    // No Microsoft font resources are bundled; other platforms use our fallback.
    for (name, filename) in [
        ("Dialog", "segoeui.ttf"),
        ("Dialog Semibold", "seguisb.ttf"),
    ] {
        let mut family = fonts.families[&egui::FontFamily::Proportional].clone();
        if cfg!(target_os = "windows")
            && let Some(root) = std::env::var_os("WINDIR")
            && let Ok(bytes) =
                std::fs::read(std::path::PathBuf::from(root).join("Fonts").join(filename))
        {
            fonts
                .font_data
                .insert(name.into(), egui::FontData::from_owned(bytes).into());
            family.insert(0, name.into());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.global_style()).clone();
    style.animation_time = POPUP_SECONDS;
    style.visuals.override_text_color = Some(Color32::from_gray(235));
    style.interaction.tooltip_delay = 0.65;
    style.spacing.item_spacing = vec2(6.0, 4.0);
    style.spacing.button_padding = vec2(10.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.visuals.window_corner_radius = egui::CornerRadius::same(8);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(8);
    style.visuals.window_fill = Color32::from_rgb(40, 40, 43);
    style.visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(62, 62, 66));
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 5],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(85),
    };
    style.visuals.window_shadow = style.visuals.popup_shadow;
    style.visuals.selection.bg_fill = Color32::from_rgb(55, 65, 69);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.open.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(54, 54, 60);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(54, 54, 60);
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(61, 61, 68);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
    ctx.set_global_style(style);
}

/// Submenu styling is inherited by nested menus, preserving hover switching,
/// arrows, keyboard navigation, outside-click dismissal and screen-edge flipping.
pub fn menu_style(style: &mut egui::Style) {
    egui::containers::menu::menu_style(style);
    style.spacing.interact_size.y = MENU_ROW_HEIGHT;
    style.spacing.item_spacing.y = 1.0;
    style.spacing.button_padding = vec2(10.0, 5.0);
}

pub fn menu_item(
    ui: &mut Ui,
    title: &str,
    shortcut: &str,
    icon: Option<Icon>,
    enabled: bool,
) -> Response {
    let text = if icon.is_some() {
        format!("      {title}")
    } else {
        title.to_owned()
    };
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .shortcut_text(shortcut)
            .min_size(vec2(230.0, MENU_ROW_HEIGHT)),
    );
    if let Some(icon) = icon {
        icons::draw(
            ui,
            egui::Rect::from_center_size(
                response.rect.left_center() + vec2(17.0, 0.0),
                Vec2::splat(16.0),
            ),
            icon,
            if enabled {
                Color32::from_gray(230)
            } else {
                Color32::from_gray(100)
            },
        );
    }
    response
}

pub fn chevron(ui: &mut Ui, size: Vec2, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4, Color32::from_gray(55));
    }
    let c = rect.center();
    ui.painter().line_segment(
        [c + vec2(-3.0, -1.5), c + vec2(0.0, 1.5)],
        Stroke::new(1.0, Color32::LIGHT_GRAY),
    );
    ui.painter().line_segment(
        [c + vec2(0.0, 1.5), c + vec2(3.0, -1.5)],
        Stroke::new(1.0, Color32::LIGHT_GRAY),
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), tooltip));
    response.on_hover_text(tooltip)
}

/// Decorative stroke samples are original geometry, not copied Paint resources.
pub fn stroke_sample(ui: &Ui, rect: egui::Rect, thickness: f32, color: Color32) {
    let points: Vec<_> = (0..32)
        .map(|i| {
            let t = i as f32 / 31.0;
            egui::pos2(
                rect.left() + t * rect.width(),
                rect.center().y + (t * std::f32::consts::TAU).sin() * 4.0,
            )
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(thickness, color)));
}

/// Canvas bounds dialog is transactional, like Paint's Image Properties.
pub struct CanvasProperties {
    pub session: u64,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
}

/// Paint's compact two-row basic palette, plus user colors in a third row.
pub const PALETTE: [[u8; 3]; 20] = [
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
];
