# Implementation checklist

Checked means implemented; validation details are in docs/VALIDATION.md, docs/PHASE2_VALIDATION.md and docs/LIVE_PAINT_TEXT_VALIDATION.md and docs/UI_BEHAVIOR_REFERENCE.md. Model support alone is distinguished from end-user tools.

## Phase 0

- [x] Inspect empty repository and both reference screenshots.
- [x] Select and resolve current compatible Rust dependencies.
- [x] Document UI-independent, layered/nondestructive architecture.
- [x] Evaluate libheif and Rust overlay binding support from primary sources.
- [ ] Inspect missing `Untitled.paint`; verify reported brands/items/layers/metadata.

## Phase 1

- [x] Buildable native eframe/wgpu Windows executable and actual launch.
- [x] Dark menus/ribbon, Paint-style grouping, original geometric icons.
- [x] Slim document tabs: create, activate, reorder, integrated hover-only close, dirty state, shortcuts.
- [x] Independent documents, layers, zoom and undo history.
- [x] Functional native file dialogs, file drops, background image open/export.
- [x] PNG/JPEG/BMP/GIF/TIFF/WebP codec round trips.
- [x] Versioned `.palette` saving, atomic replacement, close prompts.
- [x] Canvas, checkerboard, smooth wheel zoom, pan, 100% default, actual-size slider snapping and fit.
- [x] Reversible canvas corner resize and coordinate status.
- [x] Vertical size/opacity/hardness controls and numerical adjustment.
- [x] Layer thumbnails and add/select/reorder/delete/duplicate/rename/visibility/lock/opacity; properties beneath the selected layer.
- [x] Reviewed Windows screenshots and native regression capture/comparison for seven UI states.
- [x] Inspect supplied Paint video; timed reference notes for menus, dialogs, inline text and layers.
- [x] Compact split-button flyouts, aligned shortcuts, original stroke samples, hover/press transitions and animated layer tray.
- [x] Centered transactional color/image-property dialogs with compact-window scrolling.
- [x] Contextual sliders; View grid/status/ribbon/layers toggles and zoom preset menu.
- [ ] Exact native menu reveal/easing and full keyboard-focus parity.
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
- [x] Rectangular selection interface with clear, inversion and selected pixel movement.
- [ ] Advanced selection, mask and style editing interfaces.

## Phase 2

- [x] Contiguous fill with tolerance and opacity; source transparency/selection/lock handling.
- [x] System clipboard image copy/cut/paste and native image-paste events.
- [x] Rectangular selection, all/deselect/invert, clear, selected painting/shapes and move/nudge.
- [x] Crop, canvas bounds resize, nearest/bilinear resampling, rotate and flip.
- [x] Transforms retain layers, masks and live filter/effect properties.
- [x] Stroke-wide maximum coverage prevents opacity buildup at pointer joins.
- [x] History counts unique retained tile/mask buffers and caps command count.
- [x] Background edit jobs reject stale results and validate geometry allocation limits.
- [x] Live brush/pencil/eraser region compositing and partial texture uploads; progressive render fallback.
- [x] Inline text with floating formatting strip, eight resize handles and move edge, two fonts, layout/color controls and bounded rasterization.
- [x] Text Done/outside-click commitment, Cancel/Escape, queued document/tool actions and one-command raster-layer insertion.
- [ ] Exact interactive/committed text metrics, selection-preview parity and global shortcuts during typing.
- [ ] System font discovery, additional faces, complex shaping and persistent editable text objects.
- [ ] More shapes and whole-layer movement UI.
- [ ] Brush variants, pressure, spacing and mask painting.
- [ ] Gesture-coalesced property editing and further dirty-region optimization.

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
- [x] Dirty brush regions and partial texture uploads for existing point-local processing.
- [ ] General dirty regions, neighborhood stages/caches, running-job cancellation and GPU compositor.
- [ ] Packaging, signing, accessibility and all-platform visual/behavior validation.
