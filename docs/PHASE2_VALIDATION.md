# Phase 2 editing workflow — 2026-10-16

This milestone adds working selection, fill, clipboard and geometry operations while keeping the Paint-style layout. It is a Phase 2 vertical slice; text, further shapes and brush variants remain unfinished. The planned Paint video can guide later animation, gesture and interface refinements.

This records the earlier editing milestone. See [live painting and text validation](LIVE_PAINT_TEXT_VALIDATION.md) for the subsequent text tool and rendering fixes.

## Implemented behavior

- Rectangular selections are full-canvas grayscale coverage. Select all, deselect and invert work through the Selection/Edit menus and shortcuts. Static contours include holes from an inverted selection. Bounds/contours are cached in each tab; selection dimensions appear in the status bar.
- Brush, pencil, eraser and shapes respect selection coverage and layer position. Drag selected pixels to move them; Alt+drag replaces the selection. Arrows nudge one pixel, Shift+arrows ten. Movement previews the contour and commits pixels on release. Layer source/effects/masks remain separate.
- Fill uses a contiguous iterative scanline search, adjustable 0–255 tolerance, foreground/background colors and brush opacity. Comparison uses premultiplied RGBA so invisible RGB does not create false boundaries. Selection and layer locks constrain edits.
- Native image clipboard copy/cut/paste is wired through arboard; a persistent clipboard handle retains X11 ownership. Cut clears pixels only after a successful clipboard write. Native eframe image-paste events also work when a text field does not have focus. Paste creates an independent layer, anchors to selection/canvas origin and expands canvas bounds if necessary.
- Crop changes canvas bounds and reanchors layers, retaining hidden source pixels. Clockwise/counterclockwise 90°, 180° and horizontal/vertical flips permute pixels exactly. Whole-image transforms include locked layers and transform local masks/filter masks and the selection.
- Image resize has aspect locking, nearest-neighbor and premultiplied bilinear modes. Filtering RGB in premultiplied space avoids transparent-edge halos. Layer/filter/effect properties stay editable. Unchanged dimensions are a no-op. Canvas bounds resizing remains separate from resampling.
- `PaintStroke` uses sparse maximum-coverage tiles and pre-stroke source pixels, making opacity independent of pointer event frequency and retracing within a gesture. Outlined shapes use the same accumulator.
- History now measures unique retained pixel/mask allocations beyond the live document, capped at 128 MiB and 256 commands. A small edit on an 8K canvas remains undoable without counting every shared pixel buffer again.
- Expensive fills/moves/clears/transforms run on background snapshots. Session/generation checks reject obsolete results. Successful edits commit one undo command; no-op results do not dirty the document. Transform allocation preflight rejects individual invalid planes and output totals above 512 MiB before materializing them.

## Validation performed

Environment: the same Windows x86-64/Rust 1.99.1 environment as the [foundation milestone](VALIDATION.md). No macOS/Linux or remote CI run occurred here.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | 33 tests passed; desktop screenshot test separately ignored in this command |
| `cargo build --workspace --locked` | Passed |
| Native `palette.exe --screenshot …` launch | Passed; actual wgpu window captured and visually reviewed |
| `cargo test --test screenshot --locked -- --ignored` | Passed against the reviewed Phase 2 baseline |

The new tests verify: pointer-driven select/fill/move/crop/undo at 75%; brush/shape selection clipping; synthetic native image-paste input and undo; stale background-result rejection; source-layer isolation; selection-aware alpha/eraser behavior; fill contiguity/tolerance/transparency/lock handling; copied appearance versus preserved source; all five exact pixel permutations; local mask/filter and offset tracking under rotation; bilinear transparent-edge behavior; selection resizing; hidden-source retention; aggregate-memory rejection without partial mutation; full clipboard-image retention through canvas expansion; 8K history sharing; and native save/reopen after selection movement, rotation and resize.

The 1386 × 963 screenshot baseline was reviewed and updated because previously dim Selection/Fill/Image controls are now active and compact rotation/flip menus are functional. The overall menu/ribbon/workspace/panel arrangement remains intact. Regression tests compare against that reviewed Palette baseline, not pixel equality with Microsoft Paint.

Core clipboard RGBA operations and the native image-paste event path were tested. The host's real clipboard was not overwritten for testing, so OS clipboard ownership/interoperability has not been exhaustively exercised. Existing user clipboard contents were left intact. Cross-platform clipboard and dialog tests remain outstanding.

## Remaining work

Text and font/layout behavior are the next core tool slice. Follow with more shapes, whole-layer positioning, brush variants/pressure/spacing, and coalescing slider/rename gestures into single undo commands. Selection animation, floating pixel previews, antialiasing, advanced selections, mask editing and UI motion should be refined using the future video reference. Rendering still recomputes the full image; this milestone does not claim smooth interactive 4K/8K editing or a new performance benchmark.

`.paint`, AVIF and native Paint validation remain gated as before. Native `.palette` storage is still version 1 compressed JSON with its documented size limits; no format compatibility claim changed.

## UI follow-up — 2026-10-16

New and opened documents now start at 100%. The logarithmic zoom slider snaps within five percentage points of 100% during pointer interaction, while keyboard adjustments can leave actual size. Existing fractional-zoom painting and selection tests explicitly retain 75% coverage.

Tab close buttons occupy reserved space inside each tab and appear only on hover. The independent-tab test now clicks the inactive modified tab's hover-only close button and verifies its unsaved-changes prompt and retained document state. Layer properties render immediately beneath the selected layer with stable per-layer widget IDs. A separate three-layer native capture was visually reviewed with the middle layer selected, confirming that its properties appear before the bottom layer.

Formatting, strict Clippy, all 33 tests, the executable build and native launch passed again. The default 1386 × 963 capture was visually reviewed at 100%, the Windows baseline was updated, and the desktop screenshot regression passed against it. Platform validation remains Windows only.
