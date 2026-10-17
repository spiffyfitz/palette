# Phase 4 — blend modes, Magic Wand and coverage masks

> Current follow-ups: [selection/navigation](SELECTION_NAVIGATION_VALIDATION.md) and [brush selection](BRUSH_SELECTION_VALIDATION.md) supersede the historical preview/brush limitations recorded below.


This is a completed first Phase 4 slice, not completion of the entire phase. The user confirmed the generated `.paint` opens and looks correct in Microsoft Paint. That result is recorded as user-provided manual opening/appearance evidence; native layer editing and Paint-save-back were not inferred. Development proceeds with the established Paint interface.

## Working features

- All 18 requested modes plus Divide are real compositor operations. Layer properties expose their human-readable names in a scrollable combo. Source pixels, opacity, masks, filters and effects stay separate and native projects preserve every mode.
- Selection > Magic Wand selects connected pixels or all matching colors. Selection > Magic Wand options exposes tolerance 0–100, Contiguous, Sample all visible layers and Antialias edges. Current-layer sampling includes live local processing and layer opacity in canvas coordinates; merged sampling uses the visible composite. A locked layer can be sampled without changing it.
- Selection > Combine selection provides Replace/Add/Subtract/Intersect for Wand and rectangle selection. Shift adds, Alt subtracts, Shift+Alt intersects. Those settings and Wand parameters belong to each tab. Replace permits existing selection move/resize/rotate handles. Shift starts additive selection in the interior but still constrains a handle gesture. In explicit Add/Subtract/Intersect modes, transformation handles are hidden.
- Selection > Feather selection opens a compact Apply/Cancel dialog with a precise 0–128 px radius. Changes run on a session/generation-fenced background worker and commit one undo command. Cancel leaves the document untouched. Selection and mask feathering change coverage, not original image RGB/alpha.
- Layer properties immediately below the selected layer offer Mask from selection or Add reveal-all mask. Existing masks have a separate grayscale thumbnail, Enabled, Invert mask, Feather mask and Delete mask. Locks protect these changes. Disabled masks keep their underlying coverage and disabled state during editing. Delete removes coverage rather than applying it to pixels.
- Marching ants trace the 50% coverage boundary, including holes; editing and crop/transform bounds continue to retain fractional/nonzero coverage. A very faint selection can therefore have no visible ants.
- History/native saving preserve mask/selection coverage, original pixels, offsets and local processing. Full-canvas selection-to-mask conversion shares immutable coverage; mask inversion uses copy-on-write so it cannot change the selection or prior history.

## Implementation and mathematical references

`composite/blend.rs` uses independent [W3C blend equations](https://www.w3.org/TR/compositing-1/#blending) and their nonseparable luminosity/saturation helpers. Linear Dodge/Burn use clamped addition/subtraction; Divide clamps backdrop/source with a defined zero denominator. Blending occurs in the existing sRGB working space before straight-alpha source-over arithmetic. It does not apply ICC profiles or simulate Photoshop's undocumented color settings.

`selection.rs` compares premultiplied Oklab coordinates plus alpha, using the published [Oklab matrices](https://bottosson.github.io/posts/oklab/). Hidden RGB at zero alpha is ignored. Connectivity uses a four-connected hard threshold before a one-pixel tent antialias pass. A bounded color memo avoids repeated conversion of flat colors without a full perceptual image allocation. The scanline frontier caps at one million pending spans; the color cache caps at 4096 entries. Tolerance 100 explicitly selects the whole canvas. Source planes retain the existing 64-megapixel cap.

Coverage union is a+b−ab, subtraction a(1−b), intersection ab, with rounded byte output. With no selection, Add starts the incoming region; Subtract/Intersect treat the unconstrained canvas as full. Feathering approximates a Gaussian with three separable running-sum box widths; radius is sigma in pixels. It uses two byte planes, clamped boundaries and byte rounding between passes, and preserves constant masks. No new crate/native dependency or unsafe Rust was added.

## Validation on Windows

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 119 passing tests; the two desktop tests are intentionally excluded from that run.
- `cargo build --workspace --locked`: passed; native wgpu executable launched for capture.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passing tests cover 25 native states at 1386 x 963. New reviewed scenes: selection flyout, Wand submenu, layer mask and feather dialog. Expanded layer-properties baseline updated. Compact 780 x 520 captures reviewed for all four new states.
- Analytic opaque/semitransparent blend vectors, singular endpoints, hidden-RGB/zero-alpha behavior and nonseparable luminosity/gamut tests pass. All blend variants and masks round-trip through native storage/history.
- Wand tests cover disconnected islands, tolerance monotonicity, transparent hidden RGB, merged/current sampling, offsets, antialiasing, read-only locked layers and malformed parameters. A separate four-neighbor breadth-first traversal checks scanline results across generated patterns.
- Coverage tests cover fractional set algebra, canvas-size validation, immutable sharing, symmetric feathering, tiny dimensions, constant masks, disabled-mask preservation, invalid radii and offset reanchoring. Original source data remains unchanged.
- Real egui input tests select Wand/settings through the ribbon, combine clicks at fractional zoom, exercise rectangle combination instead of accidental pixel movement, edit a blend from the dropdown, cancel/apply feathering, create/invert/disable/feather/delete masks, undo/redo them, and reopen their saved native project. Other painting/text/transform/file tests still pass.

The generated visual reference is `crates/palette-core/tests/fixtures/blend-modes.png`; adjacent `.txt` labels its rows. Three columns show opaque, partial-alpha and transparent-backdrop ramps. Regenerate with `cargo run -p palette-core --example blend_reference --locked -- crates/palette-core/tests/fixtures/blend-modes.png`. It is a reviewed visual aid, alongside independent analytic tests, not proof of Photoshop pixel parity.

## Measured assumptions

`cargo run -p palette-core --example selection_benchmark --locked` measures CPU processing only. One optimized-dev Windows run, excluding UI composition/texture upload and document construction:

| Canvas | Uniform contiguous antialiased Wand | 12 px selection feather |
|---|---:|---:|
| 1280 x 720 | 66.00 ms | 15.07 ms |
| 3840 x 2160 | 595.46 ms | 147.56 ms |

These are a single run on a flat-color source that benefits from color caching. They do not establish photographic/8K latency, peak process memory or overall UI smoothness. Merged sampling additionally allocates a composite. Rectangle creation/combination remains a synchronous release-time operation. Running worker jobs are fenced on completion rather than interrupted.

## Remaining Phase 4 work

Mask brush/eraser targeting, direct grayscale canvas view, link/unlink and explicit permanent application are pending. Ordinary brushes still edit source pixels. Polygon/brush selection, expansion/contraction, selected/visible layer merges and confirmed flattening are also pending. Feather radius is a committed coverage edit, not a retained live node; there is no live slider preview, and repeated application feathers the current result. Those limits are explicit in FEATURES.md and KNOWN_LIMITATIONS.md. No macOS, Linux, Windows 10, comprehensive DPI/accessibility/tablet or remote CI execution was performed here.

## Changed files

Core: document blend catalog, composite/alpha integration, new composite/blend.rs and selection.rs, contour threshold, core exports, analytic/coverage/native tests and reference/benchmark examples. App: new app/advanced.rs, worker routing, per-tab options, selection gestures, ribbon submenus, original Wand icon, layer mask properties, dialog blocking and capture/input tests. Reviewed Windows baselines, reference PNG/legend and README/architecture/roadmap/checklist/compatibility/build/license/limitations/validation documents were updated. Existing working-tree changes were preserved. No commits, branches or pushes were made.

## Canvas selection toolbar and lasso follow-up (2026-10-16)

Wand, rectangle and new Freeform selection tools show a foreground toolbar at the top of the canvas workspace, matching the text toolbar’s 46-point outer height. Width follows actual controls; only Wand includes tolerance and a sampling-options popup. All three expose feather slider/numeric entry, four combine icons and inversion. The toolbar centers over the workspace, avoids the layers tray at 780 x 520, and fences canvas pointer input. Feather radius is a per-tab setting for incoming selections; the existing Feather selection dialog handles current coverage. No live current-selection feather preview or native format change is claimed.

Freeform draws a closed live marching outline, commits antialiased even/odd coverage on release, cancels with Escape/tool/tab changes, preserves source pixels and participates in one-step undo/redo and selection handles. Samples use logical display distance at the current zoom; long gestures decimate at 4096 retained vertices. Core validation rejects invalid dimensions, excessive/nonfinite/unbounded vertices.

Validation: fmt, strict all-target Clippy, workspace tests (129 passed, two desktop tests separately), workspace build and native Windows launch. Added input tests cover height parity with text, dynamic width, no canvas movement, numeric validation, per-tab state, toolbar click-through, all combine icons/inversion, ribbon freeform entry, fractional-zoom lasso commit/cancel/undo, feather-before-combine and minimum window geometry at 100/150/200% egui scaling. Core tests check independent concave geometry, clipped/reversed paths, fractional coverage, self-crossings, degeneracy and malformed input. Native captures reviewed at 1386 x 963 and 780 x 520; regression suite now covers 29 states, adding rectangle/Wand/freeform toolbars and a lasso draft. Existing selection-state baselines were reviewed and updated for the new overlay. Synthetic high-DPI geometry tests are not evidence of physical multi-monitor testing.

Changed files: selection toolbar, shared text-frame metrics, session/tool state, canvas/selection gestures, ribbon/icons, worker routing, core selection/shared shape coverage, input/core/screenshot tests, reviewed Windows fixtures and project documentation. No new dependencies or unsafe Rust; all edits remain uncommitted. Next: edge-aware selection brush with tolerance; polygon selection, mask painting and the other Phase 4 gates remain tracked.

Current follow-up: [selection outlines and canvas scrollbars](SELECTION_NAVIGATION_VALIDATION.md), including retained original feather contours, native v4 migration and the 32-state screenshot suite.
