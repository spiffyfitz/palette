# Phase 2: interactive raster selection transforms

## Working behavior

Select mode now shows eight compact resize handles and a circular rotation handle above the selected bounds, including automatically selected finished shapes. Drag selected pixels to move them. All three operations display the actual processed layer composite while the pointer remains down. Ants track transformed coverage, including holes and transparent selected pixels. A cyan resize outline shows the requested target immediately while processing catches up.

Shift preserves resize proportions, constrains movement to one axis or snaps rotation to 15 degrees. Alt starts a replacement selection. Escape restores the exact pre-gesture document without adding history. Normal release commits one undo command, including the final pointer position even if the worker is behind. Tab/tool/save/close transitions finish the target before the next action; undo/redo and modified-state tracking remain per document. Handles have fixed logical-point sizes, independent of zoom/DPI. Their hit targets preserve room for moving small selections.

Dragging a resize edge beyond its opposite edge clamps the extent at one pixel. It does not mirror or produce unbounded geometry. Locked layers do not offer transform handles. Select mode hides the canvas-resize corner when coverage exists to avoid overlapping controls; Image Properties still changes canvas bounds.

## Files changed in this slice

- `crates/palette-core/src/transform/selection.rs` and `transform.rs`: public selection transform descriptor, validated selected resize/move, alpha-safe sampling and coverage rejoining.
- `crates/palette-core/src/editing.rs`: route selection movement through the shared coverage-aware transform implementation.
- `src/session.rs`: immutable gesture snapshots, desired/applied state, cancellation and single-command completion.
- `src/canvas/selection.rs` and `canvas.rs`: handle placement/hit testing, pointer mapping, constraints, live outlines and input priority.
- `src/app.rs` and `src/app/editing.rs`: bounded worker preview dispatch, progressive results, stale/error guards and Escape routing.
- `src/app/tests.rs`, core selected-transform tests: interaction, mathematical and lifecycle coverage.
- `src/app/capture.rs`, `tests/screenshot.rs` and selection fixtures: deterministic actual native captures, including two new live-preview scenes.
- README, architecture, checklist, known limitations, build/UI notes and this validation record.

## Validation

On the available Windows environment:

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 80 passed; two desktop tests excluded from the ordinary run.
- `cargo build --workspace --locked`: passed.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passing desktop tests, sixteen native states at 1386 x 963.
- Actual wgpu captures reviewed for selected handles, selection with a hole, resized pixels, rotated pixels and a compact 780 x 520 layout. Four selection baselines were reviewed/updated, including the two new preview scenes.

Real egui pointer/key event tests verify all eight handle directions and fixed opposite edges, live movement with filters before mouseup, final composite parity, unchanged history during previews, one-command undo/redo, cancellation and deterministic late-result rejection, 150%/200% logical-DPI scaling at fractional zoom, rotation snapping, one-pixel crossing, Shift aspect ratio, and a tab transition before the final worker result. Core tests verify unchanged outside pixels/other layers/masks/filters, transparent-edge color, separately transformed transparent coverage, overlap alpha, clipping, no-ops, locks and failed-allocation atomicity. Existing painting, shapes, polygon/curve, file, clipboard and history checks pass.

Native capture scenes are seeded diagnostics, not OS pointer automation. High-DPI interaction uses egui input tests, not a physical multi-monitor test. macOS, Linux, Windows 10 and tablet behavior were not run locally. No dependency updates or format-support claims were made. All work remains uncommitted.

## Limits and next stage

These are raster selection handles, not persistent editable vector shapes. Every preview samples the original pixels for that gesture, but separately committed resizes/arbitrary rotations can lose detail. Selected source pixels transform; masks and live processing remain on the layer. Pixels moved outside the canvas are clipped. Resize uses bilinear sampling; pixel-art nearest-neighbor selection resizing remains a later option.

The capacity-one CPU render queue bounds queued snapshots. Running work is not cancelled, large previews can lag, and contour caching remains on the UI thread. A command that interrupts a pending final preview completes it synchronously; large selections can pause that transition. This is not a claim of measured 4K/8K latency or GPU affine rendering. Existing allocation/history limits still apply.

Recommended next Phase 2 slice: implement actual brush variants and coalesce property-slider gestures into single undo commands. Then address text-preview parity and remaining Phase 2 polish before the Phase 3 codec/interoperability work. AVIF and Microsoft `.paint` remain disabled pending real codec/fixture and Microsoft Paint validation.
