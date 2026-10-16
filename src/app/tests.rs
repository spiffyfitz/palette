use super::*;
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
    frame(&mut app, &ctx, vec![Event::PointerMoved(pos2(45.0, 47.0))]);
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
    assert_eq!(app.sessions[1].document.active().source.get(0, 0), [255; 4]);
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
            selection: None,
            id: app.sessions[0].id,
            epoch,
            generation,
            image: texture_image(&pixels, 16384),
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
        selection: None,
        id: app.current().id,
        epoch,
        generation: app.current().generation,
        image: texture_image(&pixels, 16384),
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
    assert_eq!(app.current().document.active().source.get(1, 0), [255; 4]);
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
    app.choose_tool(Tool::MoveLayer);
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
    app.choose_tool(Tool::MoveLayer);
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
            id: app.current().id,
            generation: app.current().generation + 100,
            epoch: stale_epoch,
            image: texture_image(&pixels, 16384),
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
            result: Ok(Document::new(4, 4).unwrap()),
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
    restored.start_save(original.clone(), false, None, &ctx);
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
fn avif_export_options_cancel_and_export_leave_native_document_modified() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("image.avif");
    let ctx = Context::default();
    let mut app = test_app(&ctx);
    app.current_mut().document = Document::new(16, 16).unwrap();
    app.current_mut().command("Edit", |doc| {
        doc.active_mut().source.set(2, 3, [200, 30, 80, 180])
    });
    let before = app.current().document.clone();
    app.avif_export = Some(files::AvifExport {
        path: path.clone(),
        options: io::AvifOptions::default(),
    });
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![escape_event()]);
    assert!(app.avif_export.is_none());
    assert!(!path.exists());
    app.start_save(
        path.clone(),
        true,
        Some(io::AvifOptions {
            quality: 80,
            speed: 10,
        }),
        &ctx,
    );
    wait_for_files(&mut app, &ctx);
    assert!(app.current().modified());
    assert!(app.current().path.is_none());
    assert!(app.current().document == before);
    assert_eq!(
        [
            io::open(&path).unwrap().width,
            io::open(&path).unwrap().height
        ],
        [16, 16]
    );
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
