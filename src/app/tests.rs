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
    app.request_close(first, &ctx);
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
    let origin = app.canvas_rect.center() - vec2(819.0, 594.0) * 0.75 * 0.5;
    let handle = origin + vec2(819.0, 594.0) * 0.75 + vec2(3.0, 3.0);
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
