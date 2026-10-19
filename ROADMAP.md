# Master roadmap review

This review follows the original phases. Functional source code and passing Windows tests count as local implementation; a configured CI job, disabled menu entry or model field alone does not establish a completed workflow. The selection-handle slice is sufficient for now, per the user's direction.

| Phase | Current evidence | Remaining gate / next work |
|---|---|---|
| 0 — inspection/design | Workspace, references/video, UI-independent model and libheif research documented | Supplied specimen inspected, hashed and decoded; broader Paint variants remain unverified |
| 1 — runnable interface | Native Paint-style Windows app, tabs, layers, zoom, sliders, ordinary raster files and reviewed captures | Exact WinUI nuance, accessibility, Windows 10/macOS/Linux execution |
| 2 — core painting | Live strokes, eraser, fill, shapes/polygon/curve, text, clipboard, selections, transforms and history work | Brush variants/tablet/mask painting, text metric parity, property gesture coalescing and persistent vectors remain explicit deferrals |
| 3 — file compatibility | Real still AVIF codec/options; `.palette` v5 streamed shared assets, v1/v2/v3/v4 migration, backing color and opaque metadata; genuine two-layer `.paint` subset import/export; import layer, recents and recovery | User confirms Paint opening/appearance; edit/save-back, broader variants/platforms and metadata/color/conformance remain |
| 4 — advanced layers/selections | 19 blend modes; Wand/freeform/edge-aware brush, live feather previews, canvas toolbars, coverage combination and basic layer-mask properties | Merges, polygon selection, morphology and mask painting/view/apply/link workflows |
| 5 — filters/styles | 142 live filters (135 conventional, seven limited local model-based counterparts), six styles, typed registry/common parameters, on-canvas previews, three-icon layer controls, filter masks, evaluated-layer cache and native preservation | Remaining styles, Filter Gallery, per-stage/tile caching, GPU processing |
| 6 — platform polish | CI configuration and Windows native captures | Actual platform matrix, performance benchmarks on dense 4K/8K, accessibility, packaging and signing |

## Phase 3 acceptance status

Implemented and locally validated: ordinary raster I/O; real AVIF opening/encoding with alpha and quality/speed options; lossless native preservation of all currently represented editable data and original ICC/Exif bytes; explicit v1/v2/v3/v4-to-v5 migration; atomic saves; detected animation/page import notices; image-as-layer import; persistent recent files; periodic recovery with startup restore/discard. See [Phase 3 validation](docs/PHASE3_VALIDATION.md) and [file compatibility](docs/FILE_FORMATS.md).

Phase 3's documented subset is implemented. The user has now confirmed that the generated `.paint` opens and looks correct in Microsoft Paint. That is recorded as manual opening/appearance validation and accepted for progression to Phase 4; it is not an assertion that native layer edits and a Paint-save-back cycle were tested. Those broader checks and the platform matrix remain tracked. The adapter still labels its limited export experimental. No commits, branch changes or pushes were performed.

AVIF Workflow I is tested locally on Windows with actual rav1e encoding and independent rav1d decoding. The four-target CI configuration includes codec tests and NASM setup, but no successful remote platform run is claimed. Workflow H preserves currently implemented model fields; future filters/styles require their own feature work.

## Next work

At the user's request, progress has moved to Phase 5. Its first complete vertical slice is implemented and validated: [live processing validation](docs/PHASE5_VALIDATION.md). This advances the roadmap without declaring all earlier deferrals complete.

1. Continue with remaining artistic/brush-stroke/sketch/texture operations, additional distortions and custom convolution. Despeckle, Dust & Scratches, Reduce Noise, Pinch, Spherize, Ripple and fixed Sharpen/More are implemented. Surface Blur, Spin/Zoom Radial Blur, Clouds/Difference Clouds, Poster Edges, Grain and Fragment are now implemented. Find Edges/Emboss, Solarize, Unsharp Mask, Average, Minimum/Maximum, Offset, Twirl and Wave are now implemented. Levels, five-point Curves, Motion Blur, seeded Add Noise, High Pass and Mosaic are now implemented alongside the earlier catalog.
2. Reduce full-resolution styled-layer costs through alpha-only padded assets and more general per-stage/tile caching before claiming smooth dense 4K/8K editing.
3. Continue styles with Bevel/Emboss, Satin and Gradient/Pattern Overlay; build Filter Gallery around implemented entries. Inner Shadow/Glow and inside/outside/centered Stroke are implemented.
4. Retain the open Phase 4 work: polygon selection, source-preserving mask painting/view/link/apply, selection morphology, selected/visible merges and confirmed flattening with backdrop-dependent blend tests.
5. Broaden `.paint` evidence with native layer editing/save-back and execute the platform codec/build matrix as environments become available.

The deferred Phase 2 polish remains in FEATURES.md. It has not been silently declared complete or used to delay the requested move to file compatibility.

Selection display/navigation follow-up: original and feather-extent ants, clipped inverted perimeters, per-axis canvas rails, and native v4 preservation are implemented. Evidence and remaining limits: [selection/navigation validation](docs/SELECTION_NAVIGATION_VALIDATION.md).


Latest slice: [brush selection validation](docs/BRUSH_SELECTION_VALIDATION.md). Phase 3 local code gaps identified in this review (animated WebP/APNG/TIFF import notices) are resolved and tested. Native Paint layer editing/save-back was attempted again through Computer Use, but the native pipe is unavailable (os error 2). The user-confirmed opening/appearance result remains valid; save-back and unavailable platform execution remain external acceptance gates, not completed checks. Working-profile conversion and broader metadata/codec conformance are tracked future capabilities rather than silently represented as implemented.

Latest UI follow-up: [three layer controls and on-canvas processing](docs/LAYER_PROCESSING_UI_VALIDATION.md). This replaces the floating stack window/modal parameter preview with Paint-height contextual bars; it does not expand the supported filter catalog.

Latest Phase 5 batch: [expanded catalog validation](docs/PROCESSING_CATALOG_VALIDATION.md), including algorithm ranges, native preservation, independent references and measured Box/Median costs. Phase 5 remains in progress.

Latest Phase 5 follow-up: [requested style controls and six filters](docs/STYLE_FILTER_CONTROLS_VALIDATION.md). Glow Radius, shadow Angle/Distance/Spread and Stroke Width/Rounded corners are functional. Native v5 prevents older readers silently losing round joins; v1–v4 remain readable.

Current follow-up: [conventional menu and ten filters](docs/FILTER_MENU_VALIDATION.md). Filter Gallery is disabled and first in the menu. Its future window should compare filters using one shared example image, such as a flower or sailboat; this is an agreed design direction, not built functionality. Correction, Liquify, Vanishing Point and filter-plugin sections are excluded by user instruction. The remaining conventional branches are researched; inactive filters are explicitly disabled. Phase 5 remains in progress.

Latest Phase 5 batch: [creative filters validation](docs/CREATIVE_FILTERS_VALIDATION.md). Render, Artistic and Texture branches now contain working operations. Gallery remains disabled, with the shared example-image design retained for later. No correction/plugin sections were introduced.

Latest Phase 5 batch: [noise cleanup and distortions](docs/RETOUCH_DISTORT_VALIDATION.md). All five conventional Noise entries now have independent implementations; advanced Photoshop controls and JPEG-specific cleanup remain outside their documented scope. Gallery stays disabled, and correction/plugin sections remain excluded. Phase 5 is still in progress.

Latest Phase 5 batch: [movable origins and four filters](docs/ORIGIN_FILTERS_VALIDATION.md). Mouse placement, numeric origin controls and reset accompany Polar Coordinates, ZigZag, Fibers and Glowing Edges. Native v6 preserves origin fields and migrates v1–v5. Phase 5 remains in progress; the next catalog work can fill artistic, brush-stroke, sketch and texture branches, followed by custom convolution and remaining live styles.

Latest Phase 5 batch: [eight artistic/print/texture filters](docs/ARTISTIC_FILTERS_VALIDATION.md). Artistic and Texture expand; Brush Strokes and Sketch now have working entries. Gallery remains disabled; all new operations use removable stacks and on-canvas previews. Phase 5 remains in progress. Next useful groups: remaining paint/sketch/texture filters, additional pixelate methods and custom convolution, followed by unfinished live styles.

Latest Phase 5 batch: [Pixelate completion, cells and diffusion](docs/PIXELATE_FILTERS_VALIDATION.md). All seven named Pixelate filters have working independent implementations; Texture adds Stained Glass/Mosaic Tiles and Stylize adds Diffuse. At that milestone the inventory had 63 implemented and 58 pending filters out of 121, excluding Gallery/styles. Phase 5 remains in progress. Next useful work: adjustments and remaining paint/sketch filters, Custom convolution and unfinished styles.

Latest Phase 5 batch: [six color adjustments and two fixed blurs](docs/ADJUSTMENT_FILTERS_VALIDATION.md). At this milestone: **71 implemented / 121 catalog filters, 50 remaining**, plus six layer styles. Adjustments has only Color Lookup pending; Blur still lacks Lens Blur, Shape Blur and Smart Blur. All new filters retain editable settings, live previews, masks and undo. Gallery stays disabled. Next useful group: remaining Artistic/Brush Strokes/Sketch filters or Custom convolution, then unfinished styles and processing optimization. Phase 5 remains in progress.

Latest Phase 5 batch: [eight traditional-media painting filters](docs/PAINTING_FILTERS_VALIDATION.md). Artistic now has seven working entries and Brush Strokes has six; their remaining entries stay disabled. At this milestone: **79 implemented / 121 catalog filters; 42 remain**, plus six styles. All new operations retain source pixels, live controls, native settings, masks and undo. Gallery remains disabled. Next useful group: Sketch filters, remaining Artistic/Brush Strokes entries or Custom convolution; remaining styles and processing optimization still need work. Phase 5 remains in progress.

Latest Phase 5 batch: [directional spray and six Sketch filters](docs/DRAWING_FILTERS_VALIDATION.md). All eight current Brush Strokes entries now have independent implementations. Sketch has eight working entries and six still disabled. At this milestone: **87 implemented / 121 catalog filters; 34 remain**, plus six styles. Live previews, source pixels, masks, saved colors/axes and undo remain intact. Gallery stays disabled. Next useful work: finish Sketch, additional Artistic/Distort entries or Custom convolution; remaining styles and processing optimization are still pending. Phase 5 remains in progress.

Previous Phase 5 batch: [Sketch catalog completion](docs/SKETCH_COMPLETION_VALIDATION.md). All 14 current Sketch entries now have independent implementations. Inventory at that milestone: **93 implemented / 121 catalog filters; 28 remain**, plus six styles. Conté Crayon adds generated materials; Chrome stays grayscale and Water Paper retains colors. All settings remain nondestructive, saved and undoable. Gallery remains disabled. Next useful work: remaining Artistic or Distort entries, Custom convolution and Texturizer; unfinished styles and processing optimization remain pending. Phase 5 is still in progress.

Previous Phase 5 batch: [Artistic catalog completion](docs/ARTISTIC_COMPLETION_VALIDATION.md). Eight new filters complete all 15 Artistic entries. Inventory at that milestone: **101 implemented / 121 catalog filters; 20 remain**, plus six styles. All settings remain nondestructive, saved and undoable. Gallery stays disabled. Next useful batch: the five remaining Distort entries, Texturizer and two Sharpen entries; Custom convolution, remaining styles and processing optimization also remain. Phase 5 is still in progress.

Previous Phase 5 batch: [Optical/texture filters](docs/OPTICAL_FILTERS_VALIDATION.md). Eight new filters complete the current Sharpen and Texture branches. Inventory at that milestone: **109 implemented / 121 catalog filters; 12 remain**, plus six styles. Three more origins support mouse placement; all settings remain editable, saved and undoable. Next useful work: Displace with a real map importer and Shear with a curve editor, Color Lookup with LUT import, remaining Stylize/Render/Lens Blur and Custom convolution. Filter Gallery, remaining styles and processing optimization remain pending. Phase 5 is still in progress.

Previous Phase 5 batch: [Stylize/render completion](docs/STYLIZE_RENDER_VALIDATION.md). Eight new independent filters complete the current Stylize, Blur and Render branches. Inventory at that milestone: **117 implemented / 121 catalog filters; 4 remain**, plus six styles. Remaining entries need complete editors/importers: Color Lookup (LUT), Displace (map), Shear (curve) and Custom (kernel). Gallery, remaining layer styles, color management and processing optimization are still pending. Phase 5 remains in progress.

Previous Phase 5 batch: [Editable catalog completion](docs/EDITABLE_FILTERS_VALIDATION.md). Color Lookup, Displace, Shear and Custom now have actual LUT/map/curve/kernel workflows. Current inventory: **121 implemented / 121 catalog filters; zero remain in this catalog**, plus six styles. This does not complete Phase 5 or assert Photoshop parity. Next useful milestone: remaining live layer styles (Bevel/Emboss, Satin, Gradient/Pattern Overlay), followed by Filter Gallery when requested, processing optimization and broader platform validation. The filter registry remains extensible.

Latest Phase 5 follow-up: [Expanded Photoshop catalog](docs/GALLERY_VIDEO_RENDER_VALIDATION.md) adds eleven filters omitted from the earlier inventory: five Blur Gallery filters, Flame/Picture Frame/Tree, De-Interlace/NTSC Colors and HSB/HSL. **132 / 132 conventional catalog entries** plus six styles. The [coverage audit](docs/PHOTOSHOP_FILTER_AUDIT.md) explicitly tracks unimplemented Neural/AI filters and prior exclusions. Phase 5 remains in progress; next useful work is remaining layer styles and processing optimization, with remaining model-based filters requiring validated local models and pixel/geometry adapters.

The external audit also records the unimplemented Substance-based Parametric Filters family separately from Neural/AI and the 133 conventional entries; see [coverage audit](docs/PHOTOSHOP_FILTER_AUDIT.md).

Local-only neural milestone: [first-six status](docs/NEURAL_FILTERS_FIRST_SIX.md). Mosaic Style Transfer now runs inside Palette with bundled weights and Rust CPU inference, editable parameters, live preview, masks and native preservation. User images never leave the process. JPEG Artifacts Removal now uses full-resolution DnCNN-3 tiles, a bounded restoration cache and cancellation of obsolete main renders. Colorize now infers local chrominance while retaining original source luminance/alpha. Face-guided Skin Smoothing now uses learned geometry with conventional smoothing. Cropped Super Zoom now runs a real 3x network. Smart Portrait and enlarged editable Super Zoom documents still require validated models/pipelines or output-geometry work; whole-image PNG enlargement is available from the Super Zoom options; Depth Blur now has a local learned-depth/Gaussian-defocus implementation, while the other five later-group families and Filter Gallery remain deferred. No conventional substitute is counted as a neural implementation.

Current Phase 5 follow-up: conventional [Color Transfer](docs/COLOR_TRANSFER_VALIDATION.md) adds portable reference-image Lab statistics, live controls and source-preserving native persistence. The catalog is now 139 filters (133 conventional and six limited model-based counterparts). Learned Color Transfer and the other audited parity gates remain incomplete.

Latest Phase 5 follow-up: [Makeup Transfer](docs/MAKEUP_TRANSFER_VALIDATION.md)
adds a reference-portrait lip/upper-lid workflow using existing local face models
and conventional Lab matching. The catalog is 140 filters: 133 conventional and
seven limited model-based counterparts. Four audited families still lack a
counterpart; partial capabilities and quality gates prevent Photoshop parity.

Latest Phase 5 follow-up: [Photo Restoration v2](docs/PHOTO_RESTORATION_CRACKS_VALIDATION.md)
extends thin-scratch/dust repair to wider curved cracks, 200% Strength, larger
radii, polarity and optional local facial-detail protection. It follows spatial Makeup
Transfer v2 and reference-layer Harmonization. Current catalog: 142 filters
(135 conventional and seven limited model-based counterparts), plus six styles.
Smart Portrait and Landscape Mixer still lack counterparts; learned photo/face
reconstruction and other partial-capability/quality gates remain. Phase 5 is not complete.
