use super::*;
mod brushes;
mod floating_shapes;
mod gpu;
mod layer_movement;
mod move_tool;
mod polygon_selection;
mod ribbon;
mod tabs;
mod text_tool;
use egui::{Event, Modifiers, PointerButton, RawInput, Rect, pos2};

fn test_app(ctx: &Context) -> PaletteApp {
    let mut app = PaletteApp::with_context(ctx);
    // Keep every canvas edge reachable in these gesture tests. Production new
    // documents still start at 100%; a separate test verifies that contract.
    app.current_mut().zoom = 0.75;
    app
}

fn frame(app: &mut PaletteApp, ctx: &Context, events: Vec<Event>) -> Vec<egui::TextureId> {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386.0, 963.0))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    let partials = output
        .textures_delta
        .set
        .iter()
        .filter(|(_, deltas)| deltas.iter().any(|delta| delta.pos.is_some()))
        .map(|(id, _)| *id)
        .collect();
    output.textures_delta.clear();
    partials
}

fn pointer(pos: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn painting_through_ui_at_75_percent_is_undoable_and_layer_local() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().zoom = 0.75;
    app.current_mut().command("Add layer", Document::add_layer);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let origin = app.canvas_rect.center() - vec2(1280.0, 720.0) * 0.75 * 0.5;
    let start = origin + vec2(10.5, 10.5) * 0.75;
    let end = origin + vec2(110.5, 10.5) * 0.75;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    assert!(app.current().document.active().source.get(50, 10)[3] > 200);
    assert_eq!(app.current().document.layers[0].source.get(50, 10), [0; 4]);
    app.current_mut().undo();
    assert_eq!(app.current().document.active().source.get(50, 10), [0; 4]);
    app.current_mut().redo();
    assert!(app.current().document.active().source.get(50, 10)[3] > 200);
}

#[test]
fn tab_state_and_unsaved_close_are_independent() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().command("Paint", |doc| {
        doc.active_mut().source.set(0, 0, [255, 0, 0, 255])
    });
    let first = app.current().id;
    app.current_mut().zoom = 2.0;
    app.new_document();
    app.new_document();
    assert_eq!(app.sessions.len(), 3);
    assert!(!app.current().modified());
    // Close the inactive, modified tab through its hover-only button. This must
    // prompt for that document without first activating it or losing its state.
    frame(&mut app, &ctx, vec![]);
    let first_tab = ctx
        .read_response(egui::Id::unique(("document-tab", first)))
        .expect("first document tab")
        .rect;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(first_tab.center())],
    );
    let close_button = ctx
        .read_response(egui::Id::unique(("tab-close", first)))
        .expect("hover close control")
        .rect
        .center();
    frame(&mut app, &ctx, vec![Event::PointerMoved(close_button)]);
    frame(&mut app, &ctx, vec![pointer(close_button, true)]);
    frame(&mut app, &ctx, vec![pointer(close_button, false)]);
    assert_eq!(app.close_pending, Some(first));
    assert_eq!(app.sessions.len(), 3);
    app.close_pending = None;
    let third = app.current().id;
    app.request_close(third, &ctx);
    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.sessions[0].zoom, 2.0);
    assert_eq!(
        app.sessions[0].document.active().source.get(0, 0),
        [255, 0, 0, 255]
    );
    app.sessions[0].undo();
    assert!(!app.sessions[0].modified());
    assert_eq!(app.sessions[1].document.active().source.get(0, 0), [0; 4]);
}

#[test]
fn canvas_resize_preserves_source_and_is_reversible() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut()
        .document
        .active_mut()
        .source
        .set(0, 0, [25, 50, 75, 100]);
    // Exercise the real resize command through its handle's image-space bounds.
    frame(&mut app, &ctx, vec![]);
    let handle = image_position(&app, [1280.0, 720.0]) + vec2(3.0, 3.0);
    let delta = vec2(40.0, 20.0) * app.current().zoom;
    frame(&mut app, &ctx, vec![Event::PointerMoved(handle)]);
    frame(&mut app, &ctx, vec![pointer(handle, true)]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(handle + delta)]);
    frame(&mut app, &ctx, vec![pointer(handle + delta, false)]);
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (1320, 740)
    );
    assert_eq!(
        app.current().document.active().source.get(0, 0),
        [25, 50, 75, 100]
    );
    app.current_mut().undo();
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (1280, 720)
    );
}

#[test]
fn ribbon_and_layers_remain_within_a_small_window() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(780.0, 520.0))),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();
    assert!(!output.shapes.is_empty());
    assert!(app.canvas_rect.is_positive());
    assert!(app.canvas_rect.bottom() <= 520.0);
}

fn image_position(app: &PaletteApp, point: [f32; 2]) -> egui::Pos2 {
    let size = vec2(
        app.current().document.width as f32,
        app.current().document.height as f32,
    ) * app.current().zoom;
    let origin = app.canvas_rect.center() - size * 0.5 + app.current().pan;
    pos2(origin.x.round(), origin.y.round()) + vec2(point[0], point[1]) * app.current().zoom
}

fn wait_for_render(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.current().rendered_generation != Some(app.current().generation) {
        frame(app, ctx, vec![]);
        assert!(std::time::Instant::now() < deadline, "render timed out");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn held_brush_and_eraser_update_texture_before_release_and_undo_once() {
    for tool in [Tool::Brush, Tool::Eraser] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().zoom = 0.75;
        app.current_mut().command("Add layer", |doc| {
            doc.add_layer();
            if tool == Tool::Eraser {
                doc.active_mut().source = Raster::new(1280, 720, [90, 30, 180, 200]).unwrap();
            }
            doc.active_mut().opacity = 0.7;
            doc.active_mut().blend = BlendMode::Multiply;
        });
        app.add_filter(Filter::Invert);
        let before = app.current().document.clone();
        app.tool = tool;
        app.brush.opacity = 0.4;
        app.brush.hardness = 0.2;
        frame(&mut app, &ctx, vec![]);
        wait_for_render(&mut app, &ctx);
        let texture = app.current().texture.as_ref().unwrap().id();
        let a = image_position(&app, [20.5, 20.5]);
        let b = image_position(&app, [100.5, 20.5]);
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(a), pointer(a, true)],
        );
        let output = frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
        assert!(app.current().paint_stroke.is_some(), "mouse is still held");
        assert!(output.contains(&texture));
        assert_eq!(
            app.current().rendered.as_ref().unwrap(),
            &composite::render(&app.current().document)
        );
        assert_ne!(
            app.current().document.active().source,
            before.active().source
        );
        frame(&mut app, &ctx, vec![pointer(b, false)]);
        app.undo_current();
        assert_eq!(app.current().document, before);
        app.redo_current();
        assert_ne!(
            app.current().document.active().source,
            before.active().source
        );
    }
}

#[test]
fn paint_accepts_progress_then_patches_missing_samples_but_rejects_old_epochs() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    let (tx, rx) = mpsc::channel();
    app.render_rx = rx;
    let snapshot = app.current().document.clone();
    let epoch = app.current().render_epoch;
    let generation = app.current().generation;
    app.current_mut()
        .document
        .active_mut()
        .source
        .set(20, 20, [200, 0, 0, 255]);
    app.current_mut()
        .invalidate_paint(Some(palette_core::editing::PixelRect {
            x: 20,
            y: 20,
            width: 1,
            height: 1,
        }));
    let send = |epoch, generation| {
        let pixels = composite::render(&snapshot);
        tx.send(RenderResult {
            gpu: None,
            thumbnails: vec![],
            selection: None,
            id: app.sessions[0].id,
            epoch,
            generation,
            image: FrameImage::Full(texture_image(&pixels, 16384)),
            pixels,
        })
        .unwrap();
    };
    send(epoch, generation);
    app.poll_workers(&ctx);
    assert_eq!(app.current().rendered_generation, Some(generation));
    app.refresh_live_paint();
    assert_eq!(
        app.current().rendered.as_ref().unwrap().get(20, 20),
        [200, 0, 0, 255]
    );
    app.current_mut()
        .command("Unrelated edit", |doc| doc.active_mut().opacity = 0.3);
    // Even a result stamped with a newer generation cannot cross an edit epoch.
    let pixels = composite::render(&snapshot);
    tx.send(RenderResult {
        gpu: None,
        thumbnails: vec![],
        selection: None,
        id: app.current().id,
        epoch,
        generation: app.current().generation,
        image: FrameImage::Full(texture_image(&pixels, 16384)),
        pixels,
    })
    .unwrap();
    app.poll_workers(&ctx);
    assert_eq!(
        app.current().rendered.as_ref().unwrap().get(20, 20),
        [200, 0, 0, 255]
    );
}

fn wait_for_text(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !app.text_editor.as_ref().unwrap().ready() {
        frame(app, ctx, vec![]);
        assert!(
            std::time::Instant::now() < deadline,
            "text preview timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn text_box_at_fractional_zoom_previews_cancels_and_commits_one_layer_command() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().zoom = 0.75;
    app.tool = Tool::Text;
    frame(&mut app, &ctx, vec![]);
    let before = app.current().document.clone();
    let a = image_position(&app, [40.0, 50.0]);
    let b = image_position(&app, [340.0, 160.0]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    assert_eq!(app.text_editor.as_ref().unwrap().origin, [40, 50]);
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![Event::Text("Palette\nLive text".into())],
    );
    assert_eq!(
        app.text_editor.as_ref().unwrap().spec.text,
        "Palette\nLive text"
    );
    wait_for_text(&mut app, &ctx);
    assert_eq!(app.current().document, before);
    assert!(!app.current().modified());
    // Changing a parameter immediately invalidates Apply until its own preview.
    app.text_editor.as_mut().unwrap().spec.size = 40.0;
    app.apply_text();
    assert_eq!(app.current().document, before);
    wait_for_text(&mut app, &ctx);
    app.apply_text();
    assert!(app.text_editor.is_none());
    assert_eq!(app.current().document.layers.len(), 2);
    assert_eq!(app.current().document.active().offset, [40, 50]);
    assert_eq!(app.current().document.layers[0], before.layers[0]);
    let after = app.current().document.clone();
    app.undo_current();
    assert_eq!(app.current().document, before);
    app.redo_current();
    assert_eq!(app.current().document, after);
    app.start_text(palette_core::editing::PixelRect {
        x: 10,
        y: 10,
        width: 200,
        height: 100,
    });
    app.text_editor.as_mut().unwrap().spec.text = "Cancel me".into();
    wait_for_text(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(app.text_editor.is_none());
    assert_eq!(app.current().document, after);
}
fn wait_for_edit(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.edit_pending
        || app
            .current()
            .selection_brush_draft
            .as_ref()
            .is_some_and(|d| d.released)
        || app
            .current()
            .feather_preview
            .as_ref()
            .is_some_and(|p| p.commit_requested)
        || app
            .current()
            .selection_transform
            .as_ref()
            .is_some_and(|g| g.released)
    {
        frame(app, ctx, vec![]);
        assert!(std::time::Instant::now() < deadline, "edit timed out");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    frame(app, ctx, vec![]);
    assert!(app.error.is_none(), "{:?}", app.error);
}

#[test]
fn selection_fill_move_crop_workflow_through_fractional_zoom_pointer_input() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().zoom = 0.75;
    app.current_mut().command("Add layer", Document::add_layer);
    app.tool = Tool::Select;
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [10.0, 10.0]);
    let b = image_position(&app, [30.0, 25.0]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    assert_eq!(
        app.current_mut().selection_bounds(),
        Some(palette_core::editing::PixelRect {
            x: 10,
            y: 10,
            width: 20,
            height: 15
        })
    );
    app.tool = Tool::Fill;
    app.brush.color = [255, 0, 0, 255];
    let seed = image_position(&app, [20.5, 20.5]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(seed), pointer(seed, true)],
    );
    frame(&mut app, &ctx, vec![pointer(seed, false)]);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.active().source.get(20, 20),
        [255, 0, 0, 255]
    );
    assert_eq!(app.current().document.active().source.get(9, 20), [0; 4]);
    assert_eq!(app.current().document.layers[0].source.get(20, 20), [0; 4]);
    app.tool = Tool::Select;
    let end = image_position(&app, [60.5, 40.5]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(seed), pointer(seed, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.active().source.get(60, 40),
        [255, 0, 0, 255]
    );
    assert_eq!(app.current().document.active().source.get(20, 20)[3], 0);
    app.edit(super::editing::Edit::Crop, &ctx);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (20, 15)
    );
    app.current_mut().undo();
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (1280, 720)
    );
    app.current_mut().undo();
    assert_eq!(
        app.current().document.active().source.get(20, 20),
        [255, 0, 0, 255]
    );
}

#[test]
fn native_paste_image_event_creates_an_independent_undoable_layer() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    frame(&mut app, &ctx, vec![]);
    let image = egui::ColorImage::from_rgba_unmultiplied([2, 1], &[255, 0, 0, 255, 0, 0, 255, 128]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PasteImage(std::sync::Arc::new(image))],
    );
    assert_eq!(app.current().document.layers.len(), 2);
    assert_eq!(
        app.current().document.active().source.get(1, 0),
        [0, 0, 255, 128]
    );
    assert_eq!(app.tool, Tool::Select);
    app.current_mut().undo();
    assert_eq!(app.current().document.layers.len(), 1);
    app.current_mut().redo();
    assert_eq!(app.current().document.layers.len(), 2);
    app.new_document();
    assert_eq!(app.current().document.layers.len(), 1);
}

#[test]
fn a_late_worker_result_cannot_overwrite_new_edits() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.edit(
        super::editing::Edit::Fill {
            point: [0, 0],
            color: [255, 0, 0, 255],
            opacity: 1.0,
            tolerance: 0,
        },
        &ctx,
    );
    app.current_mut().command("Newer edit", |doc| {
        doc.active_mut().source.set(0, 0, [0, 0, 255, 255])
    });
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.active().source.get(0, 0),
        [0, 0, 255, 255]
    );
    assert_eq!(app.current().document.active().source.get(1, 0), [0; 4]);
    assert!(app.message.contains("cancelled"));
}

#[test]
fn selected_shape_and_brush_do_not_change_unselected_pixels() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().command("Add layer", Document::add_layer);
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        1280,
        720,
        palette_core::editing::PixelRect {
            x: 20,
            y: 20,
            width: 20,
            height: 20,
        },
    ));
    app.tool = Tool::Rectangle;
    app.shape_fill = true;
    app.shape_outline = false;
    app.background = Color32::from_rgb(0, 255, 0);
    app.brush.color = [0, 255, 0, 255];
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [10.0, 10.0]);
    let b = image_position(&app, [50.0, 50.0]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.active().source.get(25, 25),
        [0, 255, 0, 255]
    );
    assert_eq!(app.current().document.active().source.get(15, 15), [0; 4]);
    app.tool = Tool::Brush;
    app.brush.color = [255, 0, 0, 255];
    app.brush.diameter = 12.0;
    let a = image_position(&app, [10.5, 30.5]);
    let b = image_position(&app, [50.5, 30.5]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.active().source.get(25, 30),
        [255, 0, 0, 255]
    );
    assert_eq!(app.current().document.active().source.get(15, 30), [0; 4]);
}

fn click_ui(app: &mut PaletteApp, ctx: &Context, point: egui::Pos2) {
    frame(app, ctx, vec![Event::PointerMoved(point)]);
    frame(app, ctx, vec![pointer(point, true)]);
    frame(app, ctx, vec![pointer(point, false)]);
}
fn escape_event() -> Event {
    Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
#[test]
fn paint_flyout_opens_at_its_control_and_escape_dismisses_it() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    frame(&mut app, &ctx, vec![]);
    click_ui(&mut app, &ctx, pos2(20.0, 16.0));
    frame(&mut app, &ctx, vec![]);
    assert!(egui::Popup::is_any_open(&ctx));
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(!egui::Popup::is_any_open(&ctx));
    assert!(!app.current().modified());
}
#[test]
fn inline_text_resizes_at_fractional_zoom_and_commits_when_clicked_outside() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.tool = Tool::Text;
    app.current_mut().zoom = 0.75;
    frame(&mut app, &ctx, vec![]);
    app.start_text(palette_core::editing::PixelRect {
        x: 50,
        y: 50,
        width: 300,
        height: 100,
    });
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![Event::Text("Inline text".into())]);
    wait_for_text(&mut app, &ctx);
    let handle = image_position(&app, [350.0, 150.0]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(handle)]);
    frame(&mut app, &ctx, vec![pointer(handle, true)]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(handle + vec2(30.0, 15.0))],
    );
    frame(
        &mut app,
        &ctx,
        vec![pointer(handle + vec2(30.0, 15.0), false)],
    );
    assert_eq!(app.text_editor.as_ref().unwrap().spec.width, 340);
    assert_eq!(app.text_editor.as_ref().unwrap().spec.height, 120);
    wait_for_text(&mut app, &ctx);
    let outside = image_position(&app, [700.0, 400.0]);
    click_ui(&mut app, &ctx, outside);
    assert!(app.text_editor.is_none());
    assert_eq!(app.current().document.layers.len(), 2);
    assert_eq!(app.current().document.active().offset, [50, 50]);
    app.undo_current();
    assert_eq!(app.current().document.layers.len(), 1);
}
#[test]
fn pending_text_preview_preserves_a_requested_tool_switch() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.tool = Tool::Text;
    frame(&mut app, &ctx, vec![]);
    app.start_text(palette_core::editing::PixelRect {
        x: 10,
        y: 10,
        width: 200,
        height: 100,
    });
    app.text_editor.as_mut().unwrap().spec.text = "Queued text".into();
    app.choose_tool(Tool::Brush);
    assert!(app.after_text.is_some());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.text_editor.is_some() || app.after_text.is_some() {
        frame(&mut app, &ctx, vec![]);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(app.tool, Tool::Brush);
    assert_eq!(app.current().document.layers.len(), 2);
}
#[test]
fn color_and_canvas_dialog_cancel_keep_state_and_canvas_ok_is_undoable() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    frame(&mut app, &ctx, vec![]);
    let before = app.current().document.clone();
    let color = app.brush.color;
    app.edit_color(crate::interface::ColorTarget::Foreground);
    app.interface.color_editor.as_mut().unwrap().rgb = [20, 40, 60];
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.interface.color_editor.is_none());
    assert_eq!(app.brush.color, color);
    app.show_canvas_properties();
    app.interface.canvas_properties.as_mut().unwrap().width = 400;
    frame(&mut app, &ctx, vec![]);
    // Release Escape before testing a new modal transaction.
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.interface.canvas_properties.is_none());
    assert_eq!(app.current().document, before);
    app.show_canvas_properties();
    app.interface.canvas_properties.as_mut().unwrap().width = 400;
    app.interface.canvas_properties.as_mut().unwrap().height = 300;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    click_ui(&mut app, &ctx, pos2(570.0, 744.0));
    assert!(app.interface.canvas_properties.is_none());
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (400, 300)
    );
    app.undo_current();
    assert_eq!(app.current().document, before);
}

#[test]
fn new_documents_and_properties_default_use_hd_canvas_at_actual_size() {
    let ctx = Context::default();
    let mut app = PaletteApp::with_context(&ctx);
    assert_eq!(
        [app.current().document.width, app.current().document.height],
        [1280, 720]
    );
    assert_eq!(app.current().zoom, 1.0);
    app.current_mut().command("Resize", |doc| {
        palette_core::transform::canvas_resize(doc, 400, 300).unwrap();
    });
    app.new_document();
    assert_eq!(
        [app.current().document.width, app.current().document.height],
        [1280, 720]
    );
    app.show_canvas_properties();
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let properties = app.interface.canvas_properties.as_ref().unwrap();
    assert_eq!([properties.width, properties.height], [1280, 720]);
}
#[test]
fn whole_layer_drag_is_live_cancelable_and_one_undo_with_nudges_and_lock() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().command("Add", Document::add_layer);
    app.current_mut()
        .document
        .active_mut()
        .source
        .set(200, 200, [255, 0, 0, 255]);
    let source = app.current().document.active().source.clone();
    app.choose_tool(Tool::Move);
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [200., 200.]);
    let b = a + vec2(30., 15.);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    assert_eq!(app.current().document.active().offset, [40, 20]);
    assert!(app.current().layer_drag.is_some());
    let epoch = app.current().render_epoch;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(b + vec2(0.75, 0.))],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    assert_eq!(app.current().render_epoch, epoch);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.current().rendered_generation != Some(app.current().generation) {
        frame(&mut app, &ctx, vec![]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(std::time::Instant::now() < deadline);
    }
    assert_eq!(
        app.current().rendered.as_ref().unwrap().get(240, 220),
        [255, 0, 0, 255]
    );
    assert_eq!(app.current().document.active().source, source);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    assert!(app.current().layer_drag.is_none());
    app.undo_current();
    assert_eq!(app.current().document.active().offset, [0, 0]);
    assert_eq!(app.current().document.layers.len(), 2);
    app.redo_current();
    assert_eq!(app.current().document.active().offset, [40, 20]);
    let revision = app.current().history.revision;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b + vec2(15., 0.))]);
    frame(&mut app, &ctx, vec![escape_event()]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    assert_eq!(app.current().document.active().offset, [40, 20]);
    assert_eq!(app.current().history.revision, revision);
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(app.current().document.active().offset, [41, 20]);
    app.current_mut().document.active_mut().locked = true;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(b), pointer(b, false)],
    );
    assert_eq!(app.current().document.active().offset, [41, 20]);
    assert_eq!(app.current().document.active().source, source);
}
#[test]
fn new_shape_preview_cancels_and_commits_on_its_own_layer_at_fractional_zoom() {
    use palette_core::shapes::Kind;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().command("Add", Document::add_layer);
    app.choose_tool(Tool::Shape(Kind::Star));
    app.shape_fill = true;
    app.shape_outline = false;
    app.background = Color32::from_rgb(240, 170, 20);
    app.brush.opacity = 0.5;
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [300., 200.]);
    let b = image_position(&app, [500., 400.]);
    let before = app.current().document.clone();
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    assert_eq!(app.current().document, before);
    frame(&mut app, &ctx, vec![escape_event(), pointer(b, false)]);
    assert_eq!(app.current().document, before);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.active().source.get(400, 300),
        [240, 170, 20, 128]
    );
    assert_eq!(app.current().document.layers[0], before.layers[0]);
    app.undo_current();
    assert_eq!(app.current().document, before);
    app.redo_current();
    assert_eq!(app.current().document.active().source.get(400, 300)[3], 128);
}

#[test]
fn close_during_a_layer_move_records_it_before_the_unsaved_prompt() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.choose_tool(Tool::Move);
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [300., 300.]);
    let b = a + vec2(30., 15.);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    let id = app.current().id;
    app.request_close(id, &ctx);
    assert!(app.current().layer_drag.is_none());
    assert_eq!(app.close_pending, Some(id));
    assert_ne!(app.current().history.revision, 0);
    app.close_pending = None;
    app.undo_current();
    assert_eq!(app.current().document.active().offset, [0, 0]);
    assert!(!app.current().modified());
}

#[test]
fn gallery_matches_the_reference_rows_without_duplicate_placeholders() {
    let names = [
        "Line",
        "Curve",
        "Ellipse",
        "Rectangle",
        "Rounded rectangle",
        "Polygon",
        "Triangle",
        "Right triangle",
        "Diamond",
        "Pentagon",
        "Hexagon",
        "Right arrow",
        "Left arrow",
        "Up arrow",
        "Down arrow",
        "Four-point star",
        "Five-point star",
        "Six-point star",
        "Rectangular callout",
        "Oval callout",
        "Cloud callout",
        "Heart",
        "Lightning",
    ];
    assert_eq!(crate::ribbon::SHAPE_GALLERY.len(), 23);
    for (i, (_, tool)) in crate::ribbon::SHAPE_GALLERY.iter().enumerate() {
        assert_eq!(tool.unwrap().label(), names[i]);
    }
}
#[test]
fn actual_shape_preview_does_not_emit_vertices_far_beyond_a_thin_drag() {
    use palette_core::shapes::Kind;
    for kind in [
        Kind::Ellipse,
        Kind::Triangle,
        Kind::RoundedRectangle,
        Kind::Star,
        Kind::Heart,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.choose_tool(Tool::Shape(kind));
        app.brush.color = [237, 33, 123, 255];
        app.brush.diameter = 20.;
        frame(&mut app, &ctx, vec![]);
        let a = image_position(&app, [400., 200.]);
        let b = image_position(&app, [400.1, 400.]);
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(a), pointer(a, true)],
        );
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            events: vec![Event::PointerMoved(b)],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
        let primitives = ctx.tessellate(output.shapes, 1.);
        let ink = Color32::from_rgb(237, 33, 123);
        let bounds = Rect::from_two_pos(a, b).expand(25.);
        let mut found = false;
        for primitive in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                for vertex in mesh.vertices {
                    if vertex.color == ink && vertex.pos.y > app.canvas_rect.top() {
                        found = true;
                        assert!(
                            vertex.pos.is_finite() && bounds.contains(vertex.pos),
                            "{kind:?}: {:?}",
                            vertex.pos
                        );
                    }
                }
            }
        }
        assert!(found, "visible preview {kind:?}");
        assert!(!app.current().modified());
    }
}

#[test]
fn selection_ants_advance_with_ui_time_without_changing_the_document_or_history() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.tool = Tool::Select;
    let doc = &mut app.current_mut().document;
    doc.selection = Some(palette_core::editing::rectangle(
        doc.width,
        doc.height,
        palette_core::editing::PixelRect {
            x: 300,
            y: 200,
            width: 300,
            height: 200,
        },
    ));
    let before = doc.clone();
    let revision = app.current().history.revision;
    let mut render = |time| {
        let input = RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
        output
            .shapes
            .into_iter()
            .filter_map(|s| match s.shape {
                egui::Shape::LineSegment { points, stroke }
                    if stroke.color == Color32::BLACK && stroke.width == 1. =>
                {
                    Some(points)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let a = render(1.);
    let b = render(1.2);
    assert!(!a.is_empty());
    assert_ne!(a, b);
    assert_eq!(app.current().document, before);
    assert_eq!(app.current().history.revision, revision);
}

#[test]
fn ui_transforms_scope_to_selection_and_keep_undo_and_custom_cancel() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().command("Fixture", |doc| {
        doc.add_layer();
        doc.active_mut().source.set(300, 250, [255, 0, 0, 255]);
        doc.active_mut().source.set(301, 250, [0, 255, 0, 128]);
        doc.active_mut().source.set(100, 100, [0, 0, 255, 255]);
        doc.selection = Some(palette_core::editing::rectangle(
            doc.width,
            doc.height,
            palette_core::editing::PixelRect {
                x: 300,
                y: 250,
                width: 2,
                height: 1,
            },
        ));
    });
    let original = app.current().document.clone();
    app.edit(
        editing::Edit::Transform(palette_core::transform::Orthogonal::FlipHorizontal),
        &ctx,
    );
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        [app.current().document.width, app.current().document.height],
        [1280, 720]
    );
    assert_eq!(
        app.current().document.active().source.get(300, 250),
        [0, 255, 0, 128]
    );
    assert_eq!(
        app.current().document.active().source.get(100, 100),
        [0, 0, 255, 255]
    );
    assert!(app.current().document.layers[0] == original.layers[0]);
    app.undo_current();
    assert!(app.current().document == original);
    app.show_rotation();
    frame(&mut app, &ctx, vec![]);
    app.rotation_dialog.as_mut().unwrap().degrees = "37.5".into();
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.rotation_dialog.is_none());
    assert!(app.current().document == original);
    app.show_rotation();
    frame(&mut app, &ctx, vec![]);
    app.rotation_dialog.as_mut().unwrap().degrees = "37.5".into();
    frame(&mut app, &ctx, vec![enter_event()]);
    assert!(app.rotation_dialog.is_none());
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.current().document.width, 1280);
    app.undo_current();
    assert!(app.current().document == original);
}
#[test]
fn finished_shape_is_selected_and_can_be_flipped_without_rotating_the_canvas() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.choose_tool(Tool::Shape(palette_core::shapes::Kind::RightTriangle));
    app.shape_fill = true;
    app.shape_outline = false;
    app.background = Color32::from_rgb(40, 180, 210);
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [300., 250.]);
    let b = image_position(&app, [500., 450.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(b), pointer(b, false)],
    );
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.tool, Tool::Select);
    assert!(app.current().document.selection.is_some());
    let width = app.current().document.width;
    app.edit(
        editing::Edit::Transform(palette_core::transform::Orthogonal::Clockwise),
        &ctx,
    );
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.current().document.width, width);
    app.undo_current();
    app.undo_current();
    assert!(app.current().document.selection.is_none());
    assert!(!app.current().modified());
}
fn enter_event() -> Event {
    Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
#[test]
fn polygon_has_a_rubber_band_between_clicks_then_commits_selected_pixels_once() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.choose_tool(Tool::Polygon);
    app.brush.color = [237, 33, 123, 255];
    app.brush.diameter = 4.;
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [300., 250.]);
    let b = image_position(&app, [500., 250.]);
    let c = image_position(&app, [400., 450.]);
    click_ui(&mut app, &ctx, a);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
        events: vec![Event::PointerMoved(b)],
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();
    let ink = Color32::from_rgb(237, 33, 123);
    assert!(output.shapes.iter().any(|s| match &s.shape {
        egui::Shape::Path(path) =>
            matches!(path.stroke.color,egui::epaint::ColorMode::Solid(color) if color==ink)
                && path.points.last() == Some(&b),
        egui::Shape::LineSegment { points, stroke } => stroke.color == ink && points[1] == b,
        _ => false,
    }));
    assert!(!app.current().modified());
    assert!(app.current().construction.is_some());
    click_ui(&mut app, &ctx, b);
    click_ui(&mut app, &ctx, c);
    frame(&mut app, &ctx, vec![enter_event()]);
    wait_for_edit(&mut app, &ctx);
    assert!(app.current().construction.is_none());
    assert_eq!(app.tool, Tool::Select);
    assert!(app.current().document.selection.is_some());
    app.undo_current();
    assert!(!app.current().modified());
    assert!(app.current().document.selection.is_none());
}
#[test]
fn curve_bends_preview_without_source_changes_and_commits_or_cancels() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.choose_tool(Tool::Curve);
    frame(&mut app, &ctx, vec![]);
    let a = image_position(&app, [300., 300.]);
    let b = image_position(&app, [600., 300.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    assert_eq!(app.current().construction.as_ref().unwrap().stage, 1);
    assert!(!app.current().modified());
    let c = image_position(&app, [350., 200.]);
    click_ui(&mut app, &ctx, c);
    assert_eq!(app.current().construction.as_ref().unwrap().stage, 2);
    let d = image_position(&app, [550., 400.]);
    click_ui(&mut app, &ctx, d);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.tool, Tool::Select);
    assert!(app.current().document.selection.is_some());
    app.undo_current();
    assert!(!app.current().modified());
    app.choose_tool(Tool::Polygon);
    click_ui(&mut app, &ctx, a);
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.current().construction.is_none());
    assert!(!app.current().modified());
}

fn floating_selection_app(ctx: &Context) -> PaletteApp {
    let mut app = test_app(ctx);
    let mut doc = Document::new(320, 240).unwrap();
    doc.add_layer();
    for y in 60..90 {
        for x in 70..110 {
            doc.active_mut().source.set(x, y, [30, 180, 80, 180]);
        }
    }
    doc.selection = Some(palette_core::editing::rectangle(
        320,
        240,
        palette_core::editing::PixelRect {
            x: 70,
            y: 60,
            width: 40,
            height: 30,
        },
    ));
    let id = doc.allocate_id();
    doc.active_mut().filters.push(FilterInstance {
        id,
        enabled: true,
        filter: Filter::Invert,
        mask: None,
    });
    app.current_mut().document = doc;
    app.current_mut().invalidate();
    app.tool = Tool::Select;
    frame(&mut app, ctx, vec![]);
    wait_for_render(&mut app, ctx);
    app
}

#[test]
fn selected_pixels_move_live_with_processing_and_commit_one_undo() {
    let ctx = Context::default();
    let mut app = floating_selection_app(&ctx);
    let before = app.current().document.clone();
    let revision = app.current().history.revision;
    let start = image_position(&app, [90., 75.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    for point in [[150., 95.], [170., 110.], [150., 95.]] {
        let end = image_position(&app, point);
        frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
        wait_for_render(&mut app, &ctx);
        assert!(
            app.current().selection_transform.is_some(),
            "pointer remains held"
        );
        assert_eq!(app.current().history.revision, revision);
        assert_eq!(
            app.current()
                .document
                .active()
                .source
                .get(point[0] as u32, point[1] as u32),
            [30, 180, 80, 180]
        );
        assert_eq!(app.current().document.active().source.get(90, 75)[3], 0);
        assert_eq!(
            app.current().rendered.as_ref().unwrap(),
            &composite::render(&app.current().document)
        );
    }
    let end = image_position(&app, [150., 95.]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_edit(&mut app, &ctx);
    let after = app.current().document.clone();
    assert!(after != before);
    assert_eq!(after.layers[0], before.layers[0]);
    app.undo_current();
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, revision);
    app.redo_current();
    assert!(app.current().document == after);
}

#[test]
fn resize_handles_map_fractional_zoom_and_high_dpi_then_cancel_stale_previews() {
    for dpi in [1., 1.5, 2.] {
        let ctx = Context::default();
        ctx.set_pixels_per_point(dpi);
        let mut app = floating_selection_app(&ctx);
        app.current_mut().zoom = 1.25;
        frame(&mut app, &ctx, vec![]);
        let before = app.current().document.clone();
        let revision = app.current().history.revision;
        let start = app.image_rect.min + vec2(110., 90.) * 1.25;
        let end = start + vec2(40., 30.) * 1.25;
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(start), pointer(start, true)],
        );
        frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
        wait_for_render(&mut app, &ctx);
        let gesture = app.current().selection_transform.as_ref().unwrap();
        assert_eq!(
            gesture.desired,
            palette_core::transform::SelectionTransform::Resize {
                origin: [70, 60],
                size: [80, 60]
            }
        );
        assert!(app.current().document != before);
        assert_eq!(app.current().history.revision, revision);
        let stale_document = app.current().document.clone();
        let stale_epoch = app.current().render_epoch;
        let stale_operation = gesture.desired;
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(end + vec2(10., 10.))],
        );
        frame(&mut app, &ctx, vec![escape_event()]);
        assert!(app.current().selection_transform.is_none());
        assert!(app.current().document == before);
        // Deliver a late, apparently newer preview deterministically. Its epoch
        // must keep both the document and display untouched after cancellation.
        let (tx, rx) = mpsc::channel();
        let actual_receiver = std::mem::replace(&mut app.render_rx, rx);
        let pixels = composite::render(&stale_document);
        tx.send(RenderResult {
            gpu: None,
            thumbnails: vec![],
            id: app.current().id,
            generation: app.current().generation + 100,
            epoch: stale_epoch,
            image: FrameImage::Full(texture_image(&pixels, 16384)),
            pixels,
            selection: Some((stale_operation, Ok(stale_document))),
        })
        .unwrap();
        app.poll_workers(&ctx);
        app.render_rx = actual_receiver;
        assert!(app.current().document == before);
        frame(&mut app, &ctx, vec![pointer(end, false)]);
        wait_for_render(&mut app, &ctx);
        assert!(app.current().document == before);
        assert_eq!(app.current().history.revision, revision);
    }
}

#[test]
fn final_resize_target_is_not_lost_when_tab_changes_before_worker_finishes() {
    let ctx = Context::default();
    let mut app = floating_selection_app(&ctx);
    let before = app.current().document.clone();
    let start = image_position(&app, [110., 90.]);
    let end = image_position(&app, [150., 120.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    app.new_document();
    assert_eq!(app.sessions.len(), 2);
    assert!(app.sessions[0].selection_transform.is_none());
    assert_eq!(app.sessions[0].selection_bounds().unwrap().width, 80);
    assert_eq!(app.sessions[0].selection_bounds().unwrap().height, 60);
    wait_for_render(&mut app, &ctx);
    assert!(!app.current().modified());
    app.sessions[0].undo();
    assert!(app.sessions[0].document == before);
}

#[test]
fn rotation_handle_previews_pixels_and_shift_snaps_then_undoes_once() {
    let ctx = Context::default();
    let mut app = floating_selection_app(&ctx);
    let before = app.current().document.clone();
    let center = image_position(&app, [90., 75.]);
    let start = image_position(&app, [90., 60.]) - vec2(0., 24.);
    let radius = center.distance(start);
    let end = center + vec2(radius, 0.);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    wait_for_render(&mut app, &ctx);
    assert!(app.current().document != before);
    let rect = app.current_mut().selection_bounds().unwrap();
    assert_eq!([rect.width, rect.height], [30, 40]);
    let end = center + vec2(0.5 * radius, -0.8660254 * radius);
    frame(
        &mut app,
        &ctx,
        vec![
            Event::ModifiersChanged(Modifiers::SHIFT),
            Event::PointerMoved(end),
        ],
    );
    assert_eq!(
        app.current().selection_transform.as_ref().unwrap().desired,
        palette_core::transform::SelectionTransform::Rotate(30.)
    );
    // Finish via another command while the exact target may still be pending.
    app.current_mut().end_gesture();
    let mut expected = before.clone();
    palette_core::transform::SelectionTransform::Rotate(30.)
        .apply(&mut expected)
        .unwrap();
    assert!(app.current().document == expected);
    app.undo_current();
    assert!(app.current().document == before);
}

#[test]
fn all_eight_selection_handles_resize_without_moving_the_opposite_edge() {
    for direction in [
        [-1, -1],
        [0, -1],
        [1, -1],
        [1, 0],
        [1, 1],
        [0, 1],
        [-1, 1],
        [-1, 0],
    ] {
        let ctx = Context::default();
        let mut app = floating_selection_app(&ctx);
        let before = app.current().document.clone();
        let point = [
            90. + direction[0] as f32 * 20.,
            75. + direction[1] as f32 * 15.,
        ];
        let start = image_position(&app, point);
        let end =
            start + vec2(direction[0] as f32 * 20., direction[1] as f32 * 15.) * app.current().zoom;
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(start), pointer(start, true)],
        );
        frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![pointer(end, false)]);
        wait_for_edit(&mut app, &ctx);
        let bounds = app.current_mut().selection_bounds().unwrap();
        assert_eq!(bounds.width, if direction[0] == 0 { 40 } else { 60 });
        assert_eq!(bounds.height, if direction[1] == 0 { 30 } else { 45 });
        assert_eq!(bounds.x, if direction[0] < 0 { 50 } else { 70 });
        assert_eq!(bounds.y, if direction[1] < 0 { 45 } else { 60 });
        app.undo_current();
        assert!(app.current().document == before);
    }
}

#[test]
fn crossing_resize_handle_stays_one_pixel_and_shift_preserves_aspect_ratio() {
    let ctx = Context::default();
    let mut app = floating_selection_app(&ctx);
    let start = image_position(&app, [110., 90.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    let beyond = image_position(&app, [-100., -100.]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(beyond)]);
    wait_for_render(&mut app, &ctx);
    let bounds = app.current_mut().selection_bounds().unwrap();
    assert_eq!(
        [bounds.x, bounds.y, bounds.width, bounds.height],
        [70, 60, 1, 1]
    );
    let end = image_position(&app, [170., 100.]);
    frame(
        &mut app,
        &ctx,
        vec![
            Event::ModifiersChanged(Modifiers::SHIFT),
            Event::PointerMoved(end),
        ],
    );
    wait_for_render(&mut app, &ctx);
    let bounds = app.current_mut().selection_bounds().unwrap();
    assert_eq!([bounds.width, bounds.height], [100, 75]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.current_mut().selection_bounds().unwrap().width, 100);
}

fn wait_for_files(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while app.busy || app.recovery_pending.is_some() {
        frame(app, ctx, vec![]);
        assert!(
            std::time::Instant::now() < deadline,
            "file operation timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(app.error.is_none(), "{:?}", app.error);
}

#[test]
fn raster_import_targets_its_original_tab_and_is_one_undo_without_modifying_other_layers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layer.png");
    let pixels = Raster::new(32, 24, [20, 80, 140, 128]).unwrap();
    io::export(&path, &Document::from_raster(pixels.clone())).unwrap();
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(20, 20).unwrap();
    let before = app.current().document.clone();
    app.import_image(Some(path.clone()), &ctx);
    app.new_document();
    wait_for_files(&mut app, &ctx);
    assert_eq!(app.sessions[0].document.layers.len(), 2);
    assert_eq!(app.sessions[0].document.layers[0], before.layers[0]);
    assert_eq!(
        app.sessions[0].document.active().source.rgba(),
        pixels.rgba()
    );
    assert_eq!(
        [
            app.sessions[0].document.width,
            app.sessions[0].document.height
        ],
        [32, 24]
    );
    assert!(!app.current().modified());
    app.sessions[0].undo();
    assert!(app.sessions[0].document == before);
    assert_eq!(app.recent.len(), 1);
}

#[test]
fn stale_import_cannot_overwrite_a_document_changed_while_decoding() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    let generation = app.current().generation;
    app.current_mut()
        .command("New edit", |doc| doc.active_mut().opacity = 0.5);
    let before = app.current().document.clone();
    app.handle_auxiliary_file_result(
        FileResult::Imported {
            session: app.current().id,
            generation,
            path: PathBuf::from("stale.png"),
            result: Ok(io::ImportReport {
                document: Document::new(4, 4).unwrap(),
                notice: None,
            }),
        },
        &ctx,
    );
    assert!(app.current().document == before);
    assert!(app.message.contains("changed or closed"));
}

#[test]
fn autosave_restore_save_and_recent_history_preserve_dirty_state_and_original_files() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let original = dir.path().join("original.palette");
    io::save_project(&original, &Document::new(32, 24).unwrap()).unwrap();
    let original_bytes = std::fs::read(&original).unwrap();
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.init_store(data.clone());
    app.current_mut().document = io::open(&original).unwrap();
    app.current_mut().path = Some(original.clone());
    app.current_mut().command("Unsaved stroke", |doc| {
        doc.active_mut().source.set(2, 3, [200, 30, 80, 180])
    });
    let expected = app.current().document.clone();
    app.recovery_timer -= Duration::from_secs(31);
    app.autosave_tick(&ctx);
    wait_for_files(&mut app, &ctx);
    assert!(app.current().modified());
    assert_eq!(std::fs::read(&original).unwrap(), original_bytes);
    drop(app); // Crash simulation: release lease without normal tab cleanup.
    let mut restored = test_app(&ctx);
    restored.init_store(data.clone());
    assert!(restored.recovery_prompt);
    assert_eq!(restored.recoveries.len(), 1);
    restored.recover(0, &ctx);
    wait_for_files(&mut restored, &ctx);
    assert!(restored.current().document == expected);
    assert!(restored.current().modified());
    assert_eq!(restored.current().path.as_ref(), Some(&original));
    restored.start_save(original.clone(), false, &ctx);
    wait_for_files(&mut restored, &ctx);
    assert!(!restored.current().modified());
    assert!(io::open(&original).unwrap() == expected);
    assert_eq!(restored.recent.len(), 1);
    drop(restored);
    let mut next = test_app(&ctx);
    next.init_store(data);
    assert!(next.recoveries.is_empty());
    assert_eq!(next.recent.len(), 1);
}

#[test]
fn discard_during_pending_autosave_does_not_resurrect_a_closed_document() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.init_store(data.clone());
    app.current_mut().document = Document::new(32, 24).unwrap();
    app.current_mut().command("Unsaved", |doc| {
        doc.active_mut().source.set(2, 3, [200, 30, 80, 180])
    });
    let id = app.current().id;
    app.recovery_timer -= Duration::from_secs(31);
    app.autosave_tick(&ctx);
    assert_eq!(app.recovery_pending, Some(id));
    app.request_close(id, &ctx);
    assert_eq!(app.close_pending, Some(id));
    app.remove_tab(id, &ctx);
    wait_for_files(&mut app, &ctx);
    drop(app);
    let store = crate::persistence::Store::new(data).unwrap();
    assert!(store.discover().unwrap().is_empty());
}

fn close_button(ctx: &Context, tab: u64, slot: &str) -> egui::Response {
    ctx.read_response(egui::Id::unique(("close-button", tab, slot)))
        .expect("close button")
}

#[test]
fn close_prompt_matches_reference_geometry_and_cancel_or_outside_click_preserves_edits() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut()
        .command("Edit", |d| d.active_mut().source.set(0, 0, [1, 2, 3, 255]));
    let before = app.current().document.clone();
    let id = app.current().id;
    app.quitting = true;
    app.request_close(id, &ctx);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let save = close_button(&ctx, id, "save");
    let discard = close_button(&ctx, id, "dont-save");
    let cancel = close_button(&ctx, id, "cancel");
    assert_eq!(save.rect.size(), vec2(92., 32.));
    assert_eq!(discard.rect.min - save.rect.min, vec2(100., 0.));
    assert_eq!(cancel.rect.min - save.rect.min, vec2(200., 0.));
    assert!(
        save.rect.min.distance(pos2(547., 517.5)) <= 0.5,
        "centered at physical-pixel boundary"
    );
    assert!(save.has_focus());
    let body = format!("There are unsaved changes in “{}”.", app.current().name);
    let bounds = label_bounds(&mut app, &ctx, &body);
    assert_eq!(bounds.left(), save.rect.left());
    assert!(bounds.right() <= save.rect.left() + 292.);
    click_ui(&mut app, &ctx, pos2(10., 300.));
    assert_eq!(app.close_pending, Some(id));
    assert!(app.current().document == before);
    click_ui(&mut app, &ctx, cancel.rect.center());
    assert!(app.close_pending.is_none());
    assert!(!app.quitting);
    assert!(app.current().modified());
    assert!(app.current().document == before);
    app.request_close(id, &ctx);
    frame(&mut app, &ctx, vec![]);
    // Real Tab navigation must reach the secondary actions rather than being
    // pulled back to Save by initial-focus handling.
    for slot in ["dont-save", "cancel"] {
        let tab = |pressed| Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        frame(&mut app, &ctx, vec![tab(true)]);
        frame(&mut app, &ctx, vec![tab(false)]);
        assert!(
            close_button(&ctx, id, slot).has_focus(),
            "Tab reaches {slot}"
        );
    }
    frame(&mut app, &ctx, vec![enter_event()]);
    assert!(app.close_pending.is_none());
    app.request_close(id, &ctx);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.close_pending.is_none());
    assert!(app.current().document == before);
}

#[test]
fn close_prompt_discards_only_requested_tab_and_save_waits_for_atomic_write() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut()
        .command("Edit", |d| d.active_mut().source.set(0, 0, [1, 2, 3, 255]));
    let id = app.current().id;
    let expected = app.current().document.clone();
    app.current_mut().path = Some(dir.path().join("save.palette"));
    app.new_document();
    let other = app.current().id;
    app.request_close(id, &ctx);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    // Enter on the initial Save focus invokes the actual background writer.
    frame(&mut app, &ctx, vec![enter_event()]);
    wait_for_files(&mut app, &ctx);
    assert!(app.sessions.iter().all(|s| s.id != id));
    assert_eq!(app.current().id, other);
    assert!(io::open(&dir.path().join("save.palette")).unwrap() == expected);
    app.current_mut().command("Edit other", |d| {
        d.active_mut().source.set(1, 1, [3, 2, 1, 255])
    });
    app.new_document();
    let third = app.current().id;
    app.request_close(other, &ctx);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let discard = close_button(&ctx, other, "dont-save");
    click_ui(&mut app, &ctx, discard.rect.center());
    assert!(app.sessions.iter().all(|s| s.id != other));
    assert_eq!(app.current().id, third);
}

#[test]
fn paint_export_cancel_and_native_backup_are_bound_to_the_same_document_revision() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().command("Live filter", |doc| {
        let id = doc.allocate_id();
        doc.active_mut().filters.push(FilterInstance {
            id,
            enabled: true,
            filter: Filter::Invert,
            mask: None,
        });
    });
    let before = app.current().document.clone();
    let path = dir.path().join("export.paint");
    let draft = || files::PaintExport {
        path: path.clone(),
        session: 1,
        revision: 1,
        backup_complete: false,
    };
    let mut export = draft();
    export.session = app.current().id;
    export.revision = app.current().history.revision;
    app.paint_export = Some(export);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.paint_export.is_none());
    assert!(!path.exists());
    assert!(app.current().document == before);
    let mut export = draft();
    export.session = app.current().id;
    export.revision = app.current().history.revision;
    app.paint_export = Some(export);
    let native = dir.path().join("full.palette");
    app.start_save(native.clone(), false, &ctx);
    wait_for_files(&mut app, &ctx);
    assert!(app.paint_export.as_ref().unwrap().backup_complete);
    assert!(io::open(&native).unwrap() == before);
    app.current_mut()
        .command("New edit", |doc| doc.active_mut().opacity = 0.5);
    frame(&mut app, &ctx, vec![]);
    assert!(app.paint_export.is_none());
    assert!(app.error.as_ref().unwrap().contains("document changed"));
    assert!(!path.exists());
}

fn label_bounds(app: &mut PaletteApp, ctx: &Context, label: &str) -> egui::Rect {
    scoped_label_bounds(app, ctx, label, false)
}
fn scoped_label_bounds(
    app: &mut PaletteApp,
    ctx: &Context,
    label: &str,
    toolbar: bool,
) -> egui::Rect {
    fn find(shape: &egui::epaint::Shape, label: &str, bounds: Rect) -> Option<egui::Rect> {
        match shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                bounds.contains_rect(rect).then_some(rect)
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label, bounds)),
            _ => None,
        }
    }
    // Areas have an invisible sizing pass and may re-anchor after changing
    // content. Measure stable geometry, and clear texture deltas before a test
    // assertion can unwind (epaint intentionally panics on unhandled deltas).
    let mut previous = None;
    for _ in 0..24 {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        let bounds = if toolbar {
            app.processing_toolbar_rect
        } else {
            Rect::EVERYTHING
        };
        let point = output
            .shapes
            .iter()
            .find_map(|s| find(&s.shape, label, bounds));
        output.textures_delta.clear();
        if let Some(point) = point {
            if previous == Some(point) {
                return point;
            }
            previous = Some(point);
        }
    }
    panic!("stable visible label: {label}");
}
fn label_position(app: &mut PaletteApp, ctx: &Context, label: &str) -> egui::Pos2 {
    label_bounds(app, ctx, label).center()
}

#[test]
fn paint_export_ui_blocks_baking_before_native_save_then_writes_a_separate_copy() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(12, 10).unwrap();
    app.current_mut().command("Add filter", |doc| {
        let id = doc.allocate_id();
        doc.active_mut().filters.push(FilterInstance {
            id,
            enabled: true,
            filter: Filter::Invert,
            mask: None,
        });
    });
    let before = app.current().document.clone();
    let path = dir.path().join("copy.paint");
    app.paint_export = Some(files::PaintExport {
        path: path.clone(),
        session: app.current().id,
        revision: app.current().history.revision,
        backup_complete: false,
    });
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let button = label_position(&mut app, &ctx, "Export copy");
    click_ui(&mut app, &ctx, button);
    assert!(!app.busy);
    assert!(!path.exists());
    assert!(app.paint_export.is_some());
    let native = dir.path().join("complete.palette");
    app.start_save(native.clone(), false, &ctx);
    wait_for_files(&mut app, &ctx);
    assert!(app.paint_export.as_ref().unwrap().backup_complete);
    let button = label_position(&mut app, &ctx, "Export copy");
    click_ui(&mut app, &ctx, button);
    wait_for_files(&mut app, &ctx);
    assert!(app.paint_export.is_none());
    assert!(io::open(&native).unwrap() == before);
    assert!(app.current().document == before);
    assert_eq!(app.current().path.as_ref(), Some(&native));
    assert!(composite::render(&io::open(&path).unwrap()) == composite::render(&before));
}

fn modified_click(app: &mut PaletteApp, ctx: &Context, p: egui::Pos2, modifiers: Modifiers) {
    for pressed in [true, false] {
        frame(
            app,
            ctx,
            vec![
                Event::ModifiersChanged(modifiers),
                Event::PointerMoved(p),
                Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ],
        );
    }
    frame(app, ctx, vec![Event::ModifiersChanged(Modifiers::NONE)]);
}
#[test]
fn wand_clicks_combine_at_fractional_zoom_and_keep_tabs_sources_and_undo_independent() {
    use palette_core::selection::{Combine, Wand};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(24, 16).unwrap();
    for x in [2, 3, 14, 15] {
        for y in 3..7 {
            app.current_mut()
                .document
                .active_mut()
                .source
                .set(x, y, [250, 0, 50, 255]);
        }
    }
    app.current_mut().wand = Wand {
        tolerance: 0.,
        antialias: false,
        ..Default::default()
    };
    app.choose_tool(Tool::Wand);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let before = app.current().document.clone();
    let first = image_position(&app, [2.5, 4.5]);
    let second = image_position(&app, [14.5, 4.5]);
    click_ui(&mut app, &ctx, first);
    wait_for_edit(&mut app, &ctx);
    let one = app.current().document.clone();
    assert_eq!(
        one.selection
            .as_ref()
            .unwrap()
            .values
            .iter()
            .filter(|v| **v == 255)
            .count(),
        8
    );
    modified_click(&mut app, &ctx, second, Modifiers::SHIFT);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current()
            .document
            .selection
            .as_ref()
            .unwrap()
            .values
            .iter()
            .filter(|v| **v == 255)
            .count(),
        16
    );
    modified_click(&mut app, &ctx, first, Modifiers::ALT);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.selection.as_ref().unwrap().at(2, 4),
        0.
    );
    assert_eq!(
        app.current().document.selection.as_ref().unwrap().at(14, 4),
        1.
    );
    assert_eq!(
        app.current().document.active().source,
        before.active().source
    );
    app.undo_current();
    app.undo_current();
    assert!(app.current().document == one);
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    assert!(app.current().document == one);
    app.current_mut().selection_mode = Combine::Subtract;
    app.new_document();
    assert_eq!(app.current().selection_mode, Combine::Replace);
    assert!(app.current().document.selection.is_none());
    app.active = 0;
    assert!(app.current().document == one);
    assert_eq!(app.current().selection_mode, Combine::Subtract);
}

#[test]
fn rectangle_combination_mode_does_not_move_pixels_inside_an_existing_selection() {
    use palette_core::selection::Combine;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(100, 100).unwrap();
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        100,
        100,
        palette_core::editing::PixelRect {
            x: 20,
            y: 20,
            width: 20,
            height: 20,
        },
    ));
    app.choose_tool(Tool::Select);
    app.current_mut().selection_mode = Combine::Add;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let source = app.current().document.active().source.clone();
    let a = image_position(&app, [30., 30.]);
    let b = image_position(&app, [50., 50.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(a), pointer(a, true)],
    );
    assert!(app.current().selection_transform.is_none());
    frame(&mut app, &ctx, vec![Event::PointerMoved(b)]);
    frame(&mut app, &ctx, vec![pointer(b, false)]);
    assert_eq!(
        app.current()
            .document
            .selection
            .as_ref()
            .unwrap()
            .at(20, 20),
        1.
    );
    assert_eq!(
        app.current()
            .document
            .selection
            .as_ref()
            .unwrap()
            .at(49, 49),
        1.
    );
    assert_eq!(
        app.current()
            .document
            .selection
            .as_ref()
            .unwrap()
            .at(45, 20),
        0.
    );
    assert!(app.current().document.active().source == source);
}

#[test]
fn feather_and_mask_controls_are_transactional_source_preserving_and_saved_editably() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(50, 40).unwrap();
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        50,
        40,
        palette_core::editing::PixelRect {
            x: 10,
            y: 8,
            width: 20,
            height: 20,
        },
    ));
    let before = app.current().document.clone();
    app.show_feather(false);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.current().document == before);
    app.show_feather(false);
    app.feather_dialog.as_mut().unwrap().radius = 2.;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    wait_for_edit(&mut app, &ctx);
    let soft = app.current().document.clone();
    assert!(
        soft.selection
            .as_ref()
            .unwrap()
            .values
            .iter()
            .any(|v| *v > 0 && *v < 255)
    );
    app.interface.layer_properties = true;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let add = label_position(&mut app, &ctx, "Mask from selection");
    click_ui(&mut app, &ctx, add);
    wait_for_edit(&mut app, &ctx);
    let masked = app.current().document.clone();
    assert!(masked.active().source == before.active().source);
    assert_eq!(
        masked.active().mask.as_ref().unwrap().values,
        soft.selection.as_ref().unwrap().values
    );
    let invert = label_position(&mut app, &ctx, "Invert mask");
    click_ui(&mut app, &ctx, invert);
    wait_for_edit(&mut app, &ctx);
    let inverted = app.current().document.clone();
    assert_eq!(inverted.selection, soft.selection);
    assert_eq!(inverted.active().mask.as_ref().unwrap().at(0, 0), 1.);
    app.undo_current();
    assert!(app.current().document == masked);
    app.redo_current();
    assert!(app.current().document == inverted);
    let enabled = label_position(&mut app, &ctx, "Enabled");
    click_ui(&mut app, &ctx, enabled);
    wait_for_edit(&mut app, &ctx);
    assert!(
        !app.current()
            .document
            .active()
            .mask
            .as_ref()
            .unwrap()
            .enabled
    );
    assert!(app.current().document.active().source == before.active().source);
    let disabled = app.current().document.clone();
    app.show_feather(true);
    app.feather_dialog.as_mut().unwrap().radius = 3.;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    wait_for_edit(&mut app, &ctx);
    assert!(
        !app.current()
            .document
            .active()
            .mask
            .as_ref()
            .unwrap()
            .enabled
    );
    assert!(app.current().document.active().source == before.active().source);
    assert_ne!(app.current().document.active().mask, disabled.active().mask);
    app.undo_current();
    assert!(app.current().document == disabled);
    let saved = app.current().document.clone();
    let path = dir.path().join("masked.palette");
    app.start_save(path.clone(), false, &ctx);
    wait_for_files(&mut app, &ctx);
    assert!(io::open(&path).unwrap() == saved);
    let delete = label_position(&mut app, &ctx, "Delete mask");
    click_ui(&mut app, &ctx, delete);
    wait_for_edit(&mut app, &ctx);
    assert!(app.current().document.active().mask.is_none());
    app.undo_current();
    assert!(app.current().document == saved);
}

#[test]
fn selection_submenus_and_blend_dropdown_change_real_settings() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    // Chevron position from the reference-resolution ribbon capture.
    click_ui(&mut app, &ctx, pos2(39., 86.));
    let wand = label_position(&mut app, &ctx, "      Magic Wand");
    click_ui(&mut app, &ctx, wand);
    assert_eq!(app.tool, Tool::Wand);
    click_ui(&mut app, &ctx, pos2(39., 86.));
    let options = label_position(&mut app, &ctx, "Magic Wand options");
    click_ui(&mut app, &ctx, options);
    let contiguous = label_position(&mut app, &ctx, "Contiguous");
    click_ui(&mut app, &ctx, contiguous);
    assert!(!app.current().wand.contiguous);
    egui::Popup::close_all(&ctx);
    app.interface.layer_properties = true;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let blending = app.layer_processing_buttons[2].center();
    click_ui(&mut app, &ctx, blending);
    let burn = label_position(&mut app, &ctx, "Color Burn");
    click_ui(&mut app, &ctx, burn);
    assert_eq!(app.current().document.active().blend, BlendMode::ColorBurn);
    app.undo_current();
    assert_eq!(app.current().document.active().blend, BlendMode::Normal);
}

#[test]
fn selection_toolbar_matches_text_height_fits_content_and_does_not_shift_canvas() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    frame(&mut app, &ctx, vec![]);
    let image = app.image_rect;
    app.choose_tool(Tool::Select);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let rectangle = app.selection_toolbar_rect;
    assert_eq!(app.image_rect, image);
    assert!((rectangle.top() - app.canvas_rect.top() - 4.).abs() < 1.);
    app.choose_tool(Tool::Lasso);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    assert_eq!(app.selection_toolbar_rect.size(), rectangle.size());
    app.choose_tool(Tool::Wand);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let wand = app.selection_toolbar_rect;
    assert!(wand.width() > rectangle.width() + 100.);
    assert_eq!(wand.height(), rectangle.height());
    assert_eq!(app.image_rect, image);
    app.choose_tool(Tool::Text);
    app.start_text(palette_core::editing::PixelRect {
        x: 100,
        y: 200,
        width: 300,
        height: 100,
    });
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let text = ctx
        .read_response(egui::Id::unique("text-toolbar"))
        .unwrap()
        .rect;
    assert!(
        (text.height() - wand.height()).abs() < 1.,
        "text={text:?}, wand={wand:?}"
    );
    assert_eq!(app.selection_toolbar_rect, Rect::NOTHING);
}

#[test]
fn selection_toolbar_numeric_fields_are_precise_validated_and_tab_local() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().zoom = 2.;
    app.choose_tool(Tool::Wand);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.current().document.clone();
    for (name, input, expected) in [
        ("Tolerance", "35.5", 35.5),
        ("Feather", "0", 0.),
        ("Feather", "2", 2.),
        ("Feather", "4.26", 4.3),
        ("Feather", "5", 5.),
        ("Feather", "8.5", 8.5),
        ("Feather", "NaN", 8.5),
    ] {
        let id = egui::Id::unique(("selection-number", app.current().id, name));
        let position = ctx.read_response(id).unwrap().rect.center();
        click_ui(&mut app, &ctx, position);
        frame(
            &mut app,
            &ctx,
            vec![Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            }],
        );
        frame(
            &mut app,
            &ctx,
            vec![Event::Text(input.into()), enter_event()],
        );
        assert_eq!(
            if name == "Tolerance" {
                app.current().wand.tolerance
            } else {
                app.current().selection_feather
            },
            expected
        );
        if name == "Feather" && expected < 5. {
            assert_eq!(
                app.current().selection_feather_text,
                format!("{expected:.1}")
            );
        }
    }
    assert!(app.current().document == before);
    assert!(!app.edit_pending);
    app.new_document();
    assert_eq!(app.current().selection_feather, 0.);
    app.active = 0;
    assert_eq!(app.current().selection_feather, 8.5);
}

#[test]
fn freeform_preview_cancel_commit_and_feather_at_fractional_zoom_are_one_undo() {
    use palette_core::selection::{self, Combine};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.choose_tool(Tool::Lasso);
    app.current_mut().selection_feather = 3.;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let before = app.current().document.clone();
    let points = [
        [300.25, 250.5],
        [450.25, 260.5],
        [420.25, 390.5],
        [300.25, 360.5],
    ];
    let start = image_position(&app, points[0]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    for p in &points[1..] {
        let p = image_position(&app, *p);
        frame(&mut app, &ctx, vec![Event::PointerMoved(p)]);
    }
    assert!(app.current().lasso_drag.as_ref().unwrap().points.len() >= 4);
    assert!(app.current().document == before);
    let end = image_position(&app, points[3]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_edit(&mut app, &ctx);
    let expected =
        selection::feather(&selection::freeform(1280, 720, &points).unwrap(), 3.).unwrap();
    assert_eq!(app.current().document.selection.as_ref(), Some(&expected));
    assert_eq!(
        app.current().document.active().source,
        before.active().source
    );
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    let selected = app.current().document.clone();
    app.current_mut().selection_mode = Combine::Add;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    frame(&mut app, &ctx, vec![pointer(start, false)]);
    assert!(app.current().lasso_drag.is_none());
    assert!(app.current().document == selected);
}

#[test]
fn rectangle_and_wand_feather_new_coverage_before_combining() {
    use palette_core::{
        editing::{PixelRect, rectangle},
        selection::{self, Combine, Wand},
    };
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(24, 16).unwrap();
    let rect = PixelRect {
        x: 4,
        y: 4,
        width: 10,
        height: 8,
    };
    let raw = rectangle(24, 16, rect);
    let expected = selection::feather(&raw, 2.).unwrap();
    app.current_mut().selection_feather = 2.;
    app.edit(editing::Edit::Rectangle(rect, Combine::Replace), &ctx);
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.current().document.selection.as_ref(), Some(&expected));
    app.undo_current();
    for y in 4..12 {
        for x in 4..14 {
            app.current_mut()
                .document
                .active_mut()
                .source
                .set(x, y, [200, 0, 0, 255]);
        }
    }
    app.edit(
        editing::Edit::Wand(
            [6, 6],
            Wand {
                tolerance: 0.,
                antialias: false,
                ..Default::default()
            },
            Combine::Replace,
        ),
        &ctx,
    );
    wait_for_edit(&mut app, &ctx);
    assert_eq!(app.current().document.selection.as_ref(), Some(&expected));
}

#[test]
fn toolbar_combine_icons_and_inversion_work_without_clicking_through_to_the_canvas() {
    use palette_core::selection::Combine;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    // The canvas extends behind the toolbar: an accidental fallthrough would
    // start a rectangle gesture or transform on every toolbar press.
    app.current_mut().zoom = 2.;
    app.choose_tool(Tool::Select);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.current().document.clone();
    for (index, mode) in [
        Combine::Replace,
        Combine::Add,
        Combine::Subtract,
        Combine::Intersect,
    ]
    .into_iter()
    .enumerate()
    {
        let rect = app.selection_toolbar_rect;
        let button = pos2(rect.right() - 166. + index as f32 * 33., rect.center().y);
        assert!(app.image_rect.contains(button));
        click_ui(&mut app, &ctx, button);
        assert_eq!(app.current().selection_mode, mode);
        assert!(app.current().selection_drag.is_none());
        assert!(app.current().document == before);
    }
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        1280,
        720,
        palette_core::editing::PixelRect {
            x: 10,
            y: 10,
            width: 10,
            height: 10,
        },
    ));
    let selected = app.current().document.clone();
    let rect = app.selection_toolbar_rect;
    click_ui(&mut app, &ctx, pos2(rect.right() - 22., rect.center().y));
    let mask = app.current().document.selection.as_ref().unwrap();
    assert_eq!(mask.at(15, 15), 0.);
    assert_eq!(mask.at(100, 100), 1.);
    assert_eq!(
        app.current().document.active().source,
        before.active().source
    );
    app.undo_current();
    assert!(app.current().document == selected);
}

#[test]
fn compact_selection_toolbars_fit_above_canvas_with_layer_properties_and_high_dpi() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    for properties in [false, true] {
        app.interface.layer_properties = properties;
        for dpi in [1., 1.5, 2.] {
            ctx.set_pixels_per_point(dpi);
            for tool in [
                Tool::Select,
                Tool::Wand,
                Tool::Lasso,
                Tool::PolygonSelect,
                Tool::SelectBrush,
            ] {
                app.choose_tool(tool);
                for _ in 0..4 {
                    let mut output = ctx.run_ui(
                        RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                egui::Pos2::ZERO,
                                vec2(780., 520.),
                            )),
                            ..Default::default()
                        },
                        |ui| app.draw(ui),
                    );
                    output.textures_delta.clear();
                }
                assert!(
                    app.canvas_rect.contains_rect(app.selection_toolbar_rect),
                    "{tool:?}: {:?} outside {:?}",
                    app.selection_toolbar_rect,
                    app.canvas_rect
                );
                assert!((app.selection_toolbar_rect.height() - 46.).abs() < 1.);
            }
        }
    }
}

#[test]
fn freeform_selection_is_available_through_the_ribbon_and_bounds_long_drags() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    click_ui(&mut app, &ctx, pos2(39., 86.));
    let freeform = label_position(&mut app, &ctx, "      Freeform selection");
    click_ui(&mut app, &ctx, freeform);
    assert_eq!(app.tool, Tool::Lasso);
    let mut drag = crate::session::LassoDrag {
        points: vec![[0., 0.]],
        mode: palette_core::selection::Combine::Replace,
    };
    for i in 1..20_000 {
        drag.sample([i as f32, (i % 15) as f32], 1.);
    }
    assert!(drag.points.len() <= 4096);
    assert_eq!(drag.points[0], [0., 0.]);
    assert_eq!(drag.points.last(), Some(&[19_999., 4.]));
}

#[test]
fn wand_slider_moves_from_exact_to_all_colors_and_does_not_invert_coverage() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(256, 32).unwrap();
    for x in 0..256 {
        for y in 0..32 {
            app.current_mut().document.active_mut().source.set(
                x,
                y,
                [x as u8, x as u8, x as u8, 255],
            );
        }
    }
    app.current_mut().wand.antialias = false;
    app.choose_tool(Tool::Wand);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let id = egui::Id::unique(("selection-number", app.current().id, "Tolerance"));
    let field = ctx.read_response(id).unwrap().rect;
    let seed = image_position(&app, [0.5, 15.5]);
    let center = pos2(field.left() - 36., field.center().y);
    let left = center - vec2(120., 0.);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(center), pointer(center, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(left)]);
    frame(&mut app, &ctx, vec![pointer(left, false)]);
    assert!(
        app.current().wand.tolerance < 1.,
        "value={}, field={field:?}, toolbar={:?}",
        app.current().wand.tolerance,
        app.selection_toolbar_rect
    );
    click_ui(&mut app, &ctx, seed);
    wait_for_edit(&mut app, &ctx);
    let count = app
        .current()
        .document
        .selection
        .as_ref()
        .unwrap()
        .values
        .iter()
        .filter(|v| **v > 0)
        .count();
    let right = center + vec2(120., 0.);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(center), pointer(center, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(right)]);
    frame(&mut app, &ctx, vec![pointer(right, false)]);
    assert!(app.current().wand.tolerance > 99.);
    click_ui(&mut app, &ctx, seed);
    wait_for_edit(&mut app, &ctx);
    assert!(count < 256 * 32);
    assert!(
        app.current()
            .document
            .selection
            .as_ref()
            .unwrap()
            .values
            .iter()
            .all(|v| *v == 255)
    );
}

#[test]
fn scrollbars_drag_both_axes_keep_vertical_rail_clear_of_layers_and_do_not_edit() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().zoom = 2.;
    app.interface.layer_properties = true;
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.current().document.clone();
    for axis in 0..2 {
        let rail = app.canvas_scrollbars[axis].expect("oversized canvas rail");
        assert!(!app.canvas_rect.intersects(rail));
        if axis == 1 {
            assert!(rail.right() < 1386. - 12. - 180.);
        }
        let start = rail.center();
        let mut end = start;
        end[axis] = rail.max[axis] - 1.;
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(start), pointer(start, true)],
        );
        frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![pointer(end, false)]);
        assert!(app.current().pan[axis] < -100.);
        assert!(app.current().document == before);
    }
    let pan = app.current().pan;
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::ArrowUp,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(app.current().pan.y > pan.y);
    assert!(app.current().document == before);
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::End,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let pan = app.current().pan;
    app.new_document();
    app.active = 0;
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.current().pan, pan);
    app.current_mut().fit = true;
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.canvas_scrollbars, [None, None]);
    assert_eq!(app.current().pan, Vec2::ZERO);
}

#[test]
fn inverted_selection_draws_animated_ants_on_every_visible_canvas_edge() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().zoom = 2.;
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        1280,
        720,
        palette_core::editing::PixelRect {
            x: 500,
            y: 300,
            width: 100,
            height: 100,
        },
    ));
    app.invert_selection();
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    let view = app.canvas_rect;
    let mut sides = [false; 4];
    for shape in &output.shapes {
        if let egui::Shape::LineSegment { points, stroke } = shape.shape
            && stroke.color == Color32::BLACK
            && view.contains(points[0])
            && view.contains(points[1])
        {
            for (index, distance) in [
                (points[0].x - view.left()).abs(),
                (points[0].x - view.right()).abs(),
                (points[0].y - view.top()).abs(),
                (points[0].y - view.bottom()).abs(),
            ]
            .into_iter()
            .enumerate()
            {
                sides[index] |= distance < 1.;
            }
        }
    }
    output.textures_delta.clear();
    assert!(sides.into_iter().all(|v| v), "{sides:?}");
}

#[test]
fn feather_draws_original_opaque_ants_and_separate_forty_percent_extent() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(819, 594).unwrap();
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
        &mut app.current_mut().document,
        &raw,
        palette_core::selection::Combine::Replace,
        20.,
    )
    .unwrap();
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    let (mut original, mut feather) = (false, false);
    let original_left = app.image_rect.left() + 240. * app.current().zoom;
    for shape in &output.shapes {
        if let egui::Shape::LineSegment { points, stroke } = shape.shape
            && app.image_rect.contains(points[0])
        {
            original |= stroke.color == Color32::BLACK && (points[0].x - original_left).abs() < 0.1;
            feather |=
                stroke.color == Color32::from_black_alpha(102) && points[0].x < original_left - 5.;
        }
    }
    output.textures_delta.clear();
    assert!(original && feather);
    assert_eq!(app.current().document.selection_base.as_ref(), Some(&raw));
}

fn wait_for_feather_frame(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        frame(app, ctx, vec![]);
        let preview = app
            .current()
            .feather_preview
            .as_ref()
            .expect("live feather transaction");
        if preview
            .frame
            .as_ref()
            .is_some_and(|f| f.radius == preview.radius)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "feather preview timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn feather_toolbar_outline_grows_while_held_and_release_commits_one_undo() {
    use palette_core::{
        editing::{PixelRect, rectangle},
        selection,
    };
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(819, 594).unwrap();
    let original = rectangle(
        819,
        594,
        PixelRect {
            x: 240,
            y: 180,
            width: 300,
            height: 210,
        },
    );
    app.current_mut().document.selection = Some(original.clone());
    app.choose_tool(Tool::Select);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.current().document.clone();
    let revision = app.current().history.revision;
    let field = ctx
        .read_response(egui::Id::unique((
            "selection-number",
            app.current().id,
            "Feather",
        )))
        .unwrap()
        .rect;
    let start = pos2(field.left() - 63., field.center().y);
    frame(&mut app, &ctx, vec![Event::PointerMoved(start)]);
    frame(&mut app, &ctx, vec![pointer(start, true)]);
    wait_for_feather_frame(&mut app, &ctx);
    let preview = app.current().feather_preview.as_ref().unwrap();
    let first = preview.frame.as_ref().unwrap().bounds.unwrap();
    let first_radius = preview.radius;
    assert!(!preview.commit_requested);
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, revision);
    let end = pos2(field.left() - 36., field.center().y);
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    wait_for_feather_frame(&mut app, &ctx);
    let preview = app.current().feather_preview.as_ref().unwrap();
    let last = preview.frame.as_ref().unwrap().bounds.unwrap();
    let radius = preview.radius;
    assert!(radius > first_radius);
    assert!(last.width > first.width || last.height > first.height);
    assert!(app.current().document == before);
    assert_eq!(
        app.current_mut().selection_contours(),
        palette_core::editing::contours(&original)
    );
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_edit(&mut app, &ctx);
    assert!(app.current().feather_preview.is_none());
    assert_eq!(
        app.current().document.selection.as_ref(),
        Some(&selection::feather(&original, radius).unwrap())
    );
    let committed = app.current().document.clone();
    app.undo_current();
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, revision);
    app.redo_current();
    assert!(app.current().document == committed);
}

#[test]
fn feather_dialog_previews_live_without_mutating_document_and_cancel_discards_it() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(80, 60).unwrap();
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        80,
        60,
        palette_core::editing::PixelRect {
            x: 20,
            y: 15,
            width: 30,
            height: 24,
        },
    ));
    let before = app.current().document.clone();
    app.show_feather(false);
    wait_for_feather_frame(&mut app, &ctx);
    // The shifted logarithm must display actual units, not the +10 offset.
    let value = label_position(&mut app, &ctx, "4.0");
    assert!(value.is_finite());
    let label = label_bounds(&mut app, &ctx, "Radius (px)");
    let slider = pos2(label.left() - 100., label.center().y);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(slider), pointer(slider, true)],
    );
    wait_for_feather_frame(&mut app, &ctx);
    let preview = app.current().feather_preview.as_ref().unwrap();
    assert!(preview.dialog && preview.radius > 4.);
    assert!(!preview.frame.as_ref().unwrap().contours.is_empty());
    assert!(app.current().document == before);
    frame(&mut app, &ctx, vec![pointer(slider, false)]);
    let cancel = label_position(&mut app, &ctx, "Cancel");
    click_ui(&mut app, &ctx, cancel);
    assert!(app.feather_dialog.is_none());
    assert!(app.current().feather_preview.is_none());
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, 0);
}

#[test]
fn feather_worker_coalesces_radii_fences_cancelled_jobs_and_finishes_before_tab_changes() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(64, 48).unwrap();
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        64,
        48,
        palette_core::editing::PixelRect {
            x: 20,
            y: 15,
            width: 16,
            height: 12,
        },
    ));
    let before = app.current().document.clone();
    let raw = before.selection.as_ref().unwrap().clone();
    app.preview_selection_feather(2., false, 0.);
    app.request_feather();
    app.preview_selection_feather(12., false, 0.);
    app.commit_selection_feather();
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.selection.as_ref(),
        Some(&palette_core::selection::feather(&raw, 12.).unwrap())
    );
    app.undo_current();
    assert!(app.current().document == before);
    app.preview_selection_feather(18., false, 0.);
    app.request_feather();
    app.current_mut().cancel_selection_feather();
    app.preview_selection_feather(3., false, 0.);
    app.commit_selection_feather();
    wait_for_edit(&mut app, &ctx);
    assert_eq!(
        app.current().document.selection.as_ref(),
        Some(&palette_core::selection::feather(&raw, 3.).unwrap())
    );
    app.preview_selection_feather(9., false, 0.);
    app.request_feather();
    app.new_document();
    assert!(app.current().document.selection.is_none());
    assert_eq!(
        app.sessions[0].document.selection.as_ref(),
        Some(&palette_core::selection::feather(&raw, 9.).unwrap())
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.feather_worker.busy {
        frame(&mut app, &ctx, vec![]);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(app.current().document.selection.is_none());
}

fn wait_for_brush_frame(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        frame(app, ctx, vec![]);
        assert!(app.error.is_none(), "{:?}", app.error);
        let draft = app
            .current()
            .selection_brush_draft
            .as_ref()
            .expect("brush draft");
        if draft
            .frame
            .as_ref()
            .is_some_and(|f| f.samples == draft.points.len())
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "brush preview timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn selection_brush_previews_while_held_at_fractional_zoom_and_commits_one_undo() {
    let ctx = Context::default();
    ctx.set_pixels_per_point(1.5);
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(240, 180).unwrap();
    app.current_mut().document.active_mut().locked = true;
    app.current_mut().zoom = 1.25;
    app.choose_tool(Tool::SelectBrush);
    app.current_mut().selection_brush.diameter = 24.;
    app.current_mut().selection_brush.hardness = 0.5;
    app.current_mut().selection_brush.strength = 0.6;
    app.current_mut().selection_feather = 2.5;
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let before = app.current().document.clone();
    let start = image_position(&app, [30.5, 90.5]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    wait_for_brush_frame(&mut app, &ctx);
    let first = app
        .current()
        .selection_brush_draft
        .as_ref()
        .unwrap()
        .frame
        .as_ref()
        .unwrap()
        .bounds
        .unwrap();
    let end = image_position(&app, [170.5, 90.5]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    wait_for_brush_frame(&mut app, &ctx);
    let draft = app.current().selection_brush_draft.as_ref().unwrap();
    let f = draft.frame.as_ref().unwrap();
    assert!(f.bounds.unwrap().width > first.width + 100);
    assert!(!f.contours.is_empty() && !f.feather.is_empty());
    assert!(draft.overlay.is_some());
    assert!(!draft.released);
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, 0);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_edit(&mut app, &ctx);
    assert!(app.current().selection_brush_draft.is_none());
    assert!(app.current().document.layers == before.layers);
    assert_eq!(
        app.current().selection_mode,
        palette_core::selection::Combine::Add
    );
    let committed = app.current().document.clone();
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    assert!(app.current().document == committed);
}

#[test]
fn selection_brush_cancel_stale_results_and_tab_switches_preserve_the_complete_target() {
    use palette_core::selection;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(160, 120).unwrap();
    app.choose_tool(Tool::SelectBrush);
    frame(&mut app, &ctx, vec![]);
    let before = app.current().document.clone();
    let start = image_position(&app, [30., 60.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    let old = app.current().selection_brush_draft.as_ref().unwrap().token;
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(app.current().selection_brush_draft.is_none());
    assert!(app.current().document == before);
    frame(&mut app, &ctx, vec![pointer(start, false)]);
    let start = image_position(&app, [40.5, 60.5]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    let end = image_position(&app, [130.5, 70.5]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    let draft = app.current().selection_brush_draft.as_ref().unwrap();
    assert_ne!(draft.token, old);
    let expected = selection::brush::from_points(&before, &draft.points, draft.settings).unwrap();
    app.new_document(); // finishes exact pending target, including samples not yet processed
    assert_eq!(app.sessions[0].document.selection.as_ref(), Some(&expected));
    assert_eq!(app.sessions[0].history.revision, 1);
    assert!(app.current().document.selection.is_none());
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while app.selection_brush_worker.busy {
        frame(&mut app, &ctx, vec![]);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.sessions[0].document.selection.as_ref(), Some(&expected));
    assert!(app.current().document.selection.is_none());
    app.active = 0;
    app.undo_current();
    assert!(app.current().document == before);
}

#[test]
fn selection_brush_options_are_reachable_precise_and_independent_of_paint_and_other_tabs() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    click_ui(&mut app, &ctx, pos2(39., 86.));
    let tool = label_position(&mut app, &ctx, "      Selection brush");
    click_ui(&mut app, &ctx, tool);
    assert_eq!(app.tool, Tool::SelectBrush);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let paint_size = app.brush.diameter;
    let before = app.current().selection_brush.diameter;
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::CloseBracket,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(app.current().selection_brush.diameter > before);
    assert_eq!(app.brush.diameter, paint_size);
    let field = ctx
        .read_response(egui::Id::unique((
            "selection-number",
            app.current().id,
            "Tolerance",
        )))
        .unwrap()
        .rect
        .center();
    click_ui(&mut app, &ctx, field);
    assert!(
        ctx.memory(|m| m.has_focus(egui::Id::unique((
            "selection-number",
            app.current().id,
            "Tolerance"
        )))),
        "tolerance field must receive keyboard focus"
    );
    frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }],
    );
    frame(
        &mut app,
        &ctx,
        vec![Event::Text("6.5".into()), enter_event()],
    );
    assert_eq!(app.current().selection_brush.tolerance, 6.5);
    let options = label_position(&mut app, &ctx, "...");
    click_ui(&mut app, &ctx, options);
    let matching = label_position(&mut app, &ctx, "Color matching / edge aware");
    click_ui(&mut app, &ctx, matching);
    assert!(!app.current().selection_brush.edge_aware);
    let options = label_position(&mut app, &ctx, "...");
    click_ui(&mut app, &ctx, options);
    let merged = label_position(&mut app, &ctx, "Sample all visible layers");
    assert!(merged.is_finite()); // disabled while matching is off
    let overlay = label_position(&mut app, &ctx, "Selection overlay");
    click_ui(&mut app, &ctx, overlay);
    assert!(!app.current().brush_overlay_enabled);
    app.new_document();
    assert_eq!(app.current().selection_brush.tolerance, 12.);
    assert!(app.current().selection_brush.edge_aware && app.current().brush_overlay_enabled);
}

#[test]
fn brush_selection_combines_modes_and_modifier_subtraction_without_touching_image_pixels() {
    use palette_core::selection::{self, Combine};
    for (mode, mods) in [
        (Combine::Replace, Modifiers::NONE),
        (Combine::Add, Modifiers::NONE),
        (Combine::Subtract, Modifiers::NONE),
        (Combine::Intersect, Modifiers::NONE),
        (Combine::Subtract, Modifiers::ALT),
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = Document::new(128, 96).unwrap();
        app.current_mut().document.selection = Some(palette_core::editing::rectangle(
            128,
            96,
            palette_core::editing::PixelRect {
                x: 20,
                y: 20,
                width: 60,
                height: 55,
            },
        ));
        app.choose_tool(Tool::SelectBrush);
        app.current_mut().selection_mode = if mods.alt { Combine::Add } else { mode };
        app.current_mut().selection_brush.edge_aware = false;
        app.current_mut().selection_brush.strength = 0.5;
        app.current_mut().selection_feather = 1.5;
        frame(&mut app, &ctx, vec![]);
        let before = app.current().document.clone();
        let start = image_position(&app, [55.5, 45.5]);
        let mut press = pointer(start, true);
        if let Event::PointerButton { modifiers, .. } = &mut press {
            *modifiers = mods;
        }
        frame(
            &mut app,
            &ctx,
            vec![
                Event::ModifiersChanged(mods),
                Event::PointerMoved(start),
                press,
            ],
        );
        let draft = app.current().selection_brush_draft.as_ref().unwrap();
        assert_eq!(draft.mode, mode);
        let raw = selection::brush::from_points(&before, &draft.points, draft.settings).unwrap();
        let mut expected = before.clone();
        selection::apply_region(&mut expected, &raw, mode, 1.5).unwrap();
        frame(&mut app, &ctx, vec![pointer(start, false)]);
        wait_for_edit(&mut app, &ctx);
        assert!(app.current().document == expected);
        app.undo_current();
        assert!(app.current().document == before);
    }
}

#[test]
fn animated_and_multipage_import_notices_reach_open_tabs_and_imported_layers() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    for (filename, notice) in [
        ("two-frame.webp", "first WebP frame"),
        ("default-image-apng.png", "PNG default image"),
        ("two-page.tiff", "first TIFF page"),
    ] {
        let path = PathBuf::from("crates/palette-core/tests/fixtures").join(filename);
        app.open_path(path.clone(), &ctx);
        wait_for_files(&mut app, &ctx);
        assert!(app.message.contains(notice), "{}", app.message);
        assert_eq!(
            app.current().document.active().source.get(0, 0),
            [255, 0, 0, 255]
        );
        app.import_image(Some(path), &ctx);
        wait_for_files(&mut app, &ctx);
        assert!(
            app.message.contains(notice) && app.message.contains("layer"),
            "{}",
            app.message
        );
        assert_eq!(app.current().document.layers.len(), 2);
    }
}

#[test]
fn empty_brush_coverage_finishes_and_remains_undoable_without_panicking_the_worker() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(80, 60).unwrap();
    app.choose_tool(Tool::SelectBrush);
    app.current_mut().selection_brush.strength = 0.;
    let before = app.current().document.clone();
    for mode in [
        palette_core::selection::Combine::Subtract,
        palette_core::selection::Combine::Intersect,
    ] {
        app.begin_brush_selection([30., 30.], mode);
        assert!(app.current().selection_brush_draft.is_none());
        assert!(app.current().document == before);
        assert_eq!(app.current().history.revision, 0);
    }
    app.begin_brush_selection([30., 30.], palette_core::selection::Combine::Replace);
    app.current_mut()
        .selection_brush_draft
        .as_mut()
        .unwrap()
        .released = true;
    wait_for_edit(&mut app, &ctx);
    assert!(app.current_mut().selection_bounds().is_none());
    assert!(app.current_mut().selection_contours().is_empty());
    assert!(
        app.current()
            .document
            .selection
            .as_ref()
            .unwrap()
            .values
            .iter()
            .all(|v| *v == 0)
    );
    assert!(app.current().document.layers == before.layers);
    app.undo_current();
    assert!(app.current().document == before);
}

fn processing_document() -> Document {
    let mut doc = Document::from_raster(Raster::new(180, 120, [0; 4]).unwrap());
    for y in 35..85 {
        for x in 45..120 {
            doc.active_mut().source.set(x, y, [220, 70, 30, 255]);
        }
    }
    doc
}

#[test]
fn pixelate_controls_preview_cancel_apply_persist_undo_and_remove_with_saved_colors_and_choices() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::ColorHalftone,
        Kind::Crystallize,
        Kind::Facet,
        Kind::Mezzotint,
        Kind::Pointillize,
        Kind::StainedGlass,
        Kind::MosaicTiles,
        Kind::Diffuse,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = if kind == Kind::Pointillize {
            Color32::from_rgba_unmultiplied(240, 220, 180, 128)
        } else {
            Color32::from_rgb(240, 220, 180)
        };
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        if matches!(kind, Kind::Mezzotint | Kind::Diffuse) {
            let current = label_position(
                &mut app,
                &ctx,
                if kind == Kind::Mezzotint {
                    "Fine Dots"
                } else {
                    "Normal"
                },
            );
            click_ui(&mut app, &ctx, current);
            let choice = label_position(
                &mut app,
                &ctx,
                if kind == Kind::Mezzotint {
                    "Coarse Dots"
                } else {
                    "Anisotropic"
                },
            );
            click_ui(&mut app, &ctx, choice);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog.as_ref().unwrap().operation.value(
                    if kind == Kind::Mezzotint {
                        "pattern"
                    } else {
                        "mode"
                    }
                ),
                Some(Value::Choice(3))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if matches!(kind, Kind::Pointillize | Kind::StainedGlass) {
            assert_eq!(
                changed.value("color"),
                Some(Value::Color(if kind == Kind::Pointillize {
                    canvas_color
                } else {
                    [20, 40, 80]
                }))
            );
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pixelate.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn adjustment_controls_preview_cancel_apply_persist_undo_and_remove_with_saved_gradient_colors() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::ColorBalance,
        Kind::PhotoFilter,
        Kind::ChannelMixer,
        Kind::GradientMap,
        Kind::SelectiveColor,
        Kind::ShadowsHighlights,
        Kind::Blur,
        Kind::BlurMore,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = if kind == Kind::GradientMap {
            Color32::from_rgba_unmultiplied(240, 220, 180, 128)
        } else {
            Color32::from_rgb(240, 220, 180)
        };
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if kind == Kind::GradientMap {
            assert_eq!(changed.value("low"), Some(Value::Color([20, 40, 80])));
            assert_eq!(changed.value("high"), Some(Value::Color(canvas_color)));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adjustments.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn painting_filters_preview_cancel_apply_save_and_remove_with_paper_color_and_daub_choices() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::ColoredPencil,
        Kind::DryBrush,
        Kind::PaintDaubs,
        Kind::SmudgeStick,
        Kind::AngledStrokes,
        Kind::Crosshatch,
        Kind::DarkStrokes,
        Kind::Spatter,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = if kind == Kind::ColoredPencil {
            Color32::from_rgba_unmultiplied(240, 220, 180, 128)
        } else {
            Color32::from_rgb(240, 220, 180)
        };
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        if kind == Kind::PaintDaubs {
            let current = kind.descriptor().parameters
                [app.processing_dialog.as_ref().unwrap().parameter]
                .label;
            let selected = label_position(&mut app, &ctx, current);
            click_ui(&mut app, &ctx, selected);
            let brush_type = label_position(&mut app, &ctx, "Brush type");
            click_ui(&mut app, &ctx, brush_type);
            let round = label_position(&mut app, &ctx, "Round");
            click_ui(&mut app, &ctx, round);
            let diagonal = label_position(&mut app, &ctx, "Diagonal");
            click_ui(&mut app, &ctx, diagonal);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value("style"),
                Some(Value::Choice(2))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if kind == Kind::ColoredPencil {
            assert_eq!(changed.value("paper"), Some(Value::Color(canvas_color)));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("painting.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn drawing_filters_preview_cancel_save_remove_and_retain_colors_and_directions() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::SprayedStrokes,
        Kind::SumiE,
        Kind::BasRelief,
        Kind::ChalkCharcoal,
        Kind::Charcoal,
        Kind::GraphicPen,
        Kind::NotePaper,
        Kind::Stamp,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = if kind.descriptor().category == "Sketch" {
            Color32::from_rgba_unmultiplied(240, 220, 180, 128)
        } else {
            Color32::from_rgb(240, 220, 180)
        };
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        if matches!(kind, Kind::SprayedStrokes | Kind::GraphicPen) {
            let current = kind.descriptor().parameters
                [app.processing_dialog.as_ref().unwrap().parameter]
                .label;
            let selected = label_position(&mut app, &ctx, current);
            click_ui(&mut app, &ctx, selected);
            let brush_type = label_position(&mut app, &ctx, "Stroke direction");
            click_ui(&mut app, &ctx, brush_type);
            let round = label_position(&mut app, &ctx, "Right diagonal");
            click_ui(&mut app, &ctx, round);
            let diagonal = label_position(&mut app, &ctx, "Vertical");
            click_ui(&mut app, &ctx, diagonal);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value("direction"),
                Some(Value::Choice(3))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if kind.descriptor().category == "Sketch" {
            assert_eq!(changed.value("low"), Some(Value::Color([20, 40, 80])));
            assert_eq!(changed.value("high"), Some(Value::Color(canvas_color)));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("drawing.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn sketch_and_artistic_completion_filters_preview_save_undo_remove_and_preserve_texture_and_colors()
{
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::Chrome,
        Kind::ConteCrayon,
        Kind::Plaster,
        Kind::Reticulation,
        Kind::TornEdges,
        Kind::WaterPaper,
        Kind::Fresco,
        Kind::NeonGlow,
        Kind::PaletteKnife,
        Kind::PlasticWrap,
        Kind::RoughPastels,
        Kind::Sponge,
        Kind::Underpainting,
        Kind::Watercolor,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = if matches!(
            kind,
            Kind::NeonGlow
                | Kind::ConteCrayon
                | Kind::Plaster
                | Kind::Reticulation
                | Kind::TornEdges
        ) {
            Color32::from_rgba_unmultiplied(240, 220, 180, 128)
        } else {
            Color32::from_rgb(240, 220, 180)
        };
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        if matches!(
            kind,
            Kind::ConteCrayon | Kind::RoughPastels | Kind::Underpainting
        ) {
            let current = kind.descriptor().parameters
                [app.processing_dialog.as_ref().unwrap().parameter]
                .label;
            let selected = label_position(&mut app, &ctx, current);
            click_ui(&mut app, &ctx, selected);
            let texture = label_position(&mut app, &ctx, "Texture");
            click_ui(&mut app, &ctx, texture);
            let canvas = label_position(&mut app, &ctx, "Canvas");
            click_ui(&mut app, &ctx, canvas);
            let sandstone = label_position(&mut app, &ctx, "Sandstone");
            click_ui(&mut app, &ctx, sandstone);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value("texture"),
                Some(Value::Choice(3))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if matches!(
            kind,
            Kind::NeonGlow
                | Kind::ConteCrayon
                | Kind::Plaster
                | Kind::Reticulation
                | Kind::TornEdges
        ) {
            assert_eq!(changed.value("low"), Some(Value::Color([20, 40, 80])));
            assert_eq!(changed.value("high"), Some(Value::Color(canvas_color)));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sketch.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn optical_filters_preview_save_undo_remove_and_preserve_materials_modes_and_glow_color() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::DiffuseGlow,
        Kind::Glass,
        Kind::OceanRipple,
        Kind::Texturizer,
        Kind::ShapeBlur,
        Kind::SmartBlur,
        Kind::SharpenEdges,
        Kind::SmartSharpen,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = Color32::from_rgba_unmultiplied(240, 220, 180, 128);
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        let choice = match kind {
            Kind::Glass => Some(("Texture", "Frosted", "Canvas", "texture", 3)),
            Kind::Texturizer => Some(("Texture", "Canvas", "Sandstone", "texture", 3)),
            Kind::SmartBlur => Some(("Mode", "Normal", "Overlay edge", "mode", 2)),
            Kind::ShapeBlur => Some(("Shape", "Hexagon", "Octagon", "shape", 4)),
            _ => None,
        };
        if let Some((label, old, new, key, value)) = choice {
            let current = kind.descriptor().parameters
                [app.processing_dialog.as_ref().unwrap().parameter]
                .label;
            let selected = label_position(&mut app, &ctx, current);
            click_ui(&mut app, &ctx, selected);
            let param = label_position(&mut app, &ctx, label);
            click_ui(&mut app, &ctx, param);
            let selected = label_position(&mut app, &ctx, old);
            click_ui(&mut app, &ctx, selected);
            let option = label_position(&mut app, &ctx, new);
            click_ui(&mut app, &ctx, option);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog.as_ref().unwrap().operation.value(key),
                Some(Value::Choice(value))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if kind == Kind::DiffuseGlow {
            assert_eq!(changed.value("color"), Some(Value::Color(canvas_color)));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("optical.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn editable_catalog_filters_preview_apply_save_undo_remove_and_preserve_sources() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::FieldBlur,
        Kind::IrisBlur,
        Kind::TiltShift,
        Kind::PathBlur,
        Kind::SpinBlur,
        Kind::Flame,
        Kind::PictureFrame,
        Kind::Tree,
        Kind::NtscColors,
        Kind::Deinterlace,
        Kind::HsbHsl,
        Kind::ColorLookup,
        Kind::Displace,
        Kind::Shear,
        Kind::Custom,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = Color32::from_rgba_unmultiplied(240, 220, 180, 128);
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        let choice = match kind {
            Kind::FieldBlur => Some(("Selected pin", "Pin 2", "Pin 3", "pin", 2)),
            Kind::PathBlur => Some(("Path handle", "Bend", "End", "point", 2)),
            Kind::PictureFrame => Some(("Style", "Wood", "Ornate", "style", 3)),
            Kind::Tree => Some(("Tree form", "Deciduous", "Conifer", "form", 1)),
            Kind::Deinterlace => Some((
                "Remove field",
                "Odd (rows 1, 3, …)",
                "Even (rows 2, 4, …)",
                "field",
                1,
            )),
            Kind::HsbHsl => Some(("Output channels", "HSL", "HSB", "output", 1)),
            Kind::Displace => Some(("Map fitting", "Stretch", "Tile", "fit", 1)),
            Kind::Shear => Some(("Interpolation", "Smooth", "Linear", "interpolation", 0)),
            Kind::Custom => Some(("Edges", "Repeat", "Wrap", "edges", 0)),
            _ => None,
        };
        if let Some((label, old, new, key, value)) = choice {
            let current = kind.descriptor().parameters
                [app.processing_dialog.as_ref().unwrap().parameter]
                .label;
            if current != label {
                let selected = label_position(&mut app, &ctx, current);
                click_ui(&mut app, &ctx, selected);
                let param = label_position(&mut app, &ctx, label);
                click_ui(&mut app, &ctx, param);
            }
            let selected = label_position(&mut app, &ctx, old);
            click_ui(&mut app, &ctx, selected);
            let option = label_position(&mut app, &ctx, new);
            click_ui(&mut app, &ctx, option);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog.as_ref().unwrap().operation.value(key),
                Some(Value::Choice(value))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        if kind == Kind::NtscColors {
            let toggle = scoped_label_bounds(&mut app, &ctx, "7.5 IRE", true).center();
            click_ui(&mut app, &ctx, toggle);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value("setup"),
                Some(Value::Boolean(true))
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("editable.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn stylize_render_filters_preview_save_undo_remove_and_preserve_choices_and_tile_colors() {
    use palette_core::processing::{Kind, ParameterType, Value};
    for kind in [
        Kind::Extrude,
        Kind::OilPaint,
        Kind::Tiles,
        Kind::TraceContour,
        Kind::Wind,
        Kind::LensBlur,
        Kind::LensFlare,
        Kind::LightingEffects,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = Color32::from_rgba_unmultiplied(240, 220, 180, 128);
        let [r, g, b, _] = app.background.to_srgba_unmultiplied();
        let canvas_color = [r, g, b];
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        let choice = match kind {
            Kind::Extrude => Some(("Shape", "Blocks", "Pyramids", "shape", 1)),
            Kind::Tiles => Some(("Fill gaps", "Unaltered", "Background", "fill", 3)),
            Kind::Wind => Some(("Method", "Wind", "Blast", "method", 1)),
            Kind::LensBlur => Some(("Depth source", "Uniform", "Luminance", "depth_source", 2)),
            Kind::LensFlare => Some(("Lens type", "Zoom", "Anamorphic", "style", 3)),
            Kind::LightingEffects => Some(("Light type", "Point", "Spot", "kind", 1)),
            _ => None,
        };
        if let Some((label, old, new, key, value)) = choice {
            let current = kind.descriptor().parameters
                [app.processing_dialog.as_ref().unwrap().parameter]
                .label;
            let selected = label_position(&mut app, &ctx, current);
            click_ui(&mut app, &ctx, selected);
            let param = label_position(&mut app, &ctx, label);
            click_ui(&mut app, &ctx, param);
            let selected = if kind == Kind::Wind {
                // The operation title and selected method both say Wind. Pick
                // the value widget on the right, not the noninteractive title.
                fn labels(shape: &egui::epaint::Shape, out: &mut Vec<egui::Pos2>) {
                    match shape {
                        egui::epaint::Shape::Text(t) if t.galley.text() == "Wind" => {
                            out.push(t.galley.rect.translate(t.pos.to_vec2()).center())
                        }
                        egui::epaint::Shape::Vec(v) => {
                            for s in v {
                                labels(s, out);
                            }
                        }
                        _ => {}
                    }
                }
                let mut positions = vec![];
                let mut output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
                        ..Default::default()
                    },
                    |ui| app.draw(ui),
                );
                for shape in &output.shapes {
                    labels(&shape.shape, &mut positions);
                }
                output.textures_delta.clear();
                positions
                    .into_iter()
                    .max_by(|a, b| a.x.total_cmp(&b.x))
                    .unwrap()
            } else {
                label_position(&mut app, &ctx, old)
            };
            click_ui(&mut app, &ctx, selected);
            let option = label_position(&mut app, &ctx, new);
            click_ui(&mut app, &ctx, option);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog.as_ref().unwrap().operation.value(key),
                Some(Value::Choice(value))
            );
        }
        if let Some((index, p)) = kind
            .descriptor()
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| matches!(p.kind, ParameterType::Number { .. }))
        {
            let selected = app.processing_dialog.as_ref().unwrap().parameter;
            if selected != index {
                let selected =
                    label_position(&mut app, &ctx, kind.descriptor().parameters[selected].label);
                click_ui(&mut app, &ctx, selected);
                let parameter = label_position(&mut app, &ctx, p.label);
                click_ui(&mut app, &ctx, parameter);
            }
            let setting = scoped_label_bounds(&mut app, &ctx, p.label, true);
            click_ui(
                &mut app,
                &ctx,
                pos2(setting.right() + 95., setting.center().y),
            );
            wait_for_processing(&mut app, &ctx);
            assert_ne!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value(p.key),
                before.value(p.key),
                "{kind:?}"
            );
        }
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if kind == Kind::Tiles {
            assert_eq!(changed.value("low"), Some(Value::Color([20, 40, 80])));
            assert_eq!(changed.value("high"), Some(Value::Color(canvas_color)));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stylize.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[test]
fn artistic_controls_preview_cancel_then_save_undo_and_remove_each_editable_filter() {
    use palette_core::processing::{Kind, Value};
    for kind in [
        Kind::Cutout,
        Kind::FilmGrain,
        Kind::AccentedEdges,
        Kind::InkOutlines,
        Kind::Photocopy,
        Kind::HalftonePattern,
        Kind::Patchwork,
        Kind::Craquelure,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.brush.color = [20, 40, 80, 255];
        app.background = Color32::from_rgb(240, 220, 180);
        let original = app.current().document.clone();
        let parameter = kind.descriptor().parameters[0];
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        let setting = scoped_label_bounds(&mut app, &ctx, parameter.label, true);
        click_ui(
            &mut app,
            &ctx,
            pos2(setting.right() + 95., setting.center().y),
        );
        wait_for_processing(&mut app, &ctx);
        let changed = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert_ne!(
            changed.value(parameter.key),
            before.value(parameter.key),
            "{kind:?}"
        );
        assert!(app.current().document == original);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let cancel = label_position(&mut app, &ctx, "Cancel");
        click_ui(&mut app, &ctx, cancel);
        assert!(app.current().document == original);
        app.open_processing(kind, None);
        // Reuse the accepted draft value; Apply/Cancel, menu routing and field
        // input above are exercised through real egui events.
        app.processing_dialog.as_mut().unwrap().operation = changed.clone();
        wait_for_processing(&mut app, &ctx);
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        let saved = app.current().document.clone();
        let id = saved.active().filters[0].id;
        assert!(saved.active().source == original.active().source);
        if matches!(kind, Kind::Photocopy | Kind::HalftonePattern) {
            assert_eq!(changed.value("low"), Some(Value::Color([20, 40, 80])));
            assert_eq!(changed.value("high"), Some(Value::Color([240, 220, 180])));
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artistic.palette");
        io::save_project(&path, &saved).unwrap();
        assert!(io::open(&path).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        app.brush.color = [255, 0, 0, 255];
        app.background = Color32::BLACK;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, changed);
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        let remove = label_position(&mut app, &ctx, "Remove");
        click_ui(&mut app, &ctx, remove);
        assert!(app.current().document.active().filters.is_empty());
        assert!(app.current().document.active().source == original.active().source);
        app.undo_current();
        assert!(app.current().document == saved);
    }
}

#[track_caller]
fn wait_for_processing(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        frame(app, ctx, vec![]);
        assert!(app.error.is_none(), "{:?}", app.error);
        let d = app.processing_dialog.as_ref().expect("processing dialog");
        if d.ready == Some(d.ticket) && d.import.is_none() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "processing preview timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn imported_filter_assets_preview_stay_embedded_and_reject_failed_or_stale_jobs() {
    use palette_core::processing::{Kind, Operation};
    let dir = tempfile::tempdir().unwrap();
    let cube = dir.path().join("swap.cube");
    let mut text = "LUT_3D_SIZE 2\n".to_owned();
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                text += &format!("{b} {g} {r}\n");
            }
        }
    }
    std::fs::write(&cube, text).unwrap();
    let map_path = dir.path().join("map.png");
    io::export(
        &map_path,
        &Document::from_raster(Raster::new(3, 2, [192, 128, 0, 255]).unwrap()),
    )
    .unwrap();
    for (kind, path, label) in [
        (Kind::ColorLookup, cube.clone(), "Load 3D LUT…"),
        (Kind::Displace, map_path.clone(), "Load displacement map…"),
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        let original = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let before = app.processing_dialog.as_ref().unwrap().operation.clone();
        let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
        click_ui(&mut app, &ctx, actions);
        assert!(label_bounds(&mut app, &ctx, label).is_positive());
        frame(
            &mut app,
            &ctx,
            vec![Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default(),
            }],
        );
        app.begin_processing_asset_import(path.clone(), ctx.clone());
        app.apply_processing();
        assert!(app.processing_dialog.is_some());
        assert!(app.current().document == original);
        wait_for_processing(&mut app, &ctx);
        let imported = app.processing_dialog.as_ref().unwrap().operation.clone();
        assert_ne!(before, imported);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        assert!(app.current().document == original);
        app.apply_processing();
        let saved = app.current().document.clone();
        let project = dir.path().join(format!("{kind:?}.palette"));
        io::save_project(&project, &saved).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(io::open(&project).unwrap() == saved);
        app.undo_current();
        assert!(app.current().document == original);
        app.redo_current();
        assert!(app.current().document == saved);
        let id = saved.active().filters[0].id;
        app.open_processing(kind, Some(id));
        wait_for_processing(&mut app, &ctx);
        assert_eq!(app.processing_dialog.as_ref().unwrap().operation, imported);
        // A new draft owns a new result channel; an abandoned job cannot write it.
        app.begin_processing_asset_import(dir.path().join("missing.cube"), ctx.clone());
        app.cancel_processing();
        app.open_processing(Kind::Custom, None);
        for _ in 0..10 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(app.error.is_none());
        assert!(matches!(
            app.processing_dialog.as_ref().unwrap().operation,
            Operation::Filter(Filter::Custom { .. })
        ));
    }
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.open_processing(Kind::ColorLookup, None);
    wait_for_processing(&mut app, &ctx);
    let before = app.processing_dialog.as_ref().unwrap().operation.clone();
    let bad = dir.path().join("invalid.cube");
    std::fs::write(&bad, "LUT_3D_SIZE 99").unwrap();
    app.begin_processing_asset_import(bad, ctx.clone());
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while app.error.is_none() {
        frame(&mut app, &ctx, vec![]);
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(app.processing_dialog.as_ref().unwrap().operation, before);
}

#[test]
fn kernel_popup_and_mouse_shear_curve_edit_live_without_committing_source_pixels() {
    use palette_core::processing::{Kind, Value};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let original = app.current().document.clone();
    app.open_processing(Kind::Custom, None);
    wait_for_processing(&mut app, &ctx);
    let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
    click_ui(&mut app, &ctx, actions);
    let edit = label_position(&mut app, &ctx, "Edit kernel…");
    click_ui(&mut app, &ctx, edit);
    let sharpen = label_position(&mut app, &ctx, "Sharpen");
    click_ui(&mut app, &ctx, sharpen);
    wait_for_processing(&mut app, &ctx);
    assert_eq!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("k12"),
        Some(Value::Number(5.))
    );
    assert_eq!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("k11"),
        Some(Value::Number(-1.))
    );
    assert!(app.current().document == original);
    app.cancel_processing();
    egui::Popup::close_all(&ctx);
    app.open_processing(Kind::Shear, None);
    wait_for_processing(&mut app, &ctx);
    let actions = scoped_label_bounds(&mut app, &ctx, "…", true).center();
    click_ui(&mut app, &ctx, actions);
    let edit = label_position(&mut app, &ctx, "Edit shear curve…");
    click_ui(&mut app, &ctx, edit);
    let _ = label_bounds(&mut app, &ctx, "Drag the five curve points horizontally");
    fn handles(shape: &egui::epaint::Shape, out: &mut Vec<egui::Pos2>) {
        match shape {
            egui::epaint::Shape::Circle(c)
                if c.radius == 4.
                    && c.fill.a() > 0
                    && c.fill.r() == c.fill.g()
                    && c.fill.g() == c.fill.b() =>
            {
                out.push(c.center)
            }
            egui::epaint::Shape::Vec(v) => {
                for shape in v {
                    handles(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    let mut points = vec![];
    for shape in &output.shapes {
        handles(&shape.shape, &mut points);
    }
    output.textures_delta.clear();
    points.sort_by(|a, b| a.y.total_cmp(&b.y));
    assert_eq!(points.len(), 5);
    let start = points[2];
    let end = start + vec2(35., 0.);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    wait_for_processing(&mut app, &ctx);
    let Some(Value::Number(value)) = app
        .processing_dialog
        .as_ref()
        .unwrap()
        .operation
        .value("curve2")
    else {
        panic!("curve")
    };
    assert!(value > 0.2);
    assert!(app.current().document == original);
    app.cancel_processing();
    assert!(app.current().document == original);
}
#[test]
fn gaussian_dialog_previews_real_parameter_input_cancels_and_applies_one_native_undoable_filter() {
    use palette_core::processing::Kind;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let before = app.current().document.clone();
    app.open_processing(Kind::GaussianBlur, None);
    wait_for_processing(&mut app, &ctx);
    assert!(app.processing_dialog.as_ref().unwrap().image.is_some());
    let radius = label_bounds(&mut app, &ctx, "Radius (px)");
    let slider = pos2(radius.right() + 90., radius.center().y);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(slider), pointer(slider, true)],
    );
    frame(&mut app, &ctx, vec![pointer(slider, false)]);
    wait_for_processing(&mut app, &ctx);
    assert!(
        app.processing_dialog.as_ref().unwrap().operation != Kind::GaussianBlur.default_operation()
    );
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, 0);
    let cancel = label_position(&mut app, &ctx, "Cancel");
    click_ui(&mut app, &ctx, cancel);
    assert!(app.processing_dialog.is_none());
    assert!(app.current().document == before);
    app.open_processing(Kind::GaussianBlur, None);
    wait_for_processing(&mut app, &ctx);
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    assert!(app.processing_dialog.is_none());
    assert_eq!(app.current().document.active().filters.len(), 1);
    assert_eq!(
        app.current().document.active().source,
        before.active().source
    );
    wait_for_render(&mut app, &ctx);
    assert!(app.current().rendered.as_ref().unwrap().get(44, 55)[3] > 0);
    let saved = app.current().document.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blur.palette");
    io::save_project(&path, &saved).unwrap();
    assert!(io::open(&path).unwrap() == saved);
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    assert!(app.current().document == saved);
}
#[test]
fn processing_stacks_support_masking_reordering_duplicate_toggle_and_effect_editing() {
    use super::processing::{Action, MaskAction};
    use palette_core::processing::Kind;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let source = app.current().document.active().source.clone();
    app.open_processing(Kind::GaussianBlur, None);
    app.apply_processing();
    let blur = app.current().document.active().filters[0].id;
    app.add_filter(Filter::Brightness { amount: 0.5 });
    app.processing_action(false, Action::Move(blur, 1));
    assert_eq!(app.current().document.active().filters[1].id, blur);
    app.processing_action(false, Action::Duplicate(blur));
    assert_eq!(app.current().document.active().filters.len(), 3);
    let duplicate = app.current().document.active().filters[2].id;
    assert_ne!(blur, duplicate);
    app.processing_action(false, Action::Toggle(blur, false));
    assert!(!app.current().document.active().filters[1].enabled);
    app.select_all();
    app.processing_action(false, Action::Mask(blur, MaskAction::FromSelection));
    assert!(app.current().document.active().filters[1].mask.is_some());
    assert!(
        app.current().document.active().mask.is_none(),
        "filter-mask creation cannot replace the layer mask"
    );
    app.processing_action(false, Action::Mask(blur, MaskAction::Toggle));
    assert!(
        !app.current().document.active().filters[1]
            .mask
            .as_ref()
            .unwrap()
            .enabled
    );
    app.processing_action(false, Action::Mask(blur, MaskAction::Remove));
    assert!(app.current().document.active().filters[1].mask.is_none());
    app.processing_action(false, Action::Delete(duplicate));
    assert_eq!(app.current().document.active().filters.len(), 2);
    for kind in [Kind::DropShadow, Kind::OuterGlow] {
        app.open_processing(kind, None);
        app.apply_processing();
    }
    let shadow = app.current().document.active().effects[0].id;
    app.open_processing(Kind::DropShadow, Some(shadow));
    wait_for_processing(&mut app, &ctx);
    app.cancel_processing();
    assert_eq!(app.current().document.active().effects[0].id, shadow);
    app.processing_action(true, Action::Duplicate(shadow));
    assert_eq!(app.current().document.active().effects.len(), 3);
    app.processing_action(true, Action::Move(shadow, 1));
    assert_eq!(app.current().document.active().effects[1].id, shadow);
    app.processing_action(true, Action::Toggle(shadow, false));
    assert!(!app.current().document.active().effects[1].enabled);
    assert_eq!(app.current().document.active().source, source);
    app.current().document.validate().unwrap();
}
#[test]
fn processing_previews_fence_cancelled_jobs_and_tab_changes_without_mutating_documents() {
    use palette_core::processing::Kind;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let before = app.current().document.clone();
    app.open_processing(Kind::DropShadow, None);
    frame(&mut app, &ctx, vec![]);
    let old = app.processing_dialog.as_ref().unwrap().ticket;
    app.cancel_processing();
    app.open_processing(Kind::OuterGlow, None);
    wait_for_processing(&mut app, &ctx);
    assert!(app.processing_dialog.as_ref().unwrap().ticket > old);
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, 0);
    app.new_document();
    frame(&mut app, &ctx, vec![]);
    assert!(app.processing_dialog.is_none());
    assert!(app.sessions[0].document == before);
    assert!(app.current().document.active().effects.is_empty());
}
#[test]
fn painting_with_spatial_filters_and_live_styles_uses_background_updates_and_preserves_source() {
    use palette_core::processing::Kind;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    app.current_mut().zoom = 1.25;
    for kind in [Kind::GaussianBlur, Kind::DropShadow, Kind::OuterGlow] {
        app.open_processing(kind, None);
        app.apply_processing();
    }
    wait_for_render(&mut app, &ctx);
    let before = app.current().document.clone();
    app.choose_tool(Tool::Brush);
    app.brush.diameter = 12.;
    app.brush.color = [30, 80, 200, 255];
    let start = image_position(&app, [60., 20.]);
    let end = image_position(&app, [100., 20.]);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
    wait_for_render(&mut app, &ctx);
    assert!(app.current().document.active().source.get(80, 20)[3] > 0);
    assert!(
        app.current().rendered.as_ref().unwrap().get(80, 15)[3] > 0,
        "blur/glow updates before mouse release"
    );
    assert_eq!(
        app.current().document.active().filters,
        before.active().filters
    );
    assert_eq!(
        app.current().document.active().effects,
        before.active().effects
    );
    frame(&mut app, &ctx, vec![pointer(end, false)]);
    app.undo_current();
    assert!(app.current().document == before);
}

/// Check the actual canvas mesh, rather than only the preview handle existing.
fn canvas_uses_texture(app: &mut PaletteApp, ctx: &Context, texture: egui::TextureId) -> bool {
    fn contains(shape: &egui::epaint::Shape, texture: egui::TextureId, bounds: Rect) -> bool {
        match shape {
            egui::epaint::Shape::Mesh(mesh) => {
                mesh.texture_id == texture && mesh.calc_bounds() == bounds
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().any(|s| contains(s, texture, bounds)),
            _ => false,
        }
    }
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    let found = output
        .shapes
        .iter()
        .any(|s| contains(&s.shape, texture, app.image_rect));
    output.textures_delta.clear();
    found
}

#[test]
fn layer_icon_row_opens_canvas_processing_and_live_preview_never_mutates_source() {
    use super::processing::Section;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    app.interface.layer_properties = true;
    wait_for_render(&mut app, &ctx);
    let before = app.current().document.clone();
    let buttons = app.layer_processing_buttons;
    assert!(buttons.iter().all(|r| r.is_positive()));
    assert!(
        buttons
            .windows(2)
            .all(|r| r[0].right() <= r[1].left() && r[0].top() == r[1].top())
    );
    let opacity = label_bounds(&mut app, &ctx, "Opacity");
    assert!(buttons[0].top() > opacity.bottom());
    click_ui(&mut app, &ctx, buttons[0].center());
    assert!(app.processing_section_is(Section::Filters));
    let add = label_position(&mut app, &ctx, "Add");
    click_ui(&mut app, &ctx, add);
    let blur = label_position(&mut app, &ctx, "Gaussian Blur");
    click_ui(&mut app, &ctx, blur);
    wait_for_processing(&mut app, &ctx);
    let preview = app.processing_preview().unwrap().id();
    assert!(canvas_uses_texture(&mut app, &ctx, preview));
    assert!(app.current().document == before);
    assert_eq!(app.current().history.revision, 0);
    let preview_toggle = label_position(&mut app, &ctx, "Preview");
    click_ui(&mut app, &ctx, preview_toggle);
    assert!(app.processing_preview().is_none());
    let committed = app.current().texture.as_ref().unwrap().id();
    assert!(canvas_uses_texture(&mut app, &ctx, committed));
    let preview_toggle = label_position(&mut app, &ctx, "Preview");
    click_ui(&mut app, &ctx, preview_toggle);
    wait_for_processing(&mut app, &ctx);
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    assert_eq!(app.current().document.active().filters.len(), 1);
    assert!(app.current().document.active().source == before.active().source);
    let more = label_position(&mut app, &ctx, "…");
    click_ui(&mut app, &ctx, more);
    let remove = label_position(&mut app, &ctx, "Remove");
    click_ui(&mut app, &ctx, remove);
    assert!(app.current().document.active().filters.is_empty());
    assert!(app.current().document.layers == before.layers);
    app.undo_current();
    assert_eq!(app.current().document.active().filters.len(), 1);
    let effects = app.layer_processing_buttons[1].center();
    click_ui(&mut app, &ctx, effects);
    assert!(app.processing_section_is(Section::Effects));
}

#[test]
fn effects_are_editable_and_removable_from_the_canvas_toolbar_with_undo_and_native_preservation() {
    use palette_core::processing::Kind;
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let source = app.current().document.active().source.clone();
    app.open_processing(Kind::DropShadow, None);
    wait_for_processing(&mut app, &ctx);
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    let saved = app.current().document.clone();
    let edit = scoped_label_bounds(&mut app, &ctx, "Edit", true).center();
    click_ui(&mut app, &ctx, edit);
    wait_for_processing(&mut app, &ctx);
    let color = label_position(&mut app, &ctx, "Light angle (°)");
    click_ui(&mut app, &ctx, color);
    assert!(egui::Popup::is_any_open(&ctx));
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(!egui::Popup::is_any_open(&ctx));
    assert!(
        app.processing_dialog.is_some(),
        "Escape closes the parameter menu first"
    );
    let color = label_position(&mut app, &ctx, "Light angle (°)");
    click_ui(&mut app, &ctx, color);
    let size = label_position(&mut app, &ctx, "Size (px)");
    click_ui(&mut app, &ctx, size);
    let size = label_bounds(&mut app, &ctx, "Size (px)");
    let slider = pos2(size.right() + 100., size.center().y);
    click_ui(&mut app, &ctx, slider);
    wait_for_processing(&mut app, &ctx);
    assert!(app.current().document == saved);
    let preview = app.processing_preview().unwrap().id();
    assert!(canvas_uses_texture(&mut app, &ctx, preview));
    let cancel = label_position(&mut app, &ctx, "Cancel");
    click_ui(&mut app, &ctx, cancel);
    assert!(app.current().document == saved && app.processing_preview().is_none());
    let edit = scoped_label_bounds(&mut app, &ctx, "Edit", true).center();
    click_ui(&mut app, &ctx, edit);
    wait_for_processing(&mut app, &ctx);
    let more = label_position(&mut app, &ctx, "…");
    click_ui(&mut app, &ctx, more);
    let remove = label_position(&mut app, &ctx, "Remove");
    click_ui(&mut app, &ctx, remove);
    assert!(app.processing_dialog.is_none());
    assert!(app.current().document.active().effects.is_empty());
    assert!(app.current().document.active().source == source);
    app.undo_current();
    assert!(app.current().document == saved);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("live-styles.palette");
    io::save_project(&path, &app.current().document).unwrap();
    assert!(io::open(&path).unwrap() == saved);
}

#[test]
fn registry_filter_menu_opens_new_adjustments_and_noise_with_live_parameter_previews() {
    use palette_core::processing::{Kind, Operation};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let source = app.current().document.active().source.clone();
    let filter = label_position(&mut app, &ctx, "Filter");
    click_ui(&mut app, &ctx, filter);
    let adjustments = label_position(&mut app, &ctx, "Adjustments");
    click_ui(&mut app, &ctx, adjustments);
    let hue = label_position(&mut app, &ctx, "Hue/Saturation…");
    click_ui(&mut app, &ctx, hue);
    wait_for_processing(&mut app, &ctx);
    assert_eq!(
        app.processing_dialog.as_ref().unwrap().operation.kind(),
        Kind::HueSaturation
    );
    let hue = label_position(&mut app, &ctx, "Hue (°)");
    click_ui(&mut app, &ctx, hue);
    let saturation = label_position(&mut app, &ctx, "Saturation");
    click_ui(&mut app, &ctx, saturation);
    let saturation = scoped_label_bounds(&mut app, &ctx, "Saturation", true);
    click_ui(
        &mut app,
        &ctx,
        pos2(saturation.right() + 90., saturation.center().y),
    );
    wait_for_processing(&mut app, &ctx);
    assert!(matches!(app.processing_dialog.as_ref().unwrap().operation,
        Operation::Filter(Filter::HueSaturation { saturation, .. }) if saturation != 0.));
    assert!(app.current().document.active().filters.is_empty());
    let preview = app.processing_preview().unwrap().id();
    assert!(canvas_uses_texture(&mut app, &ctx, preview));
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    assert!(app.current().document.active().source == source);
    assert_eq!(app.current().document.active().filters.len(), 1);
    let filter = label_position(&mut app, &ctx, "Filter");
    click_ui(&mut app, &ctx, filter);
    let noise = label_position(&mut app, &ctx, "Noise");
    click_ui(&mut app, &ctx, noise);
    let median = label_position(&mut app, &ctx, "Median…");
    click_ui(&mut app, &ctx, median);
    wait_for_processing(&mut app, &ctx);
    assert_eq!(
        app.processing_dialog.as_ref().unwrap().operation.kind(),
        Kind::Median
    );
    let cancel = label_position(&mut app, &ctx, "Cancel");
    click_ui(&mut app, &ctx, cancel);
    assert_eq!(app.current().document.active().filters.len(), 1);
}

#[test]
fn stroke_placement_choice_previews_on_canvas_and_commits_as_an_editable_undoable_style() {
    use palette_core::{
        document::{Effect, StrokePosition},
        processing::Kind,
    };
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let before = app.current().document.clone();
    app.open_processing(Kind::Stroke, None);
    wait_for_processing(&mut app, &ctx);
    let color = label_position(&mut app, &ctx, "Width (px)");
    click_ui(&mut app, &ctx, color);
    let position = label_position(&mut app, &ctx, "Position");
    click_ui(&mut app, &ctx, position);
    let outside = label_position(&mut app, &ctx, "Outside");
    click_ui(&mut app, &ctx, outside);
    let center = label_position(&mut app, &ctx, "Center");
    click_ui(&mut app, &ctx, center);
    wait_for_processing(&mut app, &ctx);
    assert!(app.current().document == before);
    let preview = app.processing_preview().unwrap().id();
    assert!(canvas_uses_texture(&mut app, &ctx, preview));
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    assert!(matches!(
        app.current().document.active().effects[0].effect,
        Effect::Stroke {
            position: StrokePosition::Center,
            ..
        }
    ));
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    assert_eq!(app.current().document.active().effects.len(), 1);
    assert!(app.current().document.active().source == before.active().source);
}

#[test]
fn requested_style_controls_preview_live_and_commit_without_touching_source() {
    use palette_core::processing::{Kind, Value};
    for kind in [
        Kind::OuterGlow,
        Kind::InnerGlow,
        Kind::DropShadow,
        Kind::InnerShadow,
        Kind::Stroke,
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        let before = app.current().document.clone();
        app.open_processing(kind, None);
        wait_for_processing(&mut app, &ctx);
        let label = match kind {
            Kind::OuterGlow | Kind::InnerGlow => "Radius (px)",
            Kind::DropShadow | Kind::InnerShadow => "Light angle (°)",
            _ => "Width (px)",
        };
        let control = scoped_label_bounds(&mut app, &ctx, label, true);
        let old = app.processing_dialog.as_ref().unwrap().operation.clone();
        click_ui(
            &mut app,
            &ctx,
            pos2(control.right() + 75., control.center().y),
        );
        wait_for_processing(&mut app, &ctx);
        assert!(app.processing_dialog.as_ref().unwrap().operation != old);
        if kind == Kind::Stroke {
            let rounded = label_position(&mut app, &ctx, "Rounded corners");
            click_ui(&mut app, &ctx, rounded);
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value("rounded"),
                Some(Value::Boolean(false))
            );
        }
        if matches!(kind, Kind::DropShadow | Kind::InnerShadow) {
            for (key, label) in [("distance", "Distance"), ("spread", "Spread")] {
                let label = scoped_label_bounds(&mut app, &ctx, label, true);
                let start = pos2(label.right() + 15., label.center().y);
                let end = start + vec2(30., 0.);
                let old = app.processing_dialog.as_ref().unwrap().operation.value(key);
                frame(
                    &mut app,
                    &ctx,
                    vec![Event::PointerMoved(start), pointer(start, true)],
                );
                frame(&mut app, &ctx, vec![Event::PointerMoved(end)]);
                frame(&mut app, &ctx, vec![pointer(end, false)]);
                wait_for_processing(&mut app, &ctx);
                assert_ne!(
                    app.processing_dialog.as_ref().unwrap().operation.value(key),
                    old
                );
            }
        }
        assert!(app.current().document == before);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        assert!(app.canvas_rect.contains_rect(app.processing_toolbar_rect));
        let expected = app.processing_dialog.as_ref().unwrap().operation.clone();
        let apply = label_position(&mut app, &ctx, "Apply");
        click_ui(&mut app, &ctx, apply);
        assert!(
            palette_core::processing::Operation::Effect(
                app.current().document.active().effects[0].effect.clone()
            ) == expected
        );
        assert!(app.current().document.active().source == before.active().source);
        app.undo_current();
        assert!(app.current().document == before);
    }
}

#[test]
fn new_filter_menu_entries_open_live_editors_and_noise_boolean_is_editable() {
    use palette_core::processing::{Kind, Value};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    for kind in [
        Kind::Levels,
        Kind::Curves,
        Kind::MotionBlur,
        Kind::HighPass,
        Kind::Mosaic,
        Kind::AddNoise,
        Kind::Average,
        Kind::FindEdges,
        Kind::Solarize,
        Kind::Emboss,
        Kind::UnsharpMask,
        Kind::Minimum,
        Kind::Maximum,
        Kind::Offset,
        Kind::Twirl,
        Kind::Wave,
        Kind::SurfaceBlur,
        Kind::RadialBlur,
        Kind::Clouds,
        Kind::DifferenceClouds,
        Kind::PosterEdges,
        Kind::Grain,
        Kind::Fragment,
        Kind::Despeckle,
        Kind::DustScratches,
        Kind::ReduceNoise,
        Kind::Pinch,
        Kind::Spherize,
        Kind::Ripple,
        Kind::Sharpen,
        Kind::SharpenMore,
        Kind::PolarCoordinates,
        Kind::ZigZag,
        Kind::Fibers,
        Kind::GlowingEdges,
        Kind::Cutout,
        Kind::FilmGrain,
        Kind::AccentedEdges,
        Kind::InkOutlines,
        Kind::Photocopy,
        Kind::HalftonePattern,
        Kind::Patchwork,
        Kind::Craquelure,
        Kind::ColorHalftone,
        Kind::Crystallize,
        Kind::Facet,
        Kind::Mezzotint,
        Kind::Pointillize,
        Kind::StainedGlass,
        Kind::MosaicTiles,
        Kind::Diffuse,
        Kind::ColorBalance,
        Kind::PhotoFilter,
        Kind::ChannelMixer,
        Kind::GradientMap,
        Kind::SelectiveColor,
        Kind::ShadowsHighlights,
        Kind::Blur,
        Kind::BlurMore,
        Kind::ColoredPencil,
        Kind::DryBrush,
        Kind::PaintDaubs,
        Kind::SmudgeStick,
        Kind::AngledStrokes,
        Kind::Crosshatch,
        Kind::DarkStrokes,
        Kind::Spatter,
        Kind::SprayedStrokes,
        Kind::SumiE,
        Kind::BasRelief,
        Kind::ChalkCharcoal,
        Kind::Charcoal,
        Kind::GraphicPen,
        Kind::NotePaper,
        Kind::Stamp,
        Kind::Chrome,
        Kind::ConteCrayon,
        Kind::Plaster,
        Kind::Reticulation,
        Kind::TornEdges,
        Kind::WaterPaper,
        Kind::Fresco,
        Kind::NeonGlow,
        Kind::PaletteKnife,
        Kind::PlasticWrap,
        Kind::RoughPastels,
        Kind::Sponge,
        Kind::Underpainting,
        Kind::Watercolor,
        Kind::DiffuseGlow,
        Kind::Glass,
        Kind::OceanRipple,
        Kind::Texturizer,
        Kind::ShapeBlur,
        Kind::SmartBlur,
        Kind::SharpenEdges,
        Kind::SmartSharpen,
        Kind::Extrude,
        Kind::OilPaint,
        Kind::Tiles,
        Kind::TraceContour,
        Kind::Wind,
        Kind::LensBlur,
        Kind::LensFlare,
        Kind::LightingEffects,
        Kind::ColorLookup,
        Kind::Displace,
        Kind::Shear,
        Kind::Custom,
        Kind::FieldBlur,
        Kind::IrisBlur,
        Kind::TiltShift,
        Kind::PathBlur,
        Kind::SpinBlur,
        Kind::Flame,
        Kind::PictureFrame,
        Kind::Tree,
        Kind::NtscColors,
        Kind::Deinterlace,
        Kind::HsbHsl,
    ] {
        app.cancel_processing();
        let filter = label_position(&mut app, &ctx, "Filter");
        click_ui(&mut app, &ctx, filter);
        let category = label_position(&mut app, &ctx, kind.descriptor().category);
        click_ui(&mut app, &ctx, category);
        let item = rightmost_menu_label_position(
            &mut app,
            &ctx,
            &format!(
                "{}{}",
                kind.descriptor().label,
                if kind.descriptor().parameters.is_empty() {
                    ""
                } else {
                    "…"
                }
            ),
        );
        click_ui(&mut app, &ctx, item);
        wait_for_processing(&mut app, &ctx);
        assert_eq!(
            app.processing_dialog.as_ref().unwrap().operation.kind(),
            kind
        );
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        if kind == Kind::AddNoise {
            let amount = label_position(&mut app, &ctx, "Amount");
            click_ui(&mut app, &ctx, amount);
            let monochrome = label_position(&mut app, &ctx, "Monochrome");
            click_ui(&mut app, &ctx, monochrome);
            let monochrome = scoped_label_bounds(&mut app, &ctx, "Monochrome", true).center();
            // The parameter selector is the left occurrence; the checkbox is to its right.
            click_ui(&mut app, &ctx, monochrome + vec2(148., 0.));
            wait_for_processing(&mut app, &ctx);
            assert_eq!(
                app.processing_dialog
                    .as_ref()
                    .unwrap()
                    .operation
                    .value("monochrome"),
                Some(Value::Boolean(false))
            );
        }
        assert!(app.current().document.active().filters.is_empty());
    }
}

#[test]
fn filter_gallery_stays_first_and_disabled_after_catalog_completion() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let before = app.current().document.clone();
    let filter = label_position(&mut app, &ctx, "Filter");
    click_ui(&mut app, &ctx, filter);
    let gallery = label_bounds(&mut app, &ctx, "Filter Gallery…");
    let adjustments = label_bounds(&mut app, &ctx, "Adjustments");
    assert!(gallery.bottom() < adjustments.top());
    click_ui(&mut app, &ctx, gallery.center());
    assert!(app.processing_dialog.is_none() && app.current().document == before);
    // The conventional catalog is now implemented; Gallery alone remains disabled.
    assert!(
        palette_core::processing::menu::CATEGORIES
            .iter()
            .all(|category| category
                .entries
                .iter()
                .all(|name| category.implemented(name).is_some()))
    );
}

#[test]
fn cloud_colors_are_captured_at_creation_and_scale_previews_apply_undo_and_native_preservation() {
    use palette_core::processing::{Kind, Value};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let before = app.current().document.clone();
    app.brush.color = [10, 40, 90, 255];
    app.background = Color32::from_rgb(240, 190, 30);
    app.open_processing(Kind::Clouds, None);
    wait_for_processing(&mut app, &ctx);
    assert_eq!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("low"),
        Some(Value::Color([10, 40, 90]))
    );
    assert_eq!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("high"),
        Some(Value::Color([240, 190, 30]))
    );
    let seed = label_position(&mut app, &ctx, "Seed");
    click_ui(&mut app, &ctx, seed);
    let scale = label_position(&mut app, &ctx, "Scale (px)");
    click_ui(&mut app, &ctx, scale);
    let scale = scoped_label_bounds(&mut app, &ctx, "Scale (px)", true);
    click_ui(&mut app, &ctx, pos2(scale.right() + 75., scale.center().y));
    wait_for_processing(&mut app, &ctx);
    assert_ne!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("scale"),
        Some(Value::Number(128.))
    );
    assert!(app.current().document == before);
    let preview = app.processing_preview().unwrap().id();
    assert!(canvas_uses_texture(&mut app, &ctx, preview));
    let apply = label_position(&mut app, &ctx, "Apply");
    click_ui(&mut app, &ctx, apply);
    let saved = app.current().document.clone();
    let id = saved.active().filters[0].id;
    assert!(saved.active().source == before.active().source);
    app.brush.color = [255, 0, 0, 255];
    app.background = Color32::BLACK;
    app.open_processing(Kind::Clouds, Some(id));
    wait_for_processing(&mut app, &ctx);
    assert_eq!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("low"),
        Some(Value::Color([10, 40, 90]))
    );
    assert_eq!(
        app.processing_dialog
            .as_ref()
            .unwrap()
            .operation
            .value("high"),
        Some(Value::Color([240, 190, 30]))
    );
    app.cancel_processing();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clouds.palette");
    io::save_project(&path, &saved).unwrap();
    assert!(io::open(&path).unwrap() == saved);
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    assert!(app.current().document == saved);
}

#[test]
fn noise_cleanup_controls_preview_then_cancel_or_commit_one_removable_native_history_step() {
    use palette_core::processing::{Kind, Value};
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = processing_document();
    let before = app.current().document.clone();
    for commit in [false, true] {
        app.open_processing(Kind::ReduceNoise, None);
        wait_for_processing(&mut app, &ctx);
        let strength = label_position(&mut app, &ctx, "Strength");
        click_ui(&mut app, &ctx, strength);
        let details = label_position(&mut app, &ctx, "Preserve details");
        click_ui(&mut app, &ctx, details);
        let details = scoped_label_bounds(&mut app, &ctx, "Preserve details", true);
        click_ui(
            &mut app,
            &ctx,
            pos2(details.right() + 70., details.center().y),
        );
        wait_for_processing(&mut app, &ctx);
        assert_ne!(
            app.processing_dialog
                .as_ref()
                .unwrap()
                .operation
                .value("preserve_details"),
            Some(Value::Number(0.5))
        );
        assert!(app.current().document == before);
        let preview = app.processing_preview().unwrap().id();
        assert!(canvas_uses_texture(&mut app, &ctx, preview));
        let action = label_position(&mut app, &ctx, if commit { "Apply" } else { "Cancel" });
        click_ui(&mut app, &ctx, action);
        if !commit {
            assert!(app.current().document == before);
        }
    }
    let saved = app.current().document.clone();
    assert_eq!(saved.active().filters.len(), 1);
    assert!(saved.active().source == before.active().source);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("noise-cleanup.palette");
    io::save_project(&path, &saved).unwrap();
    assert!(io::open(&path).unwrap() == saved);
    app.undo_current();
    assert!(app.current().document == before);
    app.redo_current();
    assert!(app.current().document == saved);
    let edit = scoped_label_bounds(&mut app, &ctx, "Edit", true).center();
    click_ui(&mut app, &ctx, edit);
    wait_for_processing(&mut app, &ctx);
    let more = label_position(&mut app, &ctx, "…");
    click_ui(&mut app, &ctx, more);
    let remove = label_position(&mut app, &ctx, "Remove");
    click_ui(&mut app, &ctx, remove);
    assert!(app.current().document.active().filters.is_empty());
    assert!(app.current().document.active().source == before.active().source);
}

// A branch and its first command can share a caption (Sharpen > Sharpen).
// Resolve the child by its painted position, rather than assuming pixel offsets.
fn rightmost_menu_label_position(app: &mut PaletteApp, ctx: &Context, label: &str) -> egui::Pos2 {
    fn find(shape: &egui::epaint::Shape, label: &str) -> Option<egui::Pos2> {
        match shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                Some(t.galley.rect.translate(t.pos.to_vec2()).center())
            }
            egui::epaint::Shape::Vec(shapes) => shapes
                .iter()
                .filter_map(|s| find(s, label))
                .max_by(|a, b| a.x.total_cmp(&b.x)),
            _ => None,
        }
    }
    let mut result = None;
    for _ in 0..6 {
        let mut output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386., 963.))),
                ..Default::default()
            },
            |ui| app.draw(ui),
        );
        result = output
            .shapes
            .iter()
            .filter_map(|s| find(&s.shape, label))
            .max_by(|a, b| a.x.total_cmp(&b.x));
        output.textures_delta.clear();
    }
    result.unwrap_or_else(|| panic!("Menu label not found: {label}"))
}

#[test]
fn idle_percentage_roundtrips_do_not_restart_filter_previews() {
    use palette_core::processing::{Kind, Value};
    for (kind, key) in [
        (Kind::ColorBalance, "midtone_red"),
        (Kind::ChannelMixer, "red_green"),
        (Kind::SelectiveColor, "yellows_magenta"),
        (Kind::PhotoFilter, "density"),
    ] {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.current_mut().document = processing_document();
        app.open_processing(kind, None);
        let draft = app.processing_dialog.as_mut().unwrap();
        draft.operation.set_value(key, Value::Number(0.6)).unwrap();
        draft.parameter = kind
            .descriptor()
            .parameters
            .iter()
            .position(|p| p.key == key)
            .unwrap();
        wait_for_processing(&mut app, &ctx);
        let ticket = app.processing_dialog.as_ref().unwrap().ticket;
        for _ in 0..30 {
            frame(&mut app, &ctx, vec![]);
        }
        let draft = app.processing_dialog.as_ref().unwrap();
        assert_eq!(draft.ticket, ticket, "{kind:?} idle frame changed ticket");
        assert_eq!(draft.ready, Some(ticket));
    }
}

#[test]
fn cancelled_main_render_targets_are_requeued_when_tabs_are_revisited_and_drop_cancels_work() {
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    let (tx, rx) = mpsc::sync_channel(1);
    app.render_tx = tx;
    app.request_render(&ctx);
    let first = rx.recv().unwrap();
    assert_eq!(app.current().requested, Some(first.generation));
    assert!(label_bounds(&mut app, &ctx, "Rendering…").is_positive());
    app.new_document();
    app.request_render(&ctx);
    let second = rx.recv().unwrap();
    assert_ne!(first.id, second.id);
    assert!(second.ticket > first.ticket);
    assert_eq!(app.sessions[0].requested, None);
    app.active = 0;
    app.request_render(&ctx);
    let revisited = rx.recv().unwrap();
    assert_eq!(revisited.id, first.id);
    assert_eq!(revisited.generation, first.generation);
    assert!(revisited.ticket > second.ticket);
    app.current_mut()
        .command("New generation", |d| d.active_mut().opacity = 0.5);
    app.request_render(&ctx);
    let edited = rx.recv().unwrap();
    assert!(edited.ticket > revisited.ticket);
    assert_ne!(edited.generation, revisited.generation);
    let ticket = app.render_ticket.clone();
    let before = ticket.load(Ordering::Relaxed);
    drop(app);
    assert!(ticket.load(Ordering::Relaxed) > before);
}

mod layer_panel;
mod reordering;
mod style_settings;
