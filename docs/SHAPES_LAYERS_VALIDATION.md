> Historical common-shape/layer milestone. The [selection/shape follow-up](SELECTION_SHAPES_VALIDATION.md) adds animated ants, bounded thin previews and the complete reference gallery layout.

# Phase 2: common shapes, layer movement and HD defaults

## Scope completed

This is the next complete Phase 2 editing slice from the master plan. Phase 2 as a whole and Phase 3 file compatibility remain unfinished.

- New documents use 1280 x 720 at 100% zoom. Image Properties > Default shares that size. Existing project/import sizes are preserved. On a narrower workspace, pan or Fit reveals canvas edges.
- Rounded rectangle, triangle, diamond, pentagon, hexagon, five-point star, right arrow and down arrow are functional in the existing shape gallery, alongside line/rectangle/ellipse.
- Outline and fill are independent: foreground outline, background fill, shared opacity and adjustable width. Right mouse swaps colors. Shift constrains closed shapes to a square or lines to 45-degree increments. Solid shapes do not expose a working hardness slider.
- Drag previews retain source pixels until release; Escape cancels. The guarded background edit worker rasterizes and commits one undo command. Layer offsets, locks, canvas-space grayscale selection and straight alpha are respected.
- Selection > Move active layer, or a layer context menu > Move layer, selects whole-layer movement. Dragging previews placement; Shift constrains an axis, arrows nudge one pixel, Shift+arrows ten, and Escape restores the starting position. One completed drag is one undo entry. Transparent source, masks, filter/effect stacks, layer identity and other layers remain intact.
- Progressive renders are accepted within a move gesture so fast pointer changes do not starve the display. Start/end/cancel fence obsolete results. Save, close, tab/tool switches and image-property dialogs finish a move first.

## Changed code

- `crates/palette-core/src/shapes.rs`: shared contours, concave preview triangulation, bounded curve approximations, antialiased scanline fills and stroke composition.
- `crates/palette-core/src/editing.rs`: checked whole-layer translation without pixel rewriting.
- `src/canvas.rs`, `src/session.rs`: shape preview/commit, move gesture, rendering epochs and gesture history.
- `src/ribbon.rs`, `src/icons.rs`, `src/app/panels.rs`: activate implemented shape controls, independent styles and move-tool access without adding a permanent toolbar.
- `src/interface.rs`, `src/app.rs`, `src/app/appearance.rs`: shared HD default and gesture-safe document actions.
- `src/app/editing.rs`: guarded shape jobs, nudging and cancellation; open menus consume navigation without moving layers behind them.
- Core/app tests, native capture scene and eight reviewed Windows screenshot baselines. README, architecture, checklist, limitations and file/build/UI notes updated.

## Validation performed

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 55 passed; two desktop tests intentionally ignored in this normal run.
- `cargo build --workspace --locked`: passed.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passed, comparing eight actual eframe/wgpu desktop captures at 1386 x 963.
- Visually reviewed default HD canvas, File, Brushes, Colors, inline Text, Image Properties, Layers and the new eight-shape scene. The source-only controls remain disabled where unfinished. The Paint-style ribbon dimensions and panel arrangement remain intact.

New correctness coverage includes reversed drags for every closed shape, concave triangulation, partial edge alpha, independent outline/fill opacity, selection clipping on translated layers, lock/invalid-input handling, degenerate/outside geometry, constraints, checked placement overflow, and native project preservation of a moved masked/filtered layer.

Real egui input tests cover new-document dimensions/zoom, shape preview cancellation and asynchronous commitment at 75% zoom, unchanged background layer, undo/redo, live layer movement before pointer release, cancellation without a revision, nudging, lock protection and close-during-drag history preservation. The live move test checks its rendered composite while the pointer is still held. These are in-process egui pointer/key tests; the native screenshot scenes seed reproducible states and do not claim manual OS input validation.

Only this Windows environment was built/launched here. No macOS, Linux, Windows 10, high-DPI matrix or tablet validation is claimed. No dependency changes or new native libraries were introduced. Source and documents remain uncommitted under AGENTS.md.

## Limits and next milestone

Shapes become source pixels on release. Persistent shape objects, post-draw resize/rotation handles, Bezier curves and free polygons remain pending. Previews share geometry/colors with final output but do not yet evaluate selection coverage, masks, blend modes or live filters. Ellipse/rounded contours are bounded polygon approximations rather than exact analytic curves. Fill coverage uses four vertical samples; it is not an unlimited-quality vector renderer.

Whole-layer movement is integer translation with progressive full CPU rendering. It does not provide per-layer resizing/rotation, unlinked masks, subpixel transforms or GPU/tiled movement. Large multi-layer 4K/8K movement has not been benchmarked in this pass; full background composition remains its performance limit.

Next recommended Phase 2 slice: curve/free-polygon tools and post-draw shape interaction, followed by functional brush variants and typography parity. Phase 3 still needs AVIF codec validation, real `.paint` fixtures and native asset/migration work; none is enabled by this milestone.
