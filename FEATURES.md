# Implementation checklist

Current phase review: [ROADMAP.md](ROADMAP.md). Current validation: [Phase 4 slice](docs/PHASE4_VALIDATION.md); [Phase 3](docs/PHASE3_VALIDATION.md). Checked means implemented; historical validation details are in docs/VALIDATION.md, docs/PHASE2_VALIDATION.md, docs/LIVE_PAINT_TEXT_VALIDATION.md, docs/UI_BEHAVIOR_REFERENCE.md and docs/SHAPES_LAYERS_VALIDATION.md and docs/SELECTION_SHAPES_VALIDATION.md and docs/CONSTRUCTION_TRANSFORMS_VALIDATION.md. Model support alone is distinguished from end-user tools.

## Phase 0

- [x] Inspect empty repository and both reference screenshots.
- [x] Select and resolve current compatible Rust dependencies.
- [x] Document UI-independent, layered/nondestructive architecture.
- [x] Evaluate libheif and Rust overlay binding support from primary sources.
- [x] Inspect supplied `Untitled.paint`; verify brands, two source layers, iovl ordering/background/offsets and Exif.

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
- [x] New/default canvas 1280 x 720, retaining 100% initial zoom.
- [x] Vertical size/opacity/hardness controls and numerical adjustment.
- [x] Layer thumbnails and add/select/reorder/delete/duplicate/rename/visibility/lock/opacity; properties beneath the selected layer.
- [x] Reviewed Windows screenshots and native regression capture/comparison for twenty-five UI states, including Wand settings, selection options, masks and feathering.
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
- [x] Magic Wand, selection combination/feathering and basic live layer-mask property controls.
- [ ] Lasso/polygon/brush selections, mask painting and advanced style editing interfaces.

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
- [x] Reference 23-icon shape gallery in four rows; 23 functional tools with shared contours, antialiased fills and independent outline/fill.
- [x] Distinct right triangle, four/six-point stars, left/up arrows, callouts, heart and lightning.
- [x] Finite, bounded outline previews when shape width/height approaches zero.
- [x] Animated marching ants around selections, holes, creation and movement previews; cached topology and visible-dash clipping.
- [x] Background shape commitment, selection/lock/offset handling and Shift constraints.
- [x] Whole-layer move tool: live drag, axis constraint, nudging, cancel and one-command undo.
- [x] Free polygon construction with live next segment, close/double-click/Enter, Backspace and cancellation.
- [x] Two-bend cubic Curve tool with shared preview/commit geometry and early Enter commitment.
- [x] Finished shapes become selected and immediately usable by selection move/flip/rotate.
- [x] Selection-aware flips/quarter-turns and custom-angle rotation; whole-document fallback with layers/masks retained.
- [x] Eight selected-pixel resize handles and rotation handle; live move/resize/rotation previews, Shift constraints, Escape and one-command undo.
- [ ] Persistent editable vector shape objects.
- [ ] Brush variants, pressure, spacing and mask painting.
- [ ] Gesture-coalesced property editing and further dirty-region optimization.

## Phase 3

- [x] Real still AVIF read/write, alpha/quality/speed options and Windows codec tests.
- [ ] AVIF execution on macOS/Linux/Windows 10; matrix configured, not run locally.
- [x] `.paint` fixture analysis, genuine bounded HEIF/unci/iovl layered import/export and source/composite round trips.
- [x] Generated `.paint` opens and looks correct in Microsoft Paint (user confirmation).
- [ ] Independently edit/toggle both layers in Paint, save there and reopen in Palette; broader variants/platforms.
- [x] Native v3 binary assets with tile sharing, canvas backing color, bounded decoding, preserved ICC/Exif bytes and v1/v2 migration.
- [ ] Color-managed working profiles, richer metadata UI and future migrations.
- [x] Explicit handling of unsupported features in `.paint` export: native backup first, cancellable compatibility dialog and separate baked copy.
- [x] Persistent recent files, image-as-layer import, periodic recovery snapshots and restore/discard workflow.

## Phase 4

- [x] All 18 requested blend modes plus Divide; analytic/partial-alpha tests and reference sheet.
- [x] Magic Wand: Oklab/alpha tolerance, contiguous/all-matching, current/all-visible sampling and antialiasing.
- [x] Coverage-based Replace/Add/Subtract/Intersect for rectangle and Wand tools, including modifier shortcuts.
- [x] Selection and mask feathering; background Apply/Cancel transaction, source preservation and native saving.
- [x] Reveal-all or selection-derived layer masks, grayscale thumbnail, enable/disable, invert and delete.
- [x] Marching ants use 50% coverage boundaries while edits retain all fractional coverage.
- [ ] Merge selected/visible layers and confirmed flattening.
- [ ] Lasso, polygon and brush selection with configurable tolerance.
- [ ] Selection expand/contract and live feather previews.
- [ ] Brush/eraser mask editing, direct mask view, link/unlink and explicit permanent application.

## Phases 5–6

- [ ] Gaussian Blur and neighborhood filter stages; registry and common parameters.
- [ ] Apply/Cancel preview dialogs, individual filter mask UI and filter gallery.
- [ ] Drop/inner shadows, glows, bevel, stroke, satin and gradient/pattern overlays.
- [ ] Broader independently implemented filter catalog and research inventory.
- [x] Dirty brush regions and partial texture uploads for existing point-local processing.
- [ ] General dirty regions, neighborhood stages/caches, running-job cancellation and GPU compositor.
- [ ] Packaging, signing, accessibility and all-platform visual/behavior validation.
