# Phase 2 construction tools and selection-aware transforms

> Historical milestone report. Current codec/specimen evidence and remaining gates are in [PHASE3_VALIDATION.md](PHASE3_VALIDATION.md) and [PAINT_RESEARCH.md](PAINT_RESEARCH.md).


Current follow-up: selected-pixel move/resize/rotation handles and live previews are implemented; see [selection handle validation](SELECTION_HANDLES_VALIDATION.md). Earlier pending items and test counts below describe the historical milestone.

## Requested fixes and next milestone

- Flip Horizontal/Vertical and all orthogonal rotations now act on selected source pixels of the active layer when a coverage selection exists. Other layers and canvas dimensions remain unchanged. Without a selection they retain the whole-document behavior.
- Rotate custom is available in the rotation flyout. Its centered dark dialog follows the supplied screenshot, with an initially selected angle field, positive-clockwise degrees, cyan OK, Enter confirmation and Escape/Cancel. Invalid/nonfinite input is not committed.
- Finished drag shapes, polygons and curves receive a clipped selection in the same undo command and switch to Select, allowing immediate move/flip/rotation.
- Free Polygon is functional: click/drag the first segment, click more vertices, and see the next segment follow the pointer between clicks. Enter, double-click, right-click or clicking the first vertex closes a valid polygon. Backspace removes the last vertex and Escape cancels.
- The next construction milestone also enables Curve: drag a baseline, then drag two bends to complete the cubic; Enter can finish early and Escape cancels. Both tools share preview/commit geometry and keep source pixels unchanged until completion.

All 23 icons in the reference gallery now have functional tools. The Paint-style ribbon layout, marching ants, 1280 x 720 default and 100% initial zoom remain intact. This completes the curve/polygon construction slice, not the entire Phase 2 roadmap.

## Implementation and changed files

- `crates/palette-core/src/transform/selection.rs`: source/coverage extraction, selected orthogonal transforms, coverage-aware alpha rejoining, custom-angle inverse resampling and layered whole-image rotation.
- `crates/palette-core/src/transform.rs`: expose selection-aware/custom operations while retaining the existing whole-document API.
- `crates/palette-core/src/shapes.rs`: validated explicit paths, cubic flattening and commit-plus-selection helper.
- `src/canvas/construction.rs`, `src/canvas.rs`, `src/canvas/overlays.rs`, `src/session.rs`: per-session construction transactions and shared live path previews.
- `src/app/editing.rs`, `src/app.rs`, `src/app/text.rs`, `src/app/appearance.rs`: scoped job dispatch, current-generation commitment, automatic selection mode and transactional custom rotation.
- `src/ribbon.rs`: activate Curve/Polygon and add Rotate custom.
- Core and egui interaction tests; four new native capture scenes, fourteen reviewed Windows baselines, README/checklist/architecture/limitations/file/build/UI notes.

## Transform semantics

Selected operations transform source pixels, retaining local masks, live filters/effects, opacity and blend mode rather than baking the evaluated appearance. Selected coverage follows the transform, including selected transparent pixels and grayscale coverage. Other pixels are preserved outside the old/new selected footprint. Selected content crossing the canvas edge is clipped; whole-document custom rotation expands canvas bounds and preserves all layers, including masks/filter masks.

Exact quarter turns use pixel permutations and a stable integer-grid center; four clockwise turns restore pixel positions without half-pixel drift. General angles use premultiplied bilinear sampling so transparent RGB cannot introduce colored halos. Fractional coverage is rejoined by distinguishing overlap from new-only selected coverage, avoiding double attenuation. Output/staging allocations and transformed offsets are validated before document commitment; a failure cannot partially replace the document. Large whole-plane operations remain subject to the existing 512 MiB planning policy and 64-megapixel raster limit.

Shape selection is a raster bounding selection, intersected with any existing coverage selection. It can include existing source pixels enclosed by the bounds. This is not a claim of persistent editable vector shapes or native Paint's post-draw transformation handles. Arbitrary rotations are destructive source transforms with undo; repeated nonorthogonal rotations can lose detail.

Construction drafts are transient and capture width/colors/opacity at the start. Finish them explicitly before changing tools/documents or saving; Escape and tool/document transitions cancel an unfinished draft. Saved projects contain committed pixels/selection only. Curve control-point behavior is an independent cubic implementation, not an assertion of exact Microsoft handle semantics. Preview masks/blending/filter parity and interactive resize/rotation handles remain pending.

## Validation on this Windows environment

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 71 passed; two desktop tests excluded from this normal run.
- `cargo build --workspace --locked`: passed. An initially running app locked the default executable; a separate `target/transform-milestone` build/test run preserved that session. The normal target later built successfully after the lock cleared; no user app was forcibly closed.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passing desktop tests compare fourteen native captures at 1386 x 963.
- Native captures reviewed for the custom dialog, selected shape, polygon rubber band and cubic preview; compact 780 x 520 custom-dialog layout also inspected. Existing main/menu/dialog/layer scenes remain in the regression suite.

New core tests check all five selected orthogonal operations, unchanged canvas/background/outside pixels, stable quarter-turn positioning, translated layers, fractional-alpha preservation, custom-angle scoping, exact 90-degree dispatch, invalid-angle/lock handling, native round-trip, full-document mask/filter retention and transparent-edge color. New real-egui tests cover flip scope/undo, custom dialog Cancel and Enter, automatic shape selection, a visible next-segment primitive between polygon clicks, one-command polygon commitment, and curve bending/commit/cancel. Existing live paint, alpha, masks, selection, clipboard, file and history tests pass.

The native screenshot scenes seed deterministic states and do not claim OS pointer automation. No macOS, Linux, Windows 10 or high-DPI matrix was run locally. No dependency/native-library changes were needed. All changes remain uncommitted under AGENTS.md.

## Recommended next slice

Add interactive selected-pixel resize/rotation handles and a reusable floating-selection preview. Then implement real brush variants and coalesce property gestures. Phase 3 still needs AVIF validation, real `.paint` specimens/Windows interoperability and native asset/migration work; those formats remain disabled.
