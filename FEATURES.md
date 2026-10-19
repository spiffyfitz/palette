# Implementation checklist

Current phase review: [ROADMAP.md](ROADMAP.md). Current validation: [noise cleanup, distortions and sharpening](docs/RETOUCH_DISTORT_VALIDATION.md); [blur/render/artistic/texture filters](docs/CREATIVE_FILTERS_VALIDATION.md); [conventional menus and ten more filters](docs/FILTER_MENU_VALIDATION.md); [style controls and six more filters](docs/STYLE_FILTER_CONTROLS_VALIDATION.md); [expanded processing catalog](docs/PROCESSING_CATALOG_VALIDATION.md); [layer controls and canvas previews](docs/LAYER_PROCESSING_UI_VALIDATION.md); [Phase 5 live processing](docs/PHASE5_VALIDATION.md); [brush selection](docs/BRUSH_SELECTION_VALIDATION.md); [selection/navigation](docs/SELECTION_NAVIGATION_VALIDATION.md); [Phase 4 slice](docs/PHASE4_VALIDATION.md); [Phase 3](docs/PHASE3_VALIDATION.md). Checked means implemented; historical validation details are in docs/VALIDATION.md, docs/PHASE2_VALIDATION.md, docs/LIVE_PAINT_TEXT_VALIDATION.md, docs/UI_BEHAVIOR_REFERENCE.md and docs/SHAPES_LAYERS_VALIDATION.md and docs/SELECTION_SHAPES_VALIDATION.md and docs/CONSTRUCTION_TRANSFORMS_VALIDATION.md. Model support alone is distinguished from end-user tools.

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
- [x] Reviewed Windows screenshots and native regression capture/comparison for forty-eight UI states, including Wand settings, selection options, masks and feathering.
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
- [ ] Polygon selection, mask painting and remaining advanced style editing interfaces.

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
- [x] Native v6 binary assets with tile sharing, canvas backing color, bounded decoding, preserved ICC/Exif bytes, original feather contours and v1/v2/v3/v4/v5 migration.
- [ ] Color-managed working profiles, richer metadata UI and future migrations.
- [x] Explicit handling of unsupported features in `.paint` export: native backup first, cancellable compatibility dialog and separate baked copy.
- [x] Persistent recent files, image-as-layer import, periodic recovery snapshots and restore/discard workflow.
- [x] Detected GIF/WebP/APNG/TIFF import limitations reach both open tabs and image-as-layer imports; real first-frame/page fixtures verify the pixels.

## Phase 4

- [x] All 18 requested blend modes plus Divide; analytic/partial-alpha tests and reference sheet.
- [x] Magic Wand: Oklab/alpha tolerance, contiguous/all-matching, current/all-visible sampling and antialiasing.
- [x] Coverage-based Replace/Add/Subtract/Intersect for rectangle, freeform and Wand tools, including modifier shortcuts.
- [x] Selection and mask feathering; background Apply/Cancel transaction, source preservation and native saving.
- [x] Reveal-all or selection-derived layer masks, grayscale thumbnail, enable/disable, invert and delete.
- [x] Original selection ants plus separate 40%-opacity feather-extent ants; inverted selection continues around visible canvas edges.
- [x] Horizontal/vertical canvas scrollbars outside image hit testing and clear of the layers tray; bounded pan, keyboard controls and Fit handling.
- [ ] Merge selected/visible layers and confirmed flattening.
- [x] Freeform/lasso selection with live closed ants, antialiased coverage, combine modes and undo.
- [x] Canvas selection toolbar matching text-bar height; dynamic width, precise feather input and Wand tolerance.
- [x] Edge-aware brush selection with configurable tolerance, feather, strength/hardness/size, continuous live coverage preview, sampling options and plain coverage mode.
- [ ] Polygon selection.
- [x] Live selection feather previews in the toolbar and dialog; release/Apply commits one undo step, Escape/Cancel discards.
- [ ] Selection expand/contract.
- [ ] Brush/eraser mask editing, direct mask view, link/unlink and explicit permanent application.

- [x] Feathering below 5 px displays one decimal, supports real subpixel coverage changes, and shares mild shifted-log fine-adjustment sliders with Wand tolerance. Outer feather ants use 40% opacity.

## Phase 5 — live processing

- [x] Gaussian Blur with alpha-safe tile/halo processing (0–64 px); registry and common numeric/color parameters.
- [x] Background on-canvas Apply/Cancel previews (full size through 2048 px); obsolete jobs cancel and cannot affect other tabs.
- [x] Three-icon Filters/Effects/Blend row beneath layer Opacity; contextual processing toolbars share Text/Selection height and fence canvas input. All style parameters remain accessible through a compact selector.
- [x] Filter/effect stack edit/toggle/reorder/duplicate/delete, plus selection-derived individual filter-mask controls.
- [x] Live Drop/Inner Shadow, Outer/Inner Glow, Stroke (outside/inside/center) and Color Overlay; source/mask updates and layer opacity/blending.
- [x] Brightness/Contrast, linear-light Exposure, Hue/Saturation, Vibrance, Black & White, Threshold, Posterize, Box Blur and Median, plus existing Invert/Brightness/Gaussian.
- [x] Researched conventional Filter hierarchy with 13 branches, disabled unfinished filters, disabled first Filter Gallery entry, and no correction/plugin sections.
- [x] Registry-driven Filter availability, scrollable Add lists and common numeric/color/boolean/choice controls; validated integer settings.
- [x] Independent neighborhood/adjustment references, source/alpha/style-placement tests and native/Paint round trips across all 148 implemented operations (142 filters and six styles).
- [x] Native preservation, history and compatible Paint export baking with the existing native-backup guard.
- [x] Bounded evaluated-layer LRU, background thumbnails, tiled scratch and region/full-composite correctness tests.
- [x] Windows native dialog captures and representative CPU timings; exact limits in docs/PHASE5_VALIDATION.md.
- [ ] Filter Gallery and remaining layer styles, including bevel, satin and gradient/pattern overlays.
- [x] Conventional-filter implementation inventory and explicit scope/algorithm limits in docs/PROCESSING_CATALOG_VALIDATION.md.
- [x] Levels, five-point Curves, Motion Blur, High Pass, Mosaic and seeded monochrome/color Uniform/Gaussian Add Noise.
- [x] Despeckle, Dust & Scratches, Reduce Noise with separate luminance/chroma controls, Pinch, Spherize, Ripple, Sharpen and Sharpen More.
- [x] Surface Blur, Spin/Zoom Radial Blur, seeded Clouds/Difference Clouds with saved colors, Poster Edges, five Grain patterns and Fragment.
- [x] Average, Find Edges, Emboss, Solarize, Unsharp Mask, Minimum, Maximum, Offset, Twirl and Wave, with source/alpha/sampling/cancellation references.
- [x] Glow Radius, shadow Angle/Distance/Spread and Stroke Width/Rounded corners controls; older native projects retain square strokes.
- [x] Mouse-positioned source-local origins for Twirl, Wave, Radial Blur, Pinch, Spherize, Ripple, Polar Coordinates and ZigZag; live crosshair, numeric X/Y, reset, Apply/Cancel and native v6 migration.
- [x] Polar Coordinates in both directions, three ZigZag styles, seeded two-color Fibers and Glowing Edges with width/brightness/smoothness.
- [x] Cutout (bounded color palette), Film Grain, Accented Edges, Ink Outlines, two-color Photocopy/Halftone Pattern, Patchwork and seeded Craquelure; live controls, source-alpha preservation and native/Paint round trips.
- [x] All seven Pixelate entries, Stained Glass, Mosaic Tiles and four Diffuse modes; bounded tiled cell colors, seeded marks, real screen angles, live controls and source/alpha preservation.
- [x] Registry-generated filter inventory: 142 of 142 catalog entries implemented (135 conventional plus seven limited local model-based counterparts; catalog coverage does not establish Photoshop parity); Filter Gallery and layer styles counted separately.
- [x] Current conventional catalog, including Color Lookup, Displace, Shear and Custom convolution with actual import/editor workflows. Additional filters can extend the registry.
- [x] Dirty brush regions and partial texture uploads for existing point-local processing.
- [ ] General dirty regions, per-stage/tile caches and GPU compositor.

- [x] Color Balance, Photo Filter, Channel Mixer, two-stop Gradient Map, nine-range Selective Color and local Shadows/Highlights; removable live adjustment stacks.
- [x] Fixed Blur/Blur More, alpha-safe local tone guides, saved gradient colors and idle percentage-preview regression.

- [x] Colored Pencil, Dry Brush, three Paint Daubs geometries, Smudge Stick, Angled Strokes, Crosshatch, Dark Strokes and Spatter; editable source-preserving controls, saved paper color/seeds and bounded neighborhoods.

- [x] All eight current Brush Strokes entries, including directional seeded Sprayed Strokes and wet-color Sumi-e.
- [x] Bas Relief, Chalk & Charcoal, Charcoal, Graphic Pen, Note Paper and Stamp; saved ink/paper colors, four stroke axes, editable lighting and source-alpha preservation.

- [x] All 14 current Sketch entries, including Chrome, Conté Crayon, Plaster, Reticulation, Torn Edges and Water Paper; live settings, original-alpha/source preservation and native project round trips.
- [x] Generated Canvas/Brick/Burlap/Sandstone surfaces with stored size, relief, light angle, inversion and seed; no proprietary texture assets.

- [x] All 15 current Artistic entries, including Fresco, Neon Glow, Palette Knife, Plastic Wrap, Rough Pastels, Sponge, Underpainting and Watercolor; editable material/lighting/color controls, original-alpha/source preservation and native round trips.

- [x] All five Sharpen and six Texture entries, plus Diffuse Glow, Glass, Ocean Ripple, Shape Blur and three-mode Smart Blur; source-preserving live stacks, alpha-correct warps/aperture blur, saved materials/colors/iterations and three additional mouse origins.

- [x] All ten current Stylize, eleven Blur and five Render entries, with independent Extrude/Oil Paint/Tiles/Trace Contour/Wind, aperture/depth Lens Blur, four Lens Flare styles and Point/Spot/Directional Lighting Effects; live nondestructive settings, three additional mouse origins and contextual inactive controls.

- [x] Strict 3D `.cube` import, bounded raster displacement-map import, per-draft background jobs, embedded binary filter assets, shared LUT/map accounting in history/cache, draggable shear curve and editable 5×5 kernel popup; live previews, Apply/Cancel, independent native reopening and malformed-asset tests.

- [x] Five Blur Gallery filters with movable focus/path controls; original Flame/Tree/Picture Frame renderers; Video field/chroma operations and HSB/HSL conversion.
- [x] Dated Adobe coverage audit separating the 133 conventional filters from exclusions and unavailable AI functionality.
- [ ] Remaining Neural/AI filters and broader quality/platform validation (Mosaic Style Transfer is locally implemented). See docs/PHOTOSHOP_FILTER_AUDIT.md.

- [ ] Substance-based Parametric Filters: version-specific effect inventory, original/licensed materials and independently validated implementations.

- [x] Isolated Rust feasibility probe: checksum-pinned learned style transfer and 3× luminance super resolution execute successfully. This is a development tool, not an app feature.
- [ ] First six Neural Filter counterparts: model selection/terms, numerical and image-quality references, alpha/geometry adapters, runtime scheduling, native/UI integration. See docs/NEURAL_FILTERS_FIRST_SIX.md.

- [x] Bundled offline Mosaic Style Transfer: local Rust inference, immutable model profile, exact checksum, independent CPU reference, cancellable operators, source/alpha preservation, strength/cache, native roundtrip and compact live toolbar.
- [ ] Smart Portrait, remaining first-batch capability gaps and high-resolution/custom-reference style models. See docs/NEURAL_FILTERS_FIRST_SIX.md.

- [x] Bundled local JPEG Artifacts Removal: full-resolution overlapping DnCNN-3 tiles, alpha/chroma preservation, cached strength edits, masks and native/UI transactions.
- [x] Main render tickets cancel obsolete generations/tab targets and resubmit cancelled tabs when revisited; app drop cancels render/preview work.

- [x] Bundled local Colorize: compact ECCV16 chrominance inference, Lab/source-alpha adapter, live strength/saturation controls, mask/native preservation, independent photo and numerical references.
- [x] Local face-guided Skin Smoothing: bundled BlazeFace/Face Mesh geometry,
  conventional smoothing with protected features, live Strength/Blur/Smoothness,
  alpha/masks/native/undo preservation and independent model/photo references.
- [x] Local Super Zoom cropped-output slice: real 3× inference with safe tiled
  halos, 1–3× magnification, Enhancement, draggable source center, geometric
  alpha, masks/native/undo and independent full-image seam references.
- [x] Whole-image Super Zoom PNG export: 1–3× larger output, all visible layers,
  local background inference, cancellation and atomic replacement; source project stays editable.
- [ ] Enlarged editable Super Zoom documents and related painting/selection/mask coordinate mapping.
- [ ] Full Photoshop filter parity: tracked separately from catalog counts in
  docs/PHOTOSHOP_FILTER_AUDIT.md; partial implementations do not satisfy it.
- [ ] Smart Portrait and remaining limitations of the five partial first-six counterparts; remaining learned later-group capabilities and Filter Gallery remain deferred.

- [x] Local Depth Blur: bundled inverse-depth model, click/manual focus, focal
  range/radius, depth-map view, live editable stack/masks/native/history and
  independent model/photo references. Gaussian defocus is not optical or neural reblurring.

- [x] Statistical Color Transfer in Neural Filters: original warm palette,
  local reference import, alpha-weighted Lab profiles, live strength/luminance/
  contrast/saturation, source/mask/history preservation and portable native profiles.
  This is a partial statistical counterpart, not a completed learned Color Transfer filter.

- [x] Face-guided Makeup Transfer in Neural Filters: original rose/plum default,
  reference-portrait import, separate lips/eyes/strength/softness/luminance,
  protected eye/mouth interiors, editable masks/history and portable native profiles.
  Learned landmarks guide conventional colour matching, not texture synthesis.

- [x] Spatial Makeup Transfer v2: reference atlas, gradients/liner/highlights,
  cheeks, occlusion protection and binary asset/native preservation; legacy v1 retained.
- [x] Live Harmonization: typed reference-layer picker, dependency rendering/cache,
  cycle rejection and shared clipboard/selection/Paint appearance.
- [x] Photo Restoration: legacy thin-scratch preservation plus multiscale curved/wide
  cracks, 0–200% Strength, 1–64 px radius, polarity, optional local facial-detail
  protection, harmonic fill, damage overlay and editable native/mask/history
  preservation. Learned face reconstruction remains absent.

## Phase 6

- [ ] Packaging, signing, accessibility and all-platform visual/behavior validation.
