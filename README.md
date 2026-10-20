# Palette

A native Rust raster editor with a Paint-style ribbon and a UI-independent, layered editing engine. Core Paint editing, Phase 3 file workflows, the first Phase 4 layer/selection slice and a Phase 5 live-processing slice are implemented. The user confirmed the generated `.paint` opens and looks correct in Microsoft Paint; broader interoperability and cross-platform validation remain tracked in the [roadmap](ROADMAP.md).

## Run

Install stable Rust **1.98 or newer**, then:

```sh
cargo run --locked
# Open a file from the command line:
cargo run --locked -- path/to/image.png
```

Windows needs the MSVC C++ build tools, Windows SDK and NASM on PATH. macOS needs Xcode Command Line Tools; Intel builds also need NASM. On Linux, install a C/C++ toolchain, NASM, pkg-config, X11/Wayland development packages, a working Vulkan driver, an XDG desktop portal backend and Zenity. See [build and packaging notes](docs/BUILD.md).

## Working now

New documents use a **1280 x 720** white canvas at **100% zoom**. Image Properties > Default uses the same size. Use Fit or pan when the canvas is wider than the workspace.

- Paint-style dark ribbon, split buttons, anchored flyouts, hover/press animation, compact document tabs, animated layers tray, three contextual brush controls and status bar.
- Centered Edit Colors and Image Properties dialogs with OK/Cancel, custom color slots, exact RGB/hex input and reversible canvas bounds changes. View toggles for gridlines, status bar, ribbon and layers.
- New/open documents; switch, drag to reorder and close tabs using an integrated hover-only button; independent zoom, layers and history; modified-document close prompts matching the supplied Paint dialog, including keyboard focus and Save/Don’t save/Cancel.
- Pencil, continuous round brush, soft eraser, color picker and magnifier. All 23 reference shape-gallery tools are functional, including free Polygon and a two-bend cubic Curve. Shapes have independent foreground outline/background fill, opacity, width, drag previews and Shift constraints. Finished shapes are selected automatically; right mouse swaps drawing colors.
- Size, opacity and hardness controls, including exact numerical input. Erasing reduces alpha.
- Brush, pencil and eraser strokes appear while dragging, with partial canvas texture updates and one undo command per stroke.
- Text: type directly on the canvas with a floating formatting strip, move the box by its top edge, or resize using eight handles. Ubuntu Light/Hack, size, wrapping, alignment, line spacing, colors and opacity work. Done, Ctrl/Command+Enter or clicking outside commits a separate raster layer; Cancel/Escape leaves pixels unchanged.
- Add, duplicate, rename, show/hide, lock, reorder and delete layers; properties directly beneath the selected layer; opacity; all 18 requested blend modes plus Divide, with correct alpha compositing and native project preservation.
- Move whole layers through Selection > Move active layer or the layer context menu: drag with live rendering, Shift to constrain an axis, arrows to nudge (Shift for ten pixels), Escape to cancel. One drag is one undo command; pixels, masks and filters remain intact.
- Horizontal/vertical canvas scrollbars appear only on overflowing axes, in workspace gutters clear of the layers panel. Drag their thumbs, click the rails, or use arrows/Page Up/Page Down/Home/End while focused. Fit removes the bars.
- Pan with middle mouse or Space+drag; scroll to pan; Ctrl/Command+wheel to zoom around the pointer; 100% default zoom with slider snapping at actual size, fit and canvas corner resize.
- Native open/save dialogs, file drop, atomic `.palette` v6 saving with shared binary assets and migration of existing v1/v2/v3/v4/v5 projects. PNG, JPEG, BMP, GIF, TIFF, WebP and still AVIF have real codec tests. AVIF export has quality/speed options and alpha.
- File > Import to canvas adds a raster as a separate layer with one undo command. Recent files persist across runs. Recovery copies of committed edits are made in the background approximately every 30 seconds; the next launch offers Restore, Discard or Later without overwriting original files.
- Three original icon buttons beneath layer Opacity open Filters, Effects and Blending mode. Filters/effects use a compact toolbar at the top of the canvas, matching the Text/Selection bar height. Choose a stack instance, Add/Edit, toggle On, or use the … menu to reorder, duplicate, remove or manage a filter mask. The blend icon opens the existing mode catalog.
- Filter > Blur > Gaussian Blur opens the same canvas toolbar with precise radius input. Parameter edits preview directly on the image; choose other settings from the parameter selector. Apply commits editable settings, Cancel/Escape restores the committed view, and Preview switches comparison on/off. Invert and Brightness share the nondestructive filter stack. [Layer control validation](docs/LAYER_PROCESSING_UI_VALIDATION.md).
- Live adjustments: Brightness/Contrast, Exposure, Hue/Saturation, Vibrance, Black & White, Threshold and Posterize, alongside Invert and legacy Brightness. Levels and five-point Curves are editable too. Blur offers Gaussian, Box and Motion Blur; Noise offers Median and seeded Add Noise (uniform/Gaussian, monochrome/color). Pixelate offers Mosaic; Other offers High Pass. Conventional Filter branches show unfinished entries disabled; active entries and toolbar Add menus resolve the implemented registry.
- More conventional filters: Average, Find Edges, Emboss, Solarize, Unsharp Mask, Minimum, Maximum, Offset, Twirl and Wave. Filter Gallery is first in the menu and disabled for now; lens correction, Liquify, Vanishing Point and plugin sections are omitted. [Menu and filter validation](docs/FILTER_MENU_VALIDATION.md).
- Surface Blur, Spin/Zoom Radial Blur, editable seeded Clouds/Difference Clouds, Poster Edges, five Grain patterns and Fragment. Clouds captures foreground/background colors at creation; all remain removable live filters. [Creative filter validation](docs/CREATIVE_FILTERS_VALIDATION.md).
- Noise cleanup: Despeckle, Dust & Scratches and Reduce Noise (strength, detail protection, color cleanup and sharpening). Distort adds Pinch, Spherize and Ripple; Sharpen and Sharpen More are also functional. [Validation and limits](docs/RETOUCH_DISTORT_VALIDATION.md).
- Polar Coordinates (both directions), ZigZag (three styles), seeded two-color Fibers and Glowing Edges. Twirl, Pinch, Spherize, Ripple, Wave, Radial Blur and both new distortions have movable origins: use the four-arrow placement button, then click/drag on the layer. Origin X/Y provide numeric percentages; … > Reset origin restores the default. Origins preview live and remain editable in saved projects. [Validation and exact scope](docs/ORIGIN_FILTERS_VALIDATION.md).
- Artistic: Cutout and Film Grain; Brush Strokes: Accented Edges and Ink Outlines; Sketch: Photocopy and Dot/Line/Circle Halftone Pattern; Texture: Patchwork and seeded Craquelure. All are editable live filters. Sketch ink/paper colors capture drawing colors when added and remain saved parameters. [Algorithms and validation](docs/ARTISTIC_FILTERS_VALIDATION.md).
- Pixelate now has working Color Halftone, Crystallize, Facet, Mezzotint and Pointillize alongside Mosaic/Fragment. Texture adds Stained Glass and Mosaic Tiles; Stylize adds Normal/Darken/Lighten/Anisotropic Diffuse. All remain editable live filters. [Validation](docs/PIXELATE_FILTERS_VALIDATION.md). The current catalog has **144 implemented filters (136 conventional plus eight limited local model-based counterparts)**; [generated inventory](docs/FILTER_INVENTORY.md).
- Adjustments now also includes Color Balance, Photo Filter, Channel Mixer, Gradient Map, Selective Color and Shadows/Highlights. Gradient Map captures drawing colors for new instances; all nine Selective Color ranges remain editable together. Blur and Blur More are functional fixed-kernel commands. [Algorithms, controls and validation](docs/ADJUSTMENT_FILTERS_VALIDATION.md).
- Artistic adds Colored Pencil, Dry Brush, Paint Daubs and Smudge Stick; Brush Strokes adds Angled Strokes, Crosshatch, Dark Strokes and seeded Spatter. Editable paper color/brightness, pressure, texture, brush geometry and tonal stroke controls use the existing live toolbar. [Algorithms and validation](docs/PAINTING_FILTERS_VALIDATION.md).
- Brush Strokes is complete for the current menu: Sprayed Strokes and Sumi-e join the six earlier entries. Sketch adds Bas Relief, Chalk & Charcoal, Charcoal, Graphic Pen, Note Paper and Stamp, with saved drawing colors, stroke directions and editable relief lighting. [Algorithms and validation](docs/DRAWING_FILTERS_VALIDATION.md).
- Sketch now includes all 14 current entries: Chrome, Conté Crayon, Plaster, Reticulation, Torn Edges and Water Paper complete the branch. Conté Crayon has generated Canvas/Brick/Burlap/Sandstone materials, size, lighting, relief and inversion; Chrome stays grayscale and Water Paper retains source colors. [Algorithms and validation](docs/SKETCH_COMPLETION_VALIDATION.md).
- Live Drop Shadow, Outer Glow, Inner Shadow, Inner Glow, Stroke and Color Overlay with editable parameters. Stroke supports outside, inside and centered placement, with subpixel width and a Rounded corners checkbox. Glow opens on Radius; shadows open on Angle with Distance/Spread fields alongside in wider windows. Source edits automatically update their appearance; source pixels stay intact. Full editable settings survive `.palette` saving and undo. [Current catalog, style controls and performance limits](docs/STYLE_FILTER_CONTROLS_VALIDATION.md).
- Polygon: click vertices with a live next-segment line; Enter, double-click or click the first vertex closes it. Backspace removes a vertex; Escape cancels. Curve: drag a baseline, then drag two bends; Enter can finish early.
- Eight selection resize handles and a rotation handle, with live processed-pixel previews for move/resize/rotation. Shift locks resize proportions, constrains movement or snaps rotation to 15 degrees; Escape cancels. A completed gesture is one undo command.
- Selection brush in the Selection menu: edge-aware color matching or plain coverage painting, tolerance/feather controls, left-side size/strength/hardness, all four combine modes, Alt subtraction, a cyan coverage overlay and one undo step per stroke. Preview updates while held; Escape cancels. [Behavior and validation](docs/BRUSH_SELECTION_VALIDATION.md).
- Animated marching-ants outlines around selections, including holes, moving selections and creation previews. Inverted selections retain a visible outer boundary even when zoom clips the actual canvas edges. Feathered selections keep opaque ants on the original contour and a separate 40%-opacity outline at the feather extent.
- Rectangular, freeform/lasso and Magic Wand selections, select all/deselect/invert, selection-constrained painting/erasing/shapes, clear and selection-size status. Selection > Combine selection offers Replace/Add/Subtract/Intersect; Shift adds, Alt subtracts and Shift+Alt intersects. In Replace mode, drag selected pixels or handles to transform; Shift constrains a handle gesture. Arrow keys nudge, Shift+arrows move ten pixels. Wand options provide perceptual tolerance, contiguous/all-matching colors, active/all-visible sampling and antialiasing.
- Selection tools show a compact toolbar above the canvas with Replace/Add/Subtract/Intersect icons, inversion, and a feather slider plus numeric field (0–128 image pixels, displayed to one decimal below 5 px). Wand adds tolerance (0–100) and sampling options. Both sliders use a mild shifted-log curve (midpoint about 23–27 units) for finer low-value adjustment; exact fields remain available. The toolbar uses the text strip’s height and fits its contents. Feather previews an existing selection live while dragging and commits one undo step on release; Escape cancels. It also sets the radius applied to new regions before combining. Freeform selection closes on release, has a live ants preview and supports Escape cancellation and undo.
- Selection > Feather selection previews grayscale coverage relative to the retained original; Apply commits and Cancel discards the draft. Radius zero restores the original. Original contours survive undo, transforms and native saving. Higher Wand tolerance includes more colors; active-layer versus all-visible sampling remains selectable. Layer properties can add a reveal-all mask or create one from selection, then enable/disable, invert, feather or delete it. Source pixels remain unchanged; mask operations support undo and native saving. See [Phase 4 validation](docs/PHASE4_VALIDATION.md).
- Contiguous fill with adjustable tolerance (0–255) and opacity. Right mouse uses the background color.
- System image clipboard copy/cut/paste; paste creates a new layer and expands the canvas if needed. Copy samples the active layer's evaluated appearance.
- Crop canvas to selection, horizontal/vertical flip, exact quarter-turns and custom-angle rotation. A selection limits flips/rotations to active-layer selected source pixels; without a selection they transform the whole document. Custom rotation uses alpha-safe bilinear sampling and an OK/Cancel dialog. Resize offers nearest or bilinear sampling with optional aspect lock; layers and live processing stacks are retained.
- Consistent stroke opacity across pointer events/retracing, tile-sharing-aware undo accounting, and background fill/geometry jobs with stale-result protection.

Unavailable ribbon tools and filter categories are visibly disabled. **Persistent vector shape objects, brush variants, polygon selection, mask painting and remaining layer effects are not available yet.** Text supports basic left-to-right layout with two bundled fonts and becomes raster content on Apply; system font browsing and complex-script shaping remain pending. The supplied `.paint` specimen opens as two editable layers. File > Export offers an experimental genuine HEIF `.paint` copy; masks/blends/live processing require a complete `.palette` save before compatibility conversion. The user confirmed the generated copy opens and looks correct in Microsoft Paint. Editing there and saving back remain separate unverified checks; see [specimen evidence and limits](docs/PAINT_RESEARCH.md). AVIF animation, HEIF rotation/crop properties and complete profile-aware processing are pending. Native projects retain original ICC/Exif bytes; flattened exports do not preserve metadata.

Save/Save As preserve editable state in `.palette`. Export writes a composite image and does not mark the editable project saved. GIF imports its first frame; export does not create an animation. Animated WebP/APNG and TIFF imports display first-frame/default-image/first-page limitations, including when imported as a layer. JPEG composites transparency over white.

## Shortcuts

| Action | Shortcut |
|---|---|
| New / Open / Save | Ctrl+N / Ctrl+O / Ctrl+S |
| Save As / Close tab | Ctrl+Shift+S / Ctrl+W |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y |
| Next / Previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Copy / Cut / Paste image | Ctrl+C / Ctrl+X / Ctrl+V |
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

Capture the actual wgpu window with `palette --screenshot output.png`. Debug builds also accept `--screenshot-scene colors` and `--screenshot-size 780x520`. 204 reviewed Windows states at 1386 x 963 guard the window, menus, dialogs, text and layers. See [video behavior and validation](docs/UI_BEHAVIOR_REFERENCE.md).

See [architecture](ARCHITECTURE.md), [feature checklist](FEATURES.md), [known limitations](KNOWN_LIMITATIONS.md), [file compatibility](docs/FILE_FORMATS.md), [dependency licenses](docs/DEPENDENCIES.md) and [current milestone validation](docs/PHASE5_VALIDATION.md).

Palette source is MIT licensed. Icons are original geometric Rust drawings; Microsoft and Adobe branding/resources are not bundled.

Artistic catalog completion: Fresco, Neon Glow, Palette Knife, Plastic Wrap, Rough Pastels, Sponge, Underpainting and Watercolor are live, editable and removable. The Artistic branch now has all 15 entries. [Algorithms, controls and validation](docs/ARTISTIC_COMPLETION_VALIDATION.md).

Optical/texture follow-up: Diffuse Glow, Glass, Ocean Ripple, Texturizer, Shape Blur, Smart Blur, Sharpen Edges and Smart Sharpen are live and removable. All current Sharpen and Texture entries are implemented. Glass/Ocean Ripple/Diffuse Glow have mouse-positioned origins; Shape Blur has five generated apertures and Smart Blur has Normal/Edge only/Overlay edge modes. [Algorithms, controls and validation](docs/OPTICAL_FILTERS_VALIDATION.md).

Stylize/render follow-up: Extrude, Oil Paint, Tiles, Trace Contour, Wind, Lens Blur, Lens Flare and Lighting Effects are live and removable. All current Stylize, Blur and Render entries are implemented as independent equivalents. The following batch completes the four editor/import entries. [Controls, algorithms and validation](docs/STYLIZE_RENDER_VALIDATION.md).

Previous 121-entry catalog completion: Color Lookup loads 3D `.cube` LUTs; Displace loads real raster maps with Stretch/Tile fitting; Shear has a draggable five-knot curve; Custom has a 5×5 coefficient grid and presets. Use the contextual toolbar’s … menu for imports/editors. Imported data is embedded as bounded binary `.palette` assets; previews, masks, history and removal remain nondestructive. All 121 entries at that milestone had independent implementations; Gallery, remaining layer styles and processing optimization still need work. [Scope and validation](docs/EDITABLE_FILTERS_VALIDATION.md).

Expanded filter audit: eleven more independent filters bring the catalog to **132**. Blur Gallery adds Field/Iris/Tilt-Shift/Path/Spin Blur; Render adds Flame/Picture Frame/Tree; Video adds De-Interlace/NTSC Colors; Other adds HSB/HSL. All use live removable filters. [Implementation and limits](docs/GALLERY_VIDEO_RENDER_VALIDATION.md). Local Mosaic Style Transfer, JPEG Artifacts Removal, Colorize, face-guided Skin Smoothing, cropped Super Zoom and Depth Blur have local implementations; three audited Neural Filter families have no counterpart, Color Transfer has a partial conventional implementation, and partial functionality still prevents full parity; [coverage audit](docs/PHOTOSHOP_FILTER_AUDIT.md) distinguishes them from the conventional catalog.

The external audit also records the unimplemented Substance-based Parametric Filters family separately from Neural/AI and the 133 conventional entries; see [coverage audit](docs/PHOTOSHOP_FILTER_AUDIT.md).

Local neural processing: **Filter → Neural Filters → Style Transfer** runs the bundled Mosaic network in Rust inside Palette, with editable strength, live preview, masks, undo and native project preservation. No cloud compute, account, Python runtime, model download or separate service is required. The fixed 224 px working image limits fine detail; original size and alpha remain intact. Smart Portrait now has local expression warps and face shading; generative age/hair/gaze/pose and enlarged editable Super Zoom documents remain pending. [Exact status and validation](docs/NEURAL_FILTERS_FIRST_SIX.md).

Local **JPEG Artifacts Removal** uses the bundled JPEG-trained DnCNN-3 model at source resolution, with overlapping tiles, editable strength and alpha preservation. Strength edits reuse a bounded restoration cache; obsolete renders cancel on edits/tab changes. [Model, validation and limits](docs/JPEG_RESTORATION_VALIDATION.md).

Local **Colorize** uses the bundled ECCV16 model with editable Strength and Color Saturation, retaining full-resolution source luminance/alpha, masks, undo and native preservation. Color inference runs on a 256² working image; predicted colors may differ from the original scene. [Model, reference tests and limitations](docs/COLORIZE_VALIDATION.md).

Local **Skin Smoothing** uses bundled face detection and 468-point landmarks
to guide conventional edge-preserving smoothing. Strength, Blur and Smoothness
stay editable; eyes/lips/brows are protected, original pixels/alpha remain
intact, and masks, undo and native saving work normally. No detected face leaves
the image unchanged. [Implementation and limits](docs/SKIN_SMOOTHING_VALIDATION.md).

Local **Super Zoom** uses a bundled 3× learned luminance network for editable
1–3× cropped magnification, Enhancement and a draggable source center. It keeps
canvas size and original source pixels, supports masks/undo/native saving and
runs offline. Its **… → Export enlarged whole image as PNG…** command outputs
the complete visible image at 1–3×, without changing the editable project.
Enlarged editable native documents remain unavailable.
[Exact scope and validation](docs/SUPER_ZOOM_VALIDATION.md).

**Full Photoshop filter parity has not been reached.** The catalog count only
measures implemented Palette entries. Missing and partial capabilities and the
requirements for reporting parity are tracked in the [coverage audit](docs/PHOTOSHOP_FILTER_AUDIT.md).

Local **Depth Blur** estimates relative depth with bundled MiDaS weights and
offers click-to-focus, manual focal depth, focal range, blur radius and a depth-map
view. It preserves original alpha and source pixels, with live editable previews,
masks, native saving and undo. The defocus renderer is conventional Gaussian blur;
it does not simulate occlusion-aware optics. [Scope and validation](docs/DEPTH_BLUR_VALIDATION.md).

**Filter → Neural Filters → Color Transfer** matches global Lab colour statistics
to an original warm palette or a reference image loaded with **Reference photo…**
on its contextual toolbar (or its **…** menu).
Strength, Luminance, Contrast and Saturation remain editable. Source pixels and
alpha stay intact; projects retain the complete statistical profile and reopen
without the reference file. This is conventional local processing, not Adobe's
learned Color Transfer. [Scope and validation](docs/COLOR_TRANSFER_VALIDATION.md).

**Filter → Neural Filters → Makeup Transfer** aligns a bounded reference-face
atlas to local facial landmarks, transferring spatial colours, liner and highlights.
Load a portrait with **Reference photo…**. Strength, Lips, Eyes, Cheeks, Edge softness
and Luminance remain editable, with optional occlusion protection. Layer source
pixels remain unchanged and output alpha is preserved. This is geometric transfer,
not learned cosmetic isolation. New filters start with full Strength/Luminance;
old v1 saves retain their original rendering.
[Example validation and limits](docs/MAKEUP_SPATIAL_VALIDATION.md).

**Filter → Neural Filters → Harmonization** matches colour and tone to a selected
reference layer and follows its edits, masks and effects live. The compact
parameter bar includes the reference picker, Strength, Luminance, Contrast and
Saturation. This is conventional local Lab matching, not learned relighting.
[Validation](docs/HARMONIZATION_VALIDATION.md).

**Filter → Neural Filters → Smart Portrait** adds local smile/frown, surprise,
anger and directional face shading, with all/largest face targeting and live
nondestructive controls. Expression endpoints are stronger; high Happiness opens
the lips and reuses visible teeth or renders a shaded procedural dental arch.
Negative Happiness narrows an existing smile. It reuses existing face landmarks;
age/hair/gaze/head-pose synthesis remains unavailable.
[Scope and validation](docs/SMART_PORTRAIT_VALIDATION.md).

**Filter → Neural Filters → Landscape Mixer** now uses edge-aware sky replacement
around ridges/vegetation and transfers reference terrain colours while preserving
source geometry. Sky placement keeps cloud proportions with one horizon-aligned
cover crop. A compact fixed-size reference view supports horizon dragging;
Spring/Summer/Autumn/Winter/Sunset grade both inputs. New references retain up to
2048 px. **… → Refine mix…** offers a live Selection Brush to protect source areas
and restore the automatic mix; corrections save as an editable filter mask.
Method is hidden; existing Horizontal filters keep their legacy rendering.
Loading a new reference or changing Sky detection opts into edge-aware mode. This conventional counterpart
does not synthesize terrain or provide semantic object segmentation.
[Scope and validation](docs/LANDSCAPE_EDGES_VALIDATION.md).
[Protection brush](docs/LANDSCAPE_PROTECTION_VALIDATION.md).

**Filter → Neural Filters → Photo Restoration** repairs thin scratches and wider,
curved cracks, expands faded tones and reduces luminance/colour noise. Strength
extends to 200%, with a 1–64 px repair radius, damage polarity, a detection overlay
and optional local eye/brow/lip protection. Existing v1 saves retain their rendering.
This conventional restoration preserves source pixels and does not reconstruct
missing faces. [Current scope and verification](docs/PHOTO_RESTORATION_CRACKS_VALIDATION.md).
