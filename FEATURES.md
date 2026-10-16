# Implementation checklist

Checked means implemented; validation details are in docs/VALIDATION.md. Model support alone is distinguished from end-user tools.

## Phase 0

- [x] Inspect empty repository and both reference screenshots.
- [x] Select and resolve current compatible Rust dependencies.
- [x] Document UI-independent, layered/nondestructive architecture.
- [x] Evaluate libheif and Rust overlay binding support from primary sources.
- [ ] Inspect missing `Untitled.paint`; verify reported brands/items/layers/metadata.

## Phase 1

- [x] Buildable native eframe/wgpu Windows executable and actual launch.
- [x] Dark menus/ribbon, Paint-style grouping, original geometric icons.
- [x] Slim document tabs: create, activate, reorder, close, dirty state, shortcuts.
- [x] Independent documents, layers, zoom and undo history.
- [x] Functional native file dialogs, file drops, background image open/export.
- [x] PNG/JPEG/BMP/GIF/TIFF/WebP codec round trips.
- [x] Versioned `.palette` saving, atomic replacement, close prompts.
- [x] Canvas, checkerboard, smooth wheel zoom, pan, actual size and fit.
- [x] Reversible canvas corner resize and coordinate status.
- [x] Vertical size/opacity/hardness controls and numerical adjustment.
- [x] Layer thumbnails and add/select/reorder/delete/duplicate/rename/visibility/lock/opacity.
- [x] Reviewed Windows screenshot and automated regression capture/comparison.
- [ ] Full keyboard accessibility audit and additional DPI/tablet testing.
- [ ] Windows 10, macOS and Linux launch validation.

## Early working editing slices

- [x] Continuous round brush, pencil, soft eraser, picker and magnifier.
- [x] Lines, rectangles and ellipses with preview and outline/fill.
- [x] Command undo/redo for source pixels and layer/filter/property mutations.
- [x] Normal, Multiply and Screen with reference alpha tests.
- [x] Invert and Brightness live filters: edit/toggle/reorder/delete/duplicate/save.
- [x] Core grayscale selections, layer masks and filter masks; serialization/compositing tests.
- [x] Core nondestructive Color Overlay style; native round-trip preservation.
- [ ] Mask/selection/style editing interface (core support is not a finished workflow).

## Phase 2

- [ ] Fill, text, clipboard image import/export, selection and crop.
- [ ] Resize/resample, rotate, flip, move selection/layers and more shapes.
- [ ] Brush variants, pressure, spacing, stroke-wide opacity and mask painting.
- [ ] Gesture-coalesced property edits and dirty-tile history accounting.

## Phase 3

- [ ] AVIF real read/write/alpha/quality tests on each target before enabling.
- [ ] `.paint` fixture analysis, layered import, real export and Microsoft Paint validation.
- [ ] Native format streamed assets, metadata/profiles and migrations.
- [ ] Explicit handling of unsupported features in `.paint` export.
- [ ] Recent files, image-as-layer, autosave and recovery.

## Phases 4–6

- [ ] Remaining Photoshop-style blend modes and merge/flatten.
- [ ] Coverage-based lasso/polygon/wand/brush selections and combination modes.
- [ ] Selection feather/expand/contract; complete dynamic mask workflows.
- [ ] Gaussian Blur and neighborhood filter stages; registry and common parameters.
- [ ] Apply/Cancel preview dialogs, individual filter mask UI and filter gallery.
- [ ] Drop/inner shadows, glows, bevel, stroke, satin and gradient/pattern overlays.
- [ ] Broader independently implemented filter catalog and research inventory.
- [ ] Dirty regions, stage caches, cancellation, partial texture uploads and GPU compositor.
- [ ] Packaging, signing, accessibility and all-platform visual/behavior validation.
