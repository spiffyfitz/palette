# Palette

A native Rust raster editor with a Paint-style ribbon and a UI-independent, layered editing engine. Core Paint editing, Phase 3 file workflows, the first Phase 4 layer/selection slice and a Phase 5 live-processing slice are implemented. The user confirmed the generated `.paint` opens and looks correct in Microsoft Paint; broader interoperability and cross-platform validation remain tracked in the [roadmap](ROADMAP.md).

## Run

Install stable Rust **1.98 or newer**, then:

```sh
cargo run --locked
# Open a file from the command line:
cargo run --locked -- path/to/image.png
```

Windows needs the MSVC C++ build tools and Windows SDK. macOS needs Xcode Command Line Tools. On Linux, install a C/C++ toolchain, pkg-config, X11/Wayland development packages, a working Vulkan driver, an XDG desktop portal backend and Zenity. Neural model runtimes, AVIF codecs and NASM are not required. See [build notes](docs/BUILD.md) and the [dependency audit](docs/DEPENDENCY_AUDIT.md).

## Current scope

Neural Filters and AVIF support were intentionally removed on 2026-10-20 to reduce dependencies, binary size and build requirements. All twelve operations formerly under Neural Filters are removed, including conventional approximations. PNG, JPEG, BMP, GIF, TIFF, WebP, `.paint` and `.palette` remain supported. Older projects containing a retired filter fail with an explicit diagnostic, including disabled filters; open them in an older build and remove or rasterize those filters before saving a compatible copy. No filter is silently discarded.

## Working now

Menus, Save/Undo/Redo and open-document tabs share a compact top row. Overflowing tabs scroll horizontally, keyboard switching reveals the active tab, and dragging near an edge scrolls to more tabs. Native window controls retain their platform placement.

New documents use a **1280 x 720** white canvas at **100% zoom**. Image Properties > Default uses the same size. Use Fit or pan when the canvas is wider than the workspace.

- Paint-style dark ribbon, split buttons, anchored flyouts, hover/press animation, compact document tabs, animated layers tray, three contextual brush controls and status bar. Narrow windows progressively fold Shapes, Image, Tools and Colors into functional section flyouts; Shapes first becomes a scrollable gallery.
- Centered Edit Colors and Image Properties dialogs with OK/Cancel, custom color slots, exact RGB/hex input and reversible canvas bounds changes. View toggles for gridlines, status bar, ribbon and layers.
- New/open documents; switch, drag to reorder and close tabs using an integrated hover-only button; independent zoom, layers and history; modified-document close prompts matching the supplied Paint dialog, including keyboard focus and Save/Don’t save/Cancel.
- Pencil, nine working brush styles, soft eraser, color picker and magnifier. Brushes include a round, speed-sensitive Calligraphy Brush, a fixed-nib Calligraphy Pen and round Marker, stationary Airbrush spray, Crayon with fresh mottled wax, sparse paper gaps, live buildup and fading sides, fine granular Natural Pencil and mixing Oil/Watercolor. The flyout shows samples from the actual rasterizer; the left controls adjust size, opacity and hardness. Each brush remembers its own size, starting at the reference diameter. Oil has richer bristle tracks and Watercolor a softer wash; both taper as long strokes run dry. All 23 reference shape-gallery tools are functional, including free Polygon and a two-bend cubic Curve. Shapes have independent foreground outline/background fill, opacity, width, drag previews and Shift constraints. Finished shapes remain selected as isolated raster content: moving, resizing, rotating or flipping them does not lift the image beneath their bounding box. Click outside to accept; Escape/Delete removes the pending shape. Right mouse swaps drawing colors. [Shape adjustment](docs/FLOATING_SHAPES_VALIDATION.md).
- Size, opacity and hardness controls, including exact numerical input and a live viewport-centered size preview. Erasing reduces alpha. [Brush size/paint-load behavior](docs/BRUSH_SIZE_PAINT_LOAD.md).
- Brush, pencil and eraser strokes appear while dragging, with partial canvas texture updates and one undo command per stroke.
- Text: installed system fonts appear in a scrollable picker with stable font-name previews and a highlighted selection; symbol fonts keep readable labels. Type inline, resize with eight handles, move by the border, or rotate with the round handle (Shift snaps to 15°). An internal scrollbar appears for overflowing text; enlarging the box reveals the retained draft. Size, wrapping, alignment icons, line spacing, colors and opacity work. Done, Ctrl/Command+Enter or clicking outside commits the visible viewport to a separate raster layer; Cancel/Escape leaves pixels unchanged. [Text-tool scope and validation](docs/TEXT_TOOL_VALIDATION.md).
- Bold, Italic, Underline and Strikethrough format the highlighted text and can be combined; with no selection they affect new typing. Use Ctrl/Command+B, I, U, or Ctrl/Command+Shift+X for strikethrough. Installed style faces are used where available; fallback styling runs locally. Style clicks preserve the visible highlight. Draft undo restores text and formatting together; commitment inserts one undoable raster layer.
- Add, duplicate, rename, show/hide, lock, reorder and delete layers; properties directly beneath the selected layer; opacity; all 18 requested blend modes plus Divide, with correct alpha compositing and native project preservation.
- Choose **Move** in the ribbon's Tools group: it moves selected pixels when a selection exists, otherwise the active layer. Drag anywhere on the canvas, Shift to constrain an axis, arrows to nudge (Shift for ten pixels), Escape to cancel. One drag is one undo command. Eligible raster selections, pending shapes and whole layers use cached GPU previews; selections with live processing retain CPU previews for correctness.
- Horizontal/vertical canvas scrollbars appear only on overflowing axes, in workspace gutters clear of the layers panel. Drag their thumbs, click the rails, or use arrows/Page Up/Page Down/Home/End while focused. Fit removes the bars.
- Pan with middle mouse or Space+drag; scroll to pan; Ctrl/Command+wheel to zoom around the pointer; 100% default zoom with slider snapping at actual size, fit and canvas corner resize.
- Native open/save dialogs, file drop, atomic `.palette` v8 saving with shared binary assets and migration of existing v1/v2/v3/v4/v5/v6/v7 projects. PNG, JPEG, BMP, GIF, TIFF and WebP have real codec tests.
- File > Import to canvas adds a raster as a separate layer with one undo command. Recent files persist across runs. Recovery copies of committed edits are made in the background approximately every 30 seconds; the next launch offers Restore, Discard or Later without overwriting original files.
- Three original icon buttons beneath layer Opacity open Filters, Effects and Blending mode. Filters/effects use a compact toolbar at the top of the canvas, matching the Text/Selection bar height. Choose a stack instance, Add/Edit, toggle On, or use the … menu to reorder, duplicate, remove or manage a filter mask. The blend icon opens the existing mode catalog. Drag filters/effects directly in the layer panel, or drag layer thumbnails, to rearrange them with an insertion line, translucent preview and animated neighbors. Each drop is undoable; long layer lists scroll at the viewport edge. [Reordering validation](docs/STACK_REORDER_VALIDATION.md).
- Filter > Blur > Gaussian Blur opens the same canvas toolbar with precise radius input. Parameter edits preview directly on the image; choose other settings from the parameter selector. Apply commits editable settings, Cancel/Escape restores the committed view, and Preview switches comparison on/off. Invert and Brightness share the nondestructive filter stack. [Layer control validation](docs/LAYER_PROCESSING_UI_VALIDATION.md).
- Live adjustments: Brightness/Contrast, Exposure, Hue/Saturation, Vibrance, Black & White, Threshold and Posterize, alongside Invert and legacy Brightness. Levels and five-point Curves are editable too. Blur offers Gaussian, Box and Motion Blur; Noise offers Median and seeded Add Noise (uniform/Gaussian, monochrome/color). Pixelate offers Mosaic; Other offers High Pass. Conventional Filter branches show unfinished entries disabled; active entries and toolbar Add menus resolve the implemented registry.
- More conventional filters: Average, Find Edges, Emboss, Solarize, Unsharp Mask, Minimum, Maximum, Offset, Twirl and Wave. Filter Gallery is first in the menu and disabled for now; lens correction, Liquify, Vanishing Point and plugin sections are omitted. [Menu and filter validation](docs/FILTER_MENU_VALIDATION.md).
- Surface Blur, Spin/Zoom Radial Blur, editable seeded Clouds/Difference Clouds, Poster Edges, five Grain patterns and Fragment. Clouds captures foreground/background colors at creation; all remain removable live filters. [Creative filter validation](docs/CREATIVE_FILTERS_VALIDATION.md).
- Noise cleanup: Despeckle, Dust & Scratches and Reduce Noise (strength, detail protection, color cleanup and sharpening). Distort adds Pinch, Spherize and Ripple; Sharpen and Sharpen More are also functional. [Validation and limits](docs/RETOUCH_DISTORT_VALIDATION.md).
- Polar Coordinates (both directions), ZigZag (three styles), seeded two-color Fibers and Glowing Edges. Twirl, Pinch, Spherize, Ripple, Wave, Radial Blur and both new distortions have movable origins: use the four-arrow placement button, then click/drag on the layer. Origin X/Y provide numeric percentages; … > Reset origin restores the default. Origins preview live and remain editable in saved projects. [Validation and exact scope](docs/ORIGIN_FILTERS_VALIDATION.md).
- Artistic: Cutout and Film Grain; Brush Strokes: Accented Edges and Ink Outlines; Sketch: Photocopy and Dot/Line/Circle Halftone Pattern; Texture: Patchwork and seeded Craquelure. All are editable live filters. Sketch ink/paper colors capture drawing colors when added and remain saved parameters. [Algorithms and validation](docs/ARTISTIC_FILTERS_VALIDATION.md).
- Pixelate now has working Color Halftone, Crystallize, Facet, Mezzotint and Pointillize alongside Mosaic/Fragment. Texture adds Stained Glass and Mosaic Tiles; Stylize adds Normal/Darken/Lighten/Anisotropic Diffuse. All remain editable live filters. [Validation](docs/PIXELATE_FILTERS_VALIDATION.md). The current catalog has **132 implemented filters**, with the retired Neural Filters excluded; [generated inventory](docs/FILTER_INVENTORY.md).
- Adjustments now also includes Color Balance, Photo Filter, Channel Mixer, Gradient Map, Selective Color and Shadows/Highlights. Gradient Map captures drawing colors for new instances; all nine Selective Color ranges remain editable together. Blur and Blur More are functional fixed-kernel commands. [Algorithms, controls and validation](docs/ADJUSTMENT_FILTERS_VALIDATION.md).
- Artistic adds Colored Pencil, Dry Brush, Paint Daubs and Smudge Stick; Brush Strokes adds Angled Strokes, Crosshatch, Dark Strokes and seeded Spatter. Editable paper color/brightness, pressure, texture, brush geometry and tonal stroke controls use the existing live toolbar. [Algorithms and validation](docs/PAINTING_FILTERS_VALIDATION.md).
- Brush Strokes is complete for the current menu: Sprayed Strokes and Sumi-e join the six earlier entries. Sketch adds Bas Relief, Chalk & Charcoal, Charcoal, Graphic Pen, Note Paper and Stamp, with saved drawing colors, stroke directions and editable relief lighting. [Algorithms and validation](docs/DRAWING_FILTERS_VALIDATION.md).
- Sketch now includes all 14 current entries: Chrome, Conté Crayon, Plaster, Reticulation, Torn Edges and Water Paper complete the branch. Conté Crayon has generated Canvas/Brick/Burlap/Sandstone materials, size, lighting, relief and inversion; Chrome stays grayscale and Water Paper retains source colors. [Algorithms and validation](docs/SKETCH_COMPLETION_VALIDATION.md).
- Live Drop Shadow, Outer Glow, Inner Shadow, Inner Glow, Stroke and Color Overlay with editable parameters. Stroke supports outside, inside and centered placement, with subpixel width and a Rounded corners checkbox. Glow opens on Radius; shadows open on Angle with Distance/Spread fields alongside in wider windows. Source edits automatically update their appearance; source pixels stay intact. Full editable settings survive `.palette` saving and undo. [Current catalog, style controls and performance limits](docs/STYLE_FILTER_CONTROLS_VALIDATION.md).
- Polygon: click vertices with a live next-segment line; Enter, double-click or click the first vertex closes it. Backspace removes a vertex; Escape cancels. Curve: drag a baseline, then drag two bends; Enter can finish early.
- Eight selection resize handles and a rotation handle, with live processed-pixel previews for move/resize/rotation. Shift locks resize proportions, constrains movement or snaps rotation to 15 degrees; Escape cancels. A completed gesture is one undo command.
- Selection brush in the Selection menu: edge-aware color matching or plain coverage painting, tolerance/feather controls, left-side size/strength/hardness, all four combine modes, Alt subtraction, a cyan coverage overlay and one undo step per stroke. Preview updates while held; Escape cancels. [Behavior and validation](docs/BRUSH_SELECTION_VALIDATION.md).
- Animated marching-ants outlines around selections, including holes, moving selections and creation previews. Inverted selections retain a visible outer boundary even when zoom clips the actual canvas edges. Feathered selections keep opaque ants on the original contour and a separate 40%-opacity outline at the feather extent.
- Rectangular, freeform/lasso, polygonal and Magic Wand selections, select all/deselect/invert, selection-constrained painting/erasing/shapes, clear and selection-size status. Selection > Combine selection offers Replace/Add/Subtract/Intersect; Shift adds, Alt subtracts and Shift+Alt intersects. In Replace mode, drag selected pixels or handles to transform; Shift constrains a handle gesture. Arrow keys nudge, Shift+arrows move ten pixels. Wand options provide perceptual tolerance, contiguous/all-matching colors, active/all-visible sampling and antialiasing.
- Selection tools show a compact toolbar above the canvas with Replace/Add/Subtract/Intersect icons, inversion, and a feather slider plus numeric field (0–128 image pixels, displayed to one decimal below 5 px). Wand adds tolerance (0–100) and sampling options. Both sliders use a mild shifted-log curve (midpoint about 23–27 units) for finer low-value adjustment; exact fields remain available. The toolbar uses the text strip’s height and fits its contents. Feather previews an existing selection live while dragging and commits one undo step on release; Escape cancels. It also sets the radius applied to new regions before combining. Freeform selection closes on release, has a live ants preview and supports Escape cancellation and undo. Polygonal selection places vertices by clicking; Enter, double-click or clicking the starting point finishes, Backspace removes a vertex and Escape cancels. It shares feather/combine controls and produces one undoable coverage selection. [Validation](docs/POLYGON_SELECTION_VALIDATION.md).
- Selection > Feather selection previews grayscale coverage relative to the retained original; Apply commits and Cancel discards the draft. Radius zero restores the original. Original contours survive undo, transforms and native saving. Higher Wand tolerance includes more colors; active-layer versus all-visible sampling remains selectable. Layer properties can add a reveal-all mask or create one from selection, then enable/disable, invert, feather or delete it. Source pixels remain unchanged; mask operations support undo and native saving. See [Phase 4 validation](docs/PHASE4_VALIDATION.md).
- Contiguous fill with adjustable tolerance (0–255) and opacity. Right mouse uses the background color.
- System image clipboard: Copy Layer (Ctrl/Command+C) copies the active layer; Copy Image (Ctrl/Command+Shift+C) copies the visible composite. With a selection, both crop to its bounding box and preserve freeform/polygon coverage and feathering; without one they copy the full canvas area. The selection right-click menu offers Copy and Copy visible layers using those same shortcuts. Both retain live processing in the copied appearance. Cut remains selection-aware. Paste creates a new layer and expands the canvas if needed.
- Crop canvas to selection, horizontal/vertical flip, exact quarter-turns and custom-angle rotation. A selection limits flips/rotations to active-layer selected source pixels; without a selection they transform the whole document. Custom rotation uses alpha-safe bilinear sampling and an OK/Cancel dialog. Resize offers nearest or bilinear sampling with optional aspect lock; layers and live processing stacks are retained.
- Consistent stroke opacity across pointer events/retracing, tile-sharing-aware undo accounting, and background fill/geometry jobs with stale-result protection.

Unavailable ribbon tools and filter categories are visibly disabled. **Persistent vector shape objects, mask painting and per-effect masks remain unavailable.** Text supports basic left-to-right layout with two bundled fonts and becomes raster content on Apply; system font browsing and complex-script shaping remain pending. The supplied `.paint` specimen opens as two editable layers. File > Export offers an experimental genuine HEIF `.paint` copy; masks/blends/live processing require a complete `.palette` save before compatibility conversion. The user confirmed the generated copy opens and looks correct in Microsoft Paint. Editing there and saving back remain separate unverified checks; see [specimen evidence and limits](docs/PAINT_RESEARCH.md). Broader HEIF properties and complete profile-aware processing remain pending. Native projects retain original ICC/Exif bytes; flattened exports do not preserve metadata.

Save/Save As preserve editable state in `.palette`. Export writes a composite image and does not mark the editable project saved. GIF imports its first frame; export does not create an animation. Animated WebP/APNG and TIFF imports display first-frame/default-image/first-page limitations, including when imported as a layer. JPEG composites transparency over white.

## Shortcuts

| Action | Shortcut |
|---|---|
| New / Open / Save | Ctrl+N / Ctrl+O / Ctrl+S |
| Save As / Close tab | Ctrl+Shift+S / Ctrl+W |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y |
| Next / Previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Copy Layer / Copy Image (visible layers) | Ctrl+C / Ctrl+Shift+C |
| Cut / Paste image | Ctrl+X / Ctrl+V |
| Select all / Deselect | Ctrl+A / Ctrl+D or Escape |
| Clear selected pixels | Delete / Backspace |
| Move selected pixels | Arrows; Shift+arrows for ten pixels |

Use Command in place of Ctrl for file/edit shortcuts on macOS; tab cycling uses Control.

## Verify

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
# Requires an interactive Windows desktop:
cargo test --test screenshot -- --ignored --test-threads=1
```

Capture the actual wgpu window with `palette --screenshot output.png`. Debug builds also accept `--screenshot-scene colors` and `--screenshot-size 720x520`. The screenshot suite covers 202 full-size Windows states plus eleven responsive-ribbon states at smaller widths. See [video behavior](docs/UI_BEHAVIOR_REFERENCE.md) and [responsive ribbon validation](docs/RESPONSIVE_RIBBON_VALIDATION.md).

See [architecture](ARCHITECTURE.md), [feature checklist](FEATURES.md), [known limitations](KNOWN_LIMITATIONS.md), [file compatibility](docs/FILE_FORMATS.md), [dependency licenses](docs/DEPENDENCIES.md) and [current milestone validation](docs/PHASE5_VALIDATION.md).

Palette source is MIT licensed. Icons are original geometric Rust drawings; Microsoft and Adobe branding/resources are not bundled.

Artistic catalog completion: Fresco, Neon Glow, Palette Knife, Plastic Wrap, Rough Pastels, Sponge, Underpainting and Watercolor are live, editable and removable. The Artistic branch now has all 15 entries. [Algorithms, controls and validation](docs/ARTISTIC_COMPLETION_VALIDATION.md).

Optical/texture follow-up: Diffuse Glow, Glass, Ocean Ripple, Texturizer, Shape Blur, Smart Blur, Sharpen Edges and Smart Sharpen are live and removable. All current Sharpen and Texture entries are implemented. Glass/Ocean Ripple/Diffuse Glow have mouse-positioned origins; Shape Blur has five generated apertures and Smart Blur has Normal/Edge only/Overlay edge modes. [Algorithms, controls and validation](docs/OPTICAL_FILTERS_VALIDATION.md).

Stylize/render follow-up: Extrude, Oil Paint, Tiles, Trace Contour, Wind, Lens Blur, Lens Flare and Lighting Effects are live and removable. All current Stylize, Blur and Render entries are implemented as independent equivalents. The following batch completes the four editor/import entries. [Controls, algorithms and validation](docs/STYLIZE_RENDER_VALIDATION.md).

Previous 121-entry catalog completion: Color Lookup loads 3D `.cube` LUTs; Displace loads real raster maps with Stretch/Tile fitting; Shear has a draggable five-knot curve; Custom has a 5×5 coefficient grid and presets. Use the contextual toolbar’s … menu for imports/editors. Imported data is embedded as bounded binary `.palette` assets; previews, masks, history and removal remain nondestructive. All 121 entries at that milestone had independent implementations; Gallery, remaining layer styles and processing optimization still need work. [Scope and validation](docs/EDITABLE_FILTERS_VALIDATION.md).

Expanded filter audit: eleven more independent filters bring the catalog to **132**. Blur Gallery adds Field/Iris/Tilt-Shift/Path/Spin Blur; Render adds Flame/Picture Frame/Tree; Video adds De-Interlace/NTSC Colors; Other adds HSB/HSL. All use live removable filters. [Implementation and limits](docs/GALLERY_VIDEO_RENDER_VALIDATION.md).

Substance-based Parametric Filters and full Photoshop quality/control parity remain outside the implemented conventional catalog.

Layer workflows now include ten live effect families, multi-selection with an undoable confirmed merge, an ordered active-layer stack and a separate docked Background swatch. See [layer validation](docs/LAYER_EFFECTS_PANEL_VALIDATION.md). Native saves use version 8 and read versions 1–7.

Extended effect controls, material/curve editors, shared lighting and v8 preservation are described in [layer-style settings validation](docs/LAYER_STYLE_SETTINGS_VALIDATION.md).

Layer movement now uses persistent GPU layer textures and GPU composition, preserving filters, masks, effects, opacity and all 19 blend modes. Warmed eligible drags submit no CPU render jobs or layer-image uploads. CPU preparation runs when appearance inputs change; bounded regional CPU rendering remains the fallback. [GPU architecture, measurements and validation](docs/GPU_COMPOSITOR_VALIDATION.md).
