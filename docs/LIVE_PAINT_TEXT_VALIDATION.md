> Historical live-paint/text milestone. The later video-guided pass replaces the modal text editor with inline text. See [current UI behavior and validation](UI_BEHAVIOR_REFERENCE.md).

> Historical milestone report. Current codec/specimen evidence and remaining gates are in [PHASE3_VALIDATION.md](PHASE3_VALIDATION.md) and [PAINT_RESEARCH.md](PAINT_RESEARCH.md).


# Live painting and text milestone — 2026-10-16

This completes the live-stroke fix and the next Phase 2 tool slice. It does not declare the entire Phase 2 roadmap finished. Changes remain uncommitted, following the current root `AGENTS.md`.

## Live painting

Previously every pointer sample incremented the document generation. Full-render results were accepted only at the exact current generation, so continuous movement repeatedly discarded the only available display updates. Once the mouse stopped, the final result could appear.

Painting now records changed source-pixel bounds, maps/clips them into canvas space, recomposites that region through the same CPU layer pipeline and uploads only its texture subimage. Soft opacity, selections, masks, filters, blending and erasing use the actual document pixels; there is no opaque stroke overlay substituted for the rendered result. Each gesture still produces one undo command.

Before a compatible display cache exists, or when the display texture is reduced for the GPU dimension limit, background snapshots can advance within a paint-only render epoch. Dirty bounds accumulate until a baseline is available. General edits/undo start a new epoch, and older displayed generations cannot roll the canvas backward.

## Text tool

Select the existing Text icon. Click for a default box or drag to specify bounds. A compact modal editor supports multiline entry, word wrapping, Ubuntu Light/Hack, 6–256 px size, left/center/right alignment, line spacing, RGBA text color, opacity, optional colored background and exact box position/dimensions. A bounded worker generates the canvas preview. Request tokens reject old parameters or cancelled/replaced drafts, and Apply is enabled only for the current preview.

Apply inserts the preview pixels as a new topmost raster layer in one undo command; existing layer pixels remain untouched. Selection coverage constrains preview and committed alpha identically. Cancel/Escape discards the draft without dirtying the document. Window close is deferred while the placement transaction is open; Apply/Cancel resolves it.

Text remains editable until Apply. Committed character/font data is not retained. Basic left-to-right layout, two bundled fonts, 4096 characters and 2048 × 2048 boxes are the current limits. System font discovery, bold/italic faces, complex shaping, bidirectionality and persistent text objects remain unfinished. Applied text round-trips as ordinary raster content in `.palette` v1.

## Checks performed

Environment: Windows x86-64, Rust 1.99.1, eframe/wgpu 0.37. No macOS/Linux launch or remote CI run occurred.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | 42 tests passed; desktop screenshot separately ignored in this command |
| `cargo build --workspace --locked` | Passed |
| `palette.exe --screenshot …` | Actual wgpu window launched and captured at 1386 × 963 |
| `cargo test --test screenshot --locked -- --ignored` | Passed against the visually reviewed baseline with Text enabled |
| Native pointer/keyboard smoke test | Held stroke visible before release; typed text visible during preview; Apply added a separate layer |

New tests check held brush/eraser texture subimage updates and equality with the full composite at 75% zoom; one-command undo/redo; progressive snapshot patching and rejection across render epochs; text placement through real egui pointer/text input at fractional zoom; transactional parameter freshness, Cancel and undo/redo; obsolete text-job rejection; regional/full compositor agreement with masks/filters/blending/extreme offsets; text edge RGB/alpha, line breaks/wrapping/alignment/clipping; input limits; fractional selection coverage; preservation of existing pixels; and native project round-trip.

The native interaction check launched a separate Palette instance, moved the actual pointer while holding the primary button, then typed into the Text editor and clicked Apply. Captures were visually inspected in `screenshots/actual-native-held-stroke.png`, `screenshots/actual-native-text-preview.png` and `screenshots/actual-native-text-applied.png`. These local diagnostics are ignored by Git. A first attempt using posted window messages did not exercise native input; the final check used actual pointer/keyboard events. Only the test instance was terminated, and the pointer position was restored. The host clipboard was not changed.

## Measured CPU work

Reproduce using `cargo run --locked -p palette-core --example render_bench`. Measurements below use this repository's optimized development profile, three full-canvas solid RGBA layers, one full render and the mean of 20 region renders. This is a local CPU comparison, not an end-to-end latency benchmark.

| Canvas | Full composite | 128 × 128 region |
|---|---:|---:|
| 819 × 594 | 36.31 ms | 1.21 ms |
| 3840 × 2160 | 614.13 ms | 1.21 ms |

The figures exclude event handling, patch copying, texture conversion/upload, thumbnails and presentation. General edits and large dirty rectangles remain expensive; this does not establish smooth interactive 4K/8K behavior. Neighborhood filters will need padded/staged region processing before they can use this optimization.

## Next milestone

Continue the remaining core Paint tools: additional shapes and reusable geometry, whole-layer movement, brush variants/spacing/pressure and coalesced property gestures. Then advance the Phase 3 storage/codec work. Microsoft `.paint` still requires the actual specimen and Microsoft Paint validation; AVIF remains disabled until real read/write checks. The planned Paint video can guide later gestures, animation and interface nuance.
