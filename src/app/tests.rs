use super::*;
use egui::{Event, Modifiers, PointerButton, RawInput, Rect, pos2};

fn frame(app: &mut PaletteApp, ctx: &Context, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1386.0, 963.0))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();
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
    let mut app = PaletteApp::with_context(&ctx);
    app.current_mut().zoom = 0.75;
    app.current_mut().command("Add layer", Document::add_layer);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let origin = app.canvas_rect.center() - vec2(819.0, 594.0) * 0.75 * 0.5;
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
    assert_eq!(
        app.current().document.layers[0].source.get(50, 10),
        [255; 4]
    );
    app.current_mut().undo();
    assert_eq!(app.current().document.active().source.get(50, 10), [0; 4]);
    app.current_mut().redo();
    assert!(app.current().document.active().source.get(50, 10)[3] > 200);
}

#[test]
fn tab_state_and_unsaved_close_are_independent() {
    let ctx = Context::default();
    let mut app = PaletteApp::with_context(&ctx);
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
    let close_button = pos2(77.0, 47.0);
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
    assert_eq!(app.sessions[1].document.active().source.get(0, 0), [255; 4]);
}

#[test]
fn canvas_resize_preserves_source_and_is_reversible() {
    let ctx = Context::default();
    let mut app = PaletteApp::with_context(&ctx);
    app.current_mut()
        .document
        .active_mut()
        .source
        .set(0, 0, [25, 50, 75, 100]);
    // Exercise the real resize command through its handle's image-space bounds.
    frame(&mut app, &ctx, vec![]);
    let handle = image_position(&app, [819.0, 594.0]) + vec2(3.0, 3.0);
    let delta = vec2(40.0, 20.0) * app.current().zoom;
    frame(&mut app, &ctx, vec![Event::PointerMoved(handle)]);
    frame(&mut app, &ctx, vec![pointer(handle, true)]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(handle + delta)]);
    frame(&mut app, &ctx, vec![pointer(handle + delta, false)]);
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (859, 614)
    );
    assert_eq!(
        app.current().document.active().source.get(0, 0),
        [25, 50, 75, 100]
    );
    app.current_mut().undo();
    assert_eq!(
        (app.current().document.width, app.current().document.height),
        (819, 594)
    );
}

#[test]
fn ribbon_and_layers_remain_within_a_small_window() {
    let ctx = Context::default();
    let mut app = PaletteApp::with_context(&ctx);
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
fn wait_for_edit(app: &mut PaletteApp, ctx: &Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.edit_pending {
        app.poll_edits();
        assert!(std::time::Instant::now() < deadline, "edit timed out");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    frame(app, ctx, vec![]);
    assert!(app.error.is_none(), "{:?}", app.error);
}

#[test]
fn selection_fill_move_crop_workflow_through_fractional_zoom_pointer_input() {
    let ctx = Context::default();
    let mut app = PaletteApp::with_context(&ctx);
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
    assert_eq!(
        app.current().document.layers[0].source.get(20, 20),
        [255; 4]
    );
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
        (819, 594)
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
    let mut app = PaletteApp::with_context(&ctx);
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
    let mut app = PaletteApp::with_context(&ctx);
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
    assert_eq!(app.current().document.active().source.get(1, 0), [255; 4]);
    assert!(app.message.contains("cancelled"));
}

#[test]
fn selected_shape_and_brush_do_not_change_unselected_pixels() {
    let ctx = Context::default();
    let mut app = PaletteApp::with_context(&ctx);
    app.current_mut().command("Add layer", Document::add_layer);
    app.current_mut().document.selection = Some(palette_core::editing::rectangle(
        819,
        594,
        palette_core::editing::PixelRect {
            x: 20,
            y: 20,
            width: 20,
            height: 20,
        },
    ));
    app.tool = Tool::Rectangle;
    app.shape_fill = true;
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
    assert_eq!(
        app.current().document.active().source.get(25, 30),
        [255, 0, 0, 255]
    );
    assert_eq!(app.current().document.active().source.get(15, 30), [0; 4]);
}
