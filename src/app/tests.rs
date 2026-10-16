use super::*;
use egui::{Event, Modifiers, PointerButton, RawInput, Rect, pos2};

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
        let mut app = PaletteApp::with_context(&ctx);
        app.current_mut().zoom = 0.75;
        app.current_mut().command("Add layer", |doc| {
            doc.add_layer();
            if tool == Tool::Eraser {
                doc.active_mut().source = Raster::new(819, 594, [90, 30, 180, 200]).unwrap();
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
    let mut app = PaletteApp::with_context(&ctx);
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
    let mut app = PaletteApp::with_context(&ctx);
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
    let mut app = PaletteApp::with_context(&ctx);
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
    let mut app = PaletteApp::with_context(&ctx);
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
    let mut app = PaletteApp::with_context(&ctx);
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
    let mut app = PaletteApp::with_context(&ctx);
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
