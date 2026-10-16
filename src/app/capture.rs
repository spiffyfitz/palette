//! Reproducible native UI capture scenes. Scene selection is a debug-build
//! command-line diagnostic and is never part of the editor's visible workflow.
use super::*;
#[derive(Default)]
pub(super) struct CaptureState {
    pub scene: Option<String>,
    pub settled_since: Option<std::time::Instant>,
}
impl PaletteApp {
    pub(super) fn prepare_capture(&mut self) {
        if self.screenshot.is_none() {
            return;
        }
        match self.capture.scene.as_deref() {
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
            Some("width") => self.tool = Tool::Rectangle,
            _ => {}
        }
    }
    pub(crate) fn capture_popup(&self, name: &str, response: &egui::Response) {
        if self.screenshot.is_some() && self.capture.scene.as_deref() == Some(name) {
            egui::Popup::open_id(&response.ctx, egui::Popup::default_response_id(response));
        }
    }
    pub(super) fn capture_is_settled(&mut self, ctx: &Context) -> bool {
        if self.screenshot.is_none() {
            return false;
        }
        if self.current().texture.is_none() || self.busy || self.edit_pending {
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
