# Architecture

## Scope and decisions

Phase 0 found an empty Git repository, two screenshot attachments and no `Untitled.paint`. The implementation uses a two-package Cargo workspace: `palette-core` owns editing mathematics and serialized documents; the root `palette` binary owns eframe/egui interaction and wgpu presentation. Core never depends on egui, winit or the operating system.

Current stable releases were checked with Cargo and primary documentation: [eframe 0.37](https://docs.rs/eframe/0.37.0/eframe/), [image 0.25.10](https://docs.rs/image/0.25.10/image/) and [rfd 0.17.2](https://docs.rs/rfd/0.17.2/rfd/). Cargo.lock pins the complete resolution. The toolchain here is Rust 1.99.1; the declared minimum is now 1.98, and has not been independently tested.

egui permits one shared native interface across the three target operating systems. Original vector geometry keeps icon shapes consistent at different DPI settings. Native title bars retain OS window management; title-bar appearance is intentionally OS-owned. wgpu renders the UI and now composites cached layer appearances for document presentation. The independent CPU compositor remains the correctness oracle, export path and fallback; filter/effect preparation still runs on the CPU.

## Model and ownership

`Document` contains canvas bounds, bottom-to-top raster layers, an active stable ID and an optional grayscale selection. Each layer independently owns source RGBA, visibility, lock, opacity, blend mode, signed translation, optional coverage mask, ordered filter instances and ordered style instances. IDs also identify filters and effects. Duplicating a layer allocates new nested IDs.

Raster uses 128 × 128 RGBA8 tiles behind `Arc<Vec<u8>>`. Cloning a document shares pixels. The first modification to a shared tile copies just that tile. Partial boundary tiles have defined padding; ordinary export emits only canvas pixels. Coverage masks are grayscale byte planes behind Arc, ready for selection/mask editing in Phase 4. A mask does not alter source pixels.

The per-layer transform field currently stores translation only. Whole-image rotate/flip/resize operations transform source rasters and related coverage planes. Per-layer affine transforms, color-managed ICC processing, higher working bit depths remain future model work. Phase 5 adds a real neighborhood evaluator for Gaussian Blur and padded shadow/glow styles alongside the existing point evaluator. Color Overlay now has a parameter UI.

## Alpha and pipeline

Stored RGB is straight (unassociated) sRGB; alpha is linear coverage. Per-layer order is source → enabled filters in order (individual coverage masks interpolate premultiplied original/filtered RGBA) → enabled color-overlay styles → layer mask → shadow/glow underlays from the masked alpha → translation → layer opacity and blend mode → source-over document composite. Both transparent and partly transparent input pixels use W3C-style premultiplied arithmetic internally, followed by safe unpremultiplication. Transparent RGB cannot contaminate the output. Current blend modes operate in sRGB, not linear-light RGB. Profile conversion is not implemented.

The GUI converts the straight-alpha reference composite through `ColorImage::from_rgba_unmultiplied`, so egui receives the representation it expects. Checkerboard is presentation-only and never enters saved pixels. JPEG export explicitly composites on white.

## Commands and document sessions

`History::begin` records a named pre-command COW snapshot and assigns a unique revision. A new branch clears redo. Undo/redo swap complete model states, preserving layers, filters, masks, properties and source pixels together. A stroke creates one command, rather than one per sample. `finish` trims history after a gesture/command. The bound is 256 commands and 128 MiB of uniquely retained tile/coverage buffers beyond the current live document; repeated references and buffers still reachable from the live document count once or not at all, respectively. Opaque ICC/Exif buffers also participate in retained-allocation accounting; structural metadata, rendering and live buffers are separate from this budget. Huge destructive operations can still exceed the budget and lose undo; small edits on an 8K canvas retain their undo entry.

Each UI `Session` owns a document, independent history, saved revision, native project path, texture, zoom and pan. Dirty state compares saved and current revisions, so undoing to a saved revision becomes clean. Raster imports have no native project destination; Save prompts for `.palette`. Export never marks the project clean. A save worker captures the original tab ID and revision; a later result cannot accidentally mark new edits or another tab saved. A pending close proceeds only after that tab's successful save and clean-revision check.

## Rendering and jobs

The active document is submitted to a single CPU rendering worker through a capacity-one queue. The job shares COW source data. Results carry session ID, generation and render epoch. Every general edit starts a new epoch; obsolete or closed-tab results are ignored. Within one paint-only epoch, progressively newer snapshots may be displayed even while the pointer keeps changing pixels. An older snapshot cannot replace a newer displayed generation. GPU-ready image conversion runs on the worker. Images exceeding the reported GPU texture dimension receive a reduced display texture while retaining full editing/composite data. Small layer thumbnails are cached per session/layer/generation. File operations run off the UI thread, with at most four outstanding operations, and return through a separate result channel.

Brush/pencil/eraser samples report actual changed-pixel bounds. Once a compatible full-resolution display snapshot exists, the main thread reevaluates that region through the same reference compositor, patches the retained composite and uploads only that GPU subimage before presentation. Pending dirty bounds accumulate until a baseline arrives, so no intermediate samples are lost. General edits still require full background renders. Reduced display textures use progressive background snapshots rather than partial patches. Documents containing enabled spatial filters/styles use progressive background full renders, avoiding incorrect local patches or synchronous UI-thread filtering. A bounded evaluated-layer cache and worker-built spatial thumbnails reuse processing. Per-stage/tile caches, cancellation of main render jobs and GPU compositing remain future work. No GPU-accelerated compositing claim is made.

## Interaction coordinates

Canvas origin is the viewport center minus half the scaled canvas plus pan. Image coordinates are `(pointer - origin) / zoom`. egui supplies logical display points after OS DPI conversion; applying DPI again would be wrong. The display origin is aligned to physical pixels. Integer zoom uses nearest texture sampling. Fractional zoom mapping is tested; filtered texture sampling and advanced trackpad behavior need later polish.

Brush segments rasterize the union of a circular footprint along a line, avoiding gaps during fast motion. Hardness defines the core and linear edge falloff. `PaintStroke` retains a COW pre-stroke raster and sparse float coverage tiles; each pixel is evaluated from its original value at maximum stroke coverage, so event density and retracing within one stroke do not accumulate opacity. Outlined shapes share this accumulator. A new gesture can build opacity. Painting and erasing multiply by canvas-space selection coverage after accounting for layer offsets. `brush::Stroke` wraps that accumulator for the nine material families. Round/marker/eraser paths retain the capsule renderer. Other tips use distance-spaced antialiased stamps, with clipped iteration ranges for off-canvas input. A second sparse byte RGB map exists only for mixing materials; coverage remains floating point. Natural Pencil caches a byte of fixed fine canvas relief per touched pixel, preserving its approved graphite output. Crayon captures a per-stroke seed and crossfades two independent wax patterns using image-space arclength. Its two sparse byte caches rotate over two tip diameters of travel (minimum 32 px); obsolete generations are discarded. Retracing receives fresh material and accumulates coverage even within one gesture. Seeded, warped multiscale gradient noise exposes sparse, disconnected paper hollows in a mostly covered wax film; local relief integration and quadratic deposition retain blended tones, with intrinsic outer-tip falloff for softer sides. User opacity and selection are still applied once against pre-stroke pixels. See docs/PAPER_GRAIN_REFINEMENT.md. Oil/Watercolor lay down fine physical-width bristle trails with spatial lag, instead of rotating a striped footprint with each input segment. Flow accumulates before user opacity and selection coverage are applied once against the immutable source. Timed airbrush ticks continue through stationary UI frames, with bounded catch-up after stalls. Source-color pickup ignores hidden transparent RGB. Backend force is interpolated along the path; spacing is automatic and tilt/configurable spacing remain future work.

## Core editing workflow

Brush material defaults live in `Style::default_diameter`. The UI retains nine
independent session-local sizes plus the existing non-brush tool width; tool and
material transitions save/restore diameter without changing the document. A
transient canvas overlay shows the chosen footprint at viewport center while the
size control changes. It uses image diameter × zoom, with OS DPI handled by egui.
Oil and watercolor track total stroke arclength, including clipped travel, to
deplete a fresh per-gesture reservoir. Remaining load controls film deposition,
tip narrowing and individual filament dropout. This is independent of pointer
event rate and elapsed time. Oil retains richer physical bristle tracks;
watercolor has a softer wash and lower opacity ceiling. [Details and validation](docs/BRUSH_SIZE_PAINT_LOAD.md).

`editing` implements selection coverage/bounds/contours, active-layer copy, clear, paste, move and iterative scanline fill. Clipboard output bakes the active layer's evaluated appearance into flat RGBA; source filters/masks remain untouched. Paste creates a layer and expands canvas bounds if needed. Move clears selected source coverage and composites it at the new canvas location, preserving layer processing properties. The active source buffer can expand to the union of its old extent and the canvas; local masks are reanchored alongside it. Existing hidden pixels survive canvas cropping and bounds resizing.

`transform` owns whole-document crop, exact orthogonal pixel permutations and nearest/premultiplied-bilinear resizing. Layer positions, selection and local layer/filter masks follow the geometry. Layer filters/effects are retained and reevaluated after the transform. These whole-document operations include locked layers; locks protect local pixel tools. Size/offset errors leave the original document unchanged, and output allocations are preflighted against per-raster limits and an aggregate 512 MiB output-plane budget.

The app's editing controller runs expensive edits on a shared snapshot in a background worker. The result identifies its originating session/generation and commits one command only if still current. No-op jobs do not dirty the document; stale results cannot overwrite newer edits. A retained arboard clipboard object handles native image copy/cut/paste and X11 ownership; eframe image-paste events are also routed when text input has no focus. Clipboard write failure prevents cut deletion. Each session owns its in-flight selection gesture and a pointer-identity cache of selection bounds/contours. Its cached Arc prevents pointer-address reuse from returning a stale contour.

## File boundaries

`.palette` has its own magic, explicit format version, v6 descriptors/binary assets with canvas backing color and original selection coverage, v2/v3/v4/v5 binary migration and a separate v1 JSON migration reader and validated model invariants. The reader limits compressed and decompressed input before rendering. Encoding completes in a same-directory temporary file, syncs it, then atomically replaces the target. Unsupported versions and enum variants fail explicitly, preserving the existing file. Version migration belongs at this boundary.

Microsoft `.paint` uses a dedicated bounded HEIF item-graph adapter; primary-image raster decoding is insufficient. [Specimen evidence and validation gates](docs/PAINT_RESEARCH.md) describe the verified unci/iovl subset and why native Microsoft Paint acceptance remains pending.

## Text placement

`palette-core::text` uses Fontdue 0.9.4 for basic left-to-right layout, wrapping and grayscale glyph rasterization. The app supplies Ubuntu Light and Hack bytes from the already bundled `epaint_default_fonts`; core knows only font bytes and raster/model types. Font sizes and text-box coordinates use image pixels, independent of UI scale. Glyph coverage multiplies alpha, with normal source-over arithmetic for overlapping glyphs. The canvas selection constrains the final preview alpha once.

Click or drag with Text to start a transaction. An egui TextEdit supplies inline typing, caret and character selection in image coordinates; a floating formatting strip sits below the ribbon. Eight handles resize the box and its top edge moves it, converting pointer deltas through the current zoom. The left slider margin stays reserved when changing tools, avoiding a canvas-position jump. Menus and formatting remain above the text box in the layer order.

One bounded background worker rasterizes the draft with Fontdue; unique request tokens reject stale parameters and cancelled/replaced drafts. Done, Ctrl/Command+Enter, outside-click or a document/tool action requests commitment. If rasterization is pending, `AfterText` retains the requested file, tab, tool, image operation, resize, filter or history action and resumes it after commitment. OS close follows this path before the modified-document prompt. Cancel/Escape discards the draft and any queued action; Escape first dismisses an open popup. The transaction records document ID/generation and rejects stale insertion. Layer property editing is temporarily disabled during text placement.

Commitment inserts the Fontdue raster as a topmost layer in one undo command. The interactive egui layout and final Fontdue layout are separate implementations; exact glyph metrics, selection clipping and line spacing still require parity work. Committed text is raster content, with no persistent character/font object. Boxes are capped at 2048 x 2048 and content at 4096 characters. System fonts, complex shaping, bidirectionality and additional styles remain pending.

## Interface behavior

Menus, Save/Undo/Redo and document tabs share one 34-point command row. The fixed controls retain their positions; tabs occupy the remaining width before the ribbon toggle. Overflow stays inside a horizontal scroll viewport with a floating rail. Newly activated tabs scroll into view; the remembered content/viewport widths invalidate that reveal when DPI, font layout or window size changes, without undoing manual scrolling. Dragging a tab near a viewport edge scrolls toward offscreen drop destinations. Window decorations remain native, so left- or right-side window controls never overlap the application row.

`interface` owns logical-point metrics, theme and transient presentation state. `ribbon` uses original vector controls with short hover/press transitions, split-button flyouts and shared menu styling. Native egui popup placement handles anchors, nested menus, dismissal and available screen space. Menus use 32-point rows, rounded corners and soft shadows; animation timing approximates the supplied recording rather than reproducing WinUI exactly. The right layers tray animates its width and opacity over 180 ms, normally occupies 112 points, and expands to 180 for properties directly below the active thumbnail.

`ribbon::layout` negotiates section widths before rendering, in logical points. Shapes first uses a narrower vertically scrollable grid, then folds into a section button; Image, Tools and Colors fold next. The whole ribbon no longer scrolls. Expanded sections and section flyouts call the same controls, preserving editing commands and settings. Selection/Brushes have separate main/dropdown targets; folded sections have one whole-button target. `ribbon::controls` uses a root popup for the section and egui submenus for its nested controls, so opening Rotate, Flip, outline, fill or stroke width cannot replace the parent popup. Layout transitions dismiss stale anchors. The decorated native window enforces a 720 × 520 logical-point minimum; section collapse does not change the ribbon's 110-point height.

`app::appearance` owns centered color and canvas-property transactions. OK changes drawing color or submits one undoable background bounds edit; Cancel/Escape leaves that state unchanged. Custom color slots are session-only preferences. On compact windows, dialog bodies scroll while footers stay reachable. View toggles change presentation without entering document history.

`app::capture` supplies debug-only scene selection for deterministic native wgpu captures, waits for initial rendering and animation settling, and then exits. The screenshot suite compares fourteen reviewed Windows states, including common-shape/moved-layer, selection-hole, extended-catalog, custom rotation, selected-shape and polygon/curve preview scenes. Real egui pointer/key event tests cover interaction separately; scene captures do not claim native input automation. See [shape/layer validation](docs/SHAPES_LAYERS_VALIDATION.md) and [video reference](docs/UI_BEHAVIOR_REFERENCE.md).

## Selection animation

The core `editing::outline` returns directed, coalesced edges with selected coverage to the right. `editing::contours` links these into ordered loops; right-turn handling separates diagonally touching islands. The session caches loops alongside selection bounds by retained coverage-buffer identity. Animation does not reconstruct coverage, change source pixels, create revisions or invalidate the document renderer.

`canvas::overlays` paints a white one-point outline with black four-point dashes. A shared display-time phase travels around each contour at twelve logical points per second, independent of image zoom; visible outlines request repaint every 50 ms. The contour distance is retained when clipping to the workspace, so pan/zoom does not generate thousands of offscreen dash primitives or reset phase at clip boundaries. Creation rectangles and moving-selection outlines use the same painter. Diagnostic screenshots freeze the dash phase for repeatable baselines; normal editing animates continuously.

`canvas::polygon_selection` keeps a bounded, per-session click-by-click draft in image coordinates. Its mode is captured on the first click; panning, popups and toolbar presses cannot place vertices. Eight-logical-point start closure is independent of DPI/zoom. Enter, start closure or a double-click sends the accepted vertices through the existing antialiased freeform rasterizer and selection worker, feathering before combine algebra. Hover is only a rubber-band preview. Escape/tool/tab/save cancellation leaves the original document and history untouched; a completed polygon is one ordinary coverage-selection command, with the existing transform handles and native persistence. No vector object or new project format is introduced.

## Temporary shape adjustment

`floating` keeps a newly drawn shape separate from the pixels beneath its box.
`Document::floating_shape` holds boxed, copy-on-write original/source-shape rasters
and a stable layer ID. The active layer source is a presentation composite of
these inputs, so the existing filters, effects, masks, opacity and blending run
in their original order. No temporary layer is inserted into the layer list.
Selected transforms operate on the isolated transparent shape, then recompose
from the unchanged original, rather than cutting a rectangle out of the preview.
Incoming selection coverage is applied at creation; the adjustment box includes
transparent shape interiors. RGBA8 alpha quantization can change initial channels
by up to two levels relative to immediate painting. Hidden RGB in the original is
retained exactly where the shape is transparent.

History retains both floating inputs, with shared backing allocations counted
in the bounded undo budget. Eligible shapes share the selection-move gesture and
GPU compositor: worker-packed original/shape planes are uploaded once alongside
coverage. The shader translates only the shape, recombines it over the stationary
original, then applies layer opacity/blending once. Full-source offsets remain
separate from cropped appearance offsets. Input keys and memory/texture limits
cover both planes. Pointer movement updates uniforms without CPU render jobs or
pixel uploads; release performs the final reference transform and undo entry.
Masks, enabled filters/effects, arbitrary resize/rotation and unsupported adapters
retain CPU previews for correct processing. Clicking outside accepts
the shape and clears its box; Escape/Delete cancels just the shape. Another tool
can accept it while retaining the selection as its painting clip. Selection
coverage controls are disabled during shape adjustment. Undo can restore the
complete isolated state, including after commitment.

Native v8 stores the visible composite and omits the temporary box/state when
saving a pending shape; the in-memory document is unchanged. PNG/Paint exports
also use the composite. This adds no format version or persistent vector object.
See [validation](docs/FLOATING_SHAPES_VALIDATION.md).

## Shapes and whole-layer movement

`palette-core::shapes` supplies GUI-independent contours for twenty-one implemented tools. The app uses the same contour for drag preview and background raster commitment. The reference gallery contains 23 entries in 7/7/7/2 rows; Curve and free Polygon use per-session construction transactions. Preview outlines remove consecutive duplicate points and close a bounded open/beveled stroke, avoiding the unbounded miter normals of nearly flat closed paths. Exactly flat contours become open line segments. Concave preview meshes use ear clipping. Fills use even/odd scanline intersections, four vertical samples and analytic horizontal span coverage; outlines use one maximum-coverage PaintStroke over the filled result. Outline and fill have independent straight-alpha colors and shared opacity; the secondary button swaps foreground/background. Hardness is disabled for solid shapes. Layer offsets and grayscale canvas selections are accounted for before source pixels change. A cancelled preview has no model/history mutation, and stale worker results cannot overwrite newer edits.

`editing::translate_layer` changes only signed placement with checked arithmetic and lock handling. Source, local masks, filters, effects and canvas selection retain their data. `Session::layer_drag` retains original placement while worker renders progressively newer drag positions within one render epoch. Commitment restores the original placement long enough to record one history snapshot, then keeps the final placement. Cancel restores the original position without consuming a revision. Start/end/cancel fence stale preview renders with a new epoch. Tab/tool/file/close actions finish the gesture before proceeding. The move tool ignores the canvas selection, uses integer pixel deltas through the current zoom, supports Shift axis constraint and arrow-key nudging, and never resamples pixels.

New-document dimensions and Image Properties Default share the UI constant `DEFAULT_CANVAS = [1280, 720]`. Existing projects and imported image dimensions are unaffected; initial zoom remains 1.0.

## Selection-aware transforms and construction tools

The command controller routes orthogonal transforms to `transform::selected_orthogonal` whenever a coverage selection exists. Otherwise the existing whole-document path applies. Selected operations extract active-layer source and coverage into temporary planes, transform them, clear only old selected coverage and rejoin the result on the original canvas. They preserve layer identities, other layers and live processing properties. Selected coverage is transformed independently, including transparent selected pixels. Rejoining partitions old/new coverage into overlap and new-only regions, avoiding double alpha attenuation at fractional selection edges. Quarter turns use a stable integer-grid center to avoid accumulated half-pixel drift. Pixels outside canvas bounds are clipped; errors leave the original document unchanged.

`transform::rotate` dispatches exact multiples of 90 degrees to pixel permutations. General angles inverse-map pixel centers and interpolate premultiplied RGBA with transparent outside samples. Selected rotation retains canvas bounds; whole-document rotation expands bounds and rotates each independent source/mask/filter-mask plane, with checked offsets and bounded output planning. Native document serialization already represents the resulting pixels and offsets, so no format bump is required.

`shapes::paint_selected` commits source pixels and a clipped bounding selection together. The worker records one history snapshot; the originating construction tool becomes Select after success, enabling immediate move/flip/rotate. If the user changed tools while processing, the late result does not overwrite that choice. Shapes remain raster content, not persistent vector objects.

`Session::construction` stores either polygon vertices or a cubic baseline/control-point transaction. `canvas::construction` renders the same flattened path used by the CPU painter. Polygon previews include an explicit next-segment rubber band even in fill-only mode. Curve construction uses baseline plus two draggable controls. No source/history mutation occurs until finish; Escape or a tool/document transition cancels the draft. Construction parameters are captured at the start. Background edits still carry session/generation checks, so closed or changed documents reject obsolete results. The centered custom-angle dialog is a separate transaction, matching the supplied reference footprint, with current-field validation, selected initial value, Enter/OK and Escape/Cancel.

## Floating raster-selection transactions

`transform::SelectionTransform` provides atomic move, bilinear resize and arbitrary-angle rotation of active-layer selected source/coverage. Selection moves also use the overlap-aware alpha rejoining path. Resize validates source/output staging before allocation; unchanged geometry is a no-op. The canvas, unrelated layers and local processing properties remain intact.

`Session::selection_transform` retains the original COW document, original bounds, press position, handle, desired/applied operations and release state. Eight edge/corner handles use logical-point hit targets; the rotation handle sits 24 points above the selection. Geometry uses image coordinates, with a one-pixel positive minimum when edges cross. Shift constrains translation, maintains resize aspect ratio or snaps rotation to 15 degrees. Escape restores the original document without recording history.

The existing capacity-one render queue evaluates each transform from that immutable document on its background worker, then composites all layers/filters/masks and constructs the texture. The UI accepts progressively newer results in the gesture epoch, allowing pixel previews while the mouse remains down. Ants follow the processed coverage, and a cyan resize target responds immediately. No intermediate preview is resampled again. Stale epochs, closed tabs and obsolete errors cannot overwrite a new transaction. Selection topology caching still runs on the UI thread.

On pointer release, only the result matching the exact desired operation becomes one history command; the final texture is retained. Other commands call the same completion path before switching tools/tabs, saving or closing. If the exact preview has not arrived, that path computes the target synchronously, preserving intent at the cost of possible large-selection latency. Running worker jobs are fenced but not cancelled. These are CPU full-composite previews, not a GPU/tiled affine pipeline. Completed gestures change raster source data with undo; persistent vectors and nondestructive transform objects are future work. No native format change is required.

## Next boundaries

The next Phase 2 slices are typography parity, tablet hardware validation, configurable brush spacing and gesture-coalesced properties. The nine conventional brush families now render through the existing live dirty-region path. Persistent editable shape objects remain separate from the completed raster selection handles. Advanced selections and mask-painting UI follow later. Phase 3 local file work is now implemented as described below; real `.paint` fixtures/validation and platform execution gates remain open.

## Phase 3 file workflows

`io::project` writes version 6 with a bounded JSON manifest and sequential binary zlib assets. Raster descriptors reference shared 128 x 128 tile assets, preserving exact tile bytes and padding. Masks and opaque ICC/Exif metadata reference assets too. Arc identity deduplicates shared source allocations. Encoding streams each asset into the same-directory temporary file and backfills its compressed length; no whole-document pixel JSON or archive extraction is involved. Readers preflight all references/lengths/geometry, enforce aggregate decoded limits and verify exact decompression lengths/checksums before constructing a validated document. v1 remains a separate migration reader; v2 binary assets load with transparent backing. See FILE_FORMATS.md for the wire specification.

AVIF and neural processing have been removed, together with their codec/runtime dependencies and model assets. Native project and Paint handling remain independent of those features. See [dependency audit](docs/DEPENDENCY_AUDIT.md).

`app::files` owns import, recents, export options and recovery presentation. Import snapshots the originating session/generation, decodes off-thread and inserts through existing paste semantics in one command. A changed or closed target rejects its result. File/tab actions requested during inline text are queued through AfterText. File drops wait until dialogs/drafts are finished.

`persistence::Store` uses a unique OS file-lock lease per app instance. Atomic recent metadata retains ten paths. Approximately every 30 seconds, one background job saves a stable dirty document to an independent native recovery project and a small sidecar; originals and saved revisions are untouched. Other running instances are skipped during discovery. Restore creates a dirty session and retains the old recovery until an explicit save or replacement recovery succeeds. Save/discard/close remove owned copies; a late autosave completion rechecks whether its tab is already closed/clean. Window close waits for a pending recovery write before resuming the normal save prompt, avoiding resurrected discarded state. Storage paths, limits and recovery caveats are documented in docs/FILE_FORMATS.md. No uncommitted text/path drafts or history are serialized.

## Verified Paint container boundary and canvas backing

`io::paint` contains bounded box/location readers, direct-source graph interpretation and a separate writer. It never guesses layers from all top-level images. Primary iovl dimg order defines bottom-to-top layers, and its fill becomes `Document::background`. CPU regions initialize from that color before source-over compositing. Imported sources remain straight RGBA8; no hidden white layer is fabricated. The native v3 manifest preserves this backing through history/save/reopen; v1/v2 migrate with transparent backing.

The adapter supports the independently inspected packed 8-bit RGBA subset. Unknown nonessential properties are tolerated; unsupported essential properties on the used graph and unsupported codecs/transforms fail. Item, box, extent, dimensions and decompression budgets are checked before pixel construction. libheif was evaluated first; the safe Rust binding lacks overlay creation and the tested official Windows libheif build disables uncompressed decoding. A narrow safe Rust implementation avoids a new native C++/LGPL distribution boundary. General HEIF integration remains separate, and native Paint application acceptance is still required.

Paint export is a distinct copy operation. A compatibility dialog pins session and history revision; unsupported state requires a successful full native save of that revision before conversion. Local processing is baked per layer; non-Normal blending requires merging the minimum bottom prefix. The original source model is untouched. This is deliberate compatible export, not a silent native-save downgrade.

## Phase 4 blend and coverage operations

`composite::blend` implements separable RGB equations and the W3C nonseparable luminosity/saturation helpers. A whole RGB triple is mixed before the existing straight-to-premultiplied source-over arithmetic, so Hue/Saturation/Color/Luminosity do not incorrectly act on channels in isolation. The blend catalog is `BlendMode::ALL`; labels are shared by layer properties and the generated visual reference. The working blend space remains sRGB.

`selection` owns fractional set operations, Wand sampling, feathering and canvas-to-layer mask conversion. Union is a+b−ab, intersection is ab, subtraction is a(1−b), normalized with integer rounding. Replace/Add with no existing selection starts from incoming coverage; subtraction/intersection treat no constraint as the whole canvas. Wand compares premultiplied Oklab coordinates plus alpha, ignoring hidden RGB at alpha zero. It resolves connectivity before antialiasing, uses a bounded scanline frontier and 4096-entry color cache, and samples current evaluated layer pixels or a CPU visible composite. Antialiasing is a one-pixel separable tent.

Feathering below 5 px integrates a Gaussian over source and destination pixel areas; a positive convergent erf series evaluates the resulting second-difference kernel. This avoids whole-pixel box widths collapsing small radii to identical results. At 5 px and above, three separable running-sum box widths approximate a Gaussian of the requested sigma. It is linear in pixel count with two byte planes and clamped borders. Mask creation transforms canvas selection coverage into source coordinates through signed layer offsets. Mask properties never overwrite the source. Edits run on existing session/generation-fenced workers and commit one history command. The feather dialog pins both IDs and discards pending parameters on Cancel or document changes; selection feathering now has live preview as described below, while layer-mask feathering remains an Apply-only edit. `Session` owns selection-combination and Wand settings per tab. Visual ants use the 50% contour, while bounds and edits retain all nonzero coverage. Replace mode permits handle transformations; Shift on a handle constrains it, whereas Shift in the interior starts an additive selection.

## Selection toolbars and freeform selection

pp::selection_toolbar overlays an automatically sized foreground frame at the workspace’s top, using the text toolbar’s shared 28-point controls, 8-point margin and popup frame (46-point outer height). It centers within the canvas workspace and reduces slider width when layer properties constrain a minimum-sized window. The measured rectangle excludes pointer input from the canvas before gesture dispatch, including the first frame after a tool switch. Numeric fields reject nonfinite/out-of-range parameters. Tool settings are per session; text focus suppresses image shortcuts. The four original combine icons and inversion operate on the existing coverage model.

Session::lasso_drag retains image-space points, combine mode and a live closed ants preview. Samples closer than half a logical display point are skipped; very long paths decimate at 4096 points while retaining endpoints. Release closes the path and dispatches a generation-fenced worker; Escape/tool/tab changes cancel the uncommitted contour. Core selection::freeform validates dimensions/points and shares shapes::fill_coverage, an antialiased even/odd span rasterizer independent of egui. Committed lasso selections support the same move/resize/rotate handles as rectangles.

Toolbar feathering sets the next region and previews the current selection: worker jobs feather incoming rectangle/lasso/Wand coverage before combining with existing selection, committing one history step. Zero-feather rectangle behavior retains the existing synchronous path. The existing Feather selection dialog previews from the retained original and commits only on Apply. Native v4 projects preserve resulting coverage and its retained original contour; transient tool settings and raw lasso drafts are not project data. Edge-aware selection brush is the next requested slice.

## Selection provenance, clipped boundaries and canvas rails

`Document::selection_base` is optional original coverage, separate from the feathered `selection` used by painting, clipboard, filters and masks. `selection::apply_region` combines both original and evaluated planes consistently; `feather_selection` evaluates from the original instead of repeatedly blurring the last result. Inversion maps both planes, geometry maps both through identical anchors/sampling, and fresh plain replacement/deselect clears provenance. Immutable asset sharing and history budgets account for both. The v4 writer protects this state from older readers silently discarding it; v1–v3 remain readable without rewriting their inputs.

The session contour cache keys both retained buffers, dimensions and enabled states. Opaque ants trace the original at half coverage; 40%-opacity ants trace nonzero evaluated coverage, showing the actual feather support after byte quantization. A display-pixel border scan continues selected runs along the visible canvas intersection, including clipped inverted regions, with a consistent perimeter animation phase. These are presentation-only lines: they do not change selected pixels, the canvas dimensions or history.

`canvas::navigation` resolves coupled horizontal/vertical overflow, reserves 14-point rails within the existing workspace and outside the layers tray, then maps thumb travel to the same per-session `pan` used by the wheel and middle/Space drag. End positions retain a small canvas-edge margin; nonoverflowing axes center, and Fit removes the rails. Pointer hit testing uses the reduced viewport. Thumb grabs retain their initial offset; rail clicks seek, focus enables arrow/page/Home/End controls, and image shortcuts yield while a rail has focus. Canvas presses return focus to editing.


### Live feather transactions

`SelectionFeather` pins immutable original coverage, its contour cache, tab generation and a gesture token. A single bounded worker computes full-resolution coverage, bounds and feather contours off the UI thread. Only one job runs at a time; newer desired radii replace queued intentions rather than accumulating full-image jobs. Completed intermediate frames can be displayed while the newest radius computes. Results from cancelled gestures, closed tabs or obsolete generations cannot overwrite current state.

Dragging changes presentation only. Toolbar release or dialog Apply requests a final commit; the exact final result becomes one history command, with its worker-built contour cache installed directly. Escape/Cancel discards the transaction. An explicit tool/tab/save/undo change finishes a pending toolbar gesture before proceeding, using a synchronous final evaluation if necessary; an unapplied modal draft is discarded. Selection edits retain original coverage, so slider changes are absolute and zero restores the source. Mask feathering retains its existing Apply-only behavior.

Selection sliders map real values through a shifted logarithm with a 10-unit offset. Midpoint values are approximately 23 for tolerance and 27 for feathering, retaining precision near zero without the former extremely steep curve. Custom text parsing/formatting exposes actual units, and 0.1 increments plus one decimal below 5 px remain supported. Feather ants use alpha 102/255; original ants remain opaque.


### Brush selection and bounded feathering

`selection::brush::Stroke` samples an immutable document snapshot and incrementally rasterizes continuous round capsules with the shared brush footprint. Coverage is the maximum per pixel during a stroke, so pointer-event density and retracing do not increase strength. Edge-aware mode retains the initial Oklab-plus-alpha sample and four-connected matching coverage inside the swept footprint; hidden transparent RGB cannot create false edges. Plain mode bypasses color/connectivity gating. Local sampling includes layer processing/offsets/opacity, while merged sampling renders the visible canvas once per stroke. Locks do not block read-only selection sampling.

A session-owned `BrushSelection` draft captures settings, combine mode, feather, source generation and an append-only path. A bounded worker keeps incremental capsule state, accepts coalesced path snapshots, combines feathered/raw masks against the pre-gesture selection, and computes contours and overlay pixels. A token and generation fence every result. The UI displays drafts without changing source pixels or committed selection coverage. Release waits for the exact final frame and records one history step; Escape discards. Explicit tool/tab/save/undo actions finish a pending target synchronously if necessary. Worker scratch state is released when the gesture ends. Overlay textures are keyed by generation and optional, and their size respects GPU limits. Settings/history remain per document; only committed coverage and original contours enter native v4 files.

`feather_region` crops sparse coverage with conservative padding larger than both blur kernels' finite support. Interior crop edges contain zeros, and original canvas edges retain clamp behavior. Full-size output coverage is restored after processing; dense/disabled masks use the reference full-canvas path. Tests compare every byte to `feather`, including subpixel radii, large radii and canvas boundaries. Directed contour scans visit nonzero bounds while preserving loop/run order. Empty bounds use lazy construction to avoid unsigned subtraction on empty selections.

Raster `ImportReport` carries a document and transient detected-format notice. This keeps animation/page loss explicit in both open and layer-import workflows without adding warnings to serialized project metadata.

## Phase 5 live processing

`processing::REGISTRY` owns the implemented catalog, typed parameter domains, labels and valid defaults. The document enums retain typed serialized parameters; UI descriptors do not enter project assets. Validation checks finite/range constraints and padded style allocation estimates. Native v5 preserves the current variants and rounded-stroke boolean; older readers reject the newer version explicitly.

`processing::gaussian` processes 256-pixel output tiles with complete input halos. It filters fixed-point premultiplied RGBA together, uses a sampled Gaussian below sigma 2 and a three-box approximation otherwise, and safely unpremultiplies to RGBA8. Alpha-zero hidden RGB never bleeds into visible edges. Scratch is bounded by tile plus halo; output remains a complete raster. Source edges clamp. `processing::layer` executes ordered filter stages, mixes individual filter masks in premultiplied space, applies Color Overlay then the layer mask, and derives padded shadow/glow silhouettes from the resulting alpha. Spread is square max-dilation; size controls the remaining blur. Shadow angle describes the light direction and its fractional translation uses bilinear alpha sampling. Outer Glow excludes foreground coverage. Underlays compose in effect-list order, foreground goes above, then layer opacity/blend is applied once. Canvas bounds clip final output; original sources stay intact.

`composite::Renderer` retains at most eight prepared spatial layers within a conservative 128 MiB budget including source and mask snapshots. Equality of the complete layer invalidates any changed source/parameters/masks/properties; unrelated layer edits can reuse it. Over-budget layers render without cache retention. This is evaluated-layer caching, not an intermediate/tile cache. Region results use complete neighborhoods before clipping. Bulk clipboard/Wand/brush sampling reuses one prepared layer rather than reevaluating it per pixel. Paint compatibility baking renders spatial styles through the same compositor before clearing their copied stacks.

`app::processing` pins a contextual canvas draft to the original session, generation and layer. Its separate capacity-one worker reduces a shared snapshot to at most 2048 px, scales image-space parameters and renders with an atomic cancellation ticket. Superseded jobs/results are cancelled/discarded. Apply validates and commits full-resolution parameters as one history command; Cancel, a changed tab or OS close discards the draft. The original document, autosave and native saving never contain pending parameters. Preview resize itself and general canvas render jobs are fenced but not interrupted. Spatial canvas textures and their thumbnails are built in the existing render worker, with generation/epoch guards.

[Phase 5 evidence and benchmark limits](docs/PHASE5_VALIDATION.md). Mathematical references: [W3C compositing](https://www.w3.org/TR/compositing-1/) and [SVG filter effects](https://www.w3.org/TR/SVG11/filters.html); independently implemented Rust, with no new dependency or proprietary source code.

### Contextual layer processing controls

The selected layer's property card has three original geometric icon buttons beneath Opacity: Filters, Effects and Blend. Blend uses an anchored scrollable popup; the other buttons select a session/layer-bound `processing::Toolbar`. Text, Selection and processing bars are mutually exclusive. The processing area shares the 28-point controls/8-point frame and dynamic-width anchoring; its measured rectangle fences canvas pointer handling before any tool dispatch. A parameter selector exposes one setting at a time so multi-parameter styles also fit compact windows.

Processing drafts remain separate from `Session::document`. The canvas uses the current draft worker texture when its session, generation and layer match; preview-off, Cancel, tool/tab/layer changes or stale jobs fall back to committed presentation. Pending previews retain the last accepted texture and paint-only progress status. Status changes cannot alter control IDs midway through pointer/menu interaction. Apply commits parameters as one history command and retains a ready preview for presentation until the normal worker installs its full result; it never treats reduced pixels as source or saved document data. Stack removal is independently undoable and does not change source assets. Tests inspect the actual canvas texture mesh as well as model/source equality.

### Expanded processing catalog

The shared `processing::point` evaluator now serves both point compositing and staged spatial filters, avoiding divergent adjustment mathematics. Its alpha-preserving operations cover contrast/brightness, linear-light exposure, HSL hue/saturation/lightness and vibrance, encoded-RGB grayscale, threshold and posterization. `processing::neighborhood` reuses the Gaussian fixed-point box pass for tiled Box Blur and uses per-window rank selection for Median. Both operate on premultiplied 16-bit intermediates; RGB/alpha are filtered together and safely unpremultiplied. Box scratch is tile-plus-halo bounded, while Median retains only four small window vectors beyond source/output rasters. Cancellation checks discard partial outputs between rows/passes.

Spatial styles retain ordered foreground recoloring and background contributions. Inner Shadow/Glow sample the complementary shifted/blurred silhouette, recolor the existing masked foreground RGB and preserve its alpha. Spread erodes the foreground for inner styles. Stroke interpolates whole-radius square dilation/complement dilation for subpixel widths and supports outside/inside/centered placement; center retains two planes, counted in allocation preflight. Original source assets remain untouched, and final layer opacity/blend is applied once. Current variants share native v5 descriptors and reject invalid values before rendering/encoding.

Registry parameter metadata includes booleans and finite enumerations as well as numbers/colors. Stroke placement serializes as a named model enum and maps to stable choice indices only in the UI adapter. Integer filter fields reject fractional values. Filter menu categories enumerate registered implementations and disable empty categories; toolbar Add lists scroll. Preview scaling also scales stroke widths and rounds whole-pixel Box/Median radii. See [catalog validation and limits](docs/PROCESSING_CATALOG_VALIDATION.md).


### Style controls and six more conventional filters

The registry retains typed source-preserving filters and ten styles; subsequent catalog batches are described below. `processing::extended` evaluates premultiplied bilinear Motion Blur, alpha-preserving Gaussian High Pass and premultiplied block-area Mosaic. Point processing now takes layer-local coordinates for deterministic seeded Uniform/Gaussian Add Noise; partial/full/staged evaluation samples the same field. Levels validates ordered black/white inputs before any mutation; its gamma/output transfer and five-knot cubic-Hermite Curves preserve alpha. Curves knots are fixed at inputs 0/64/128/192/255 with editable output values. Spatial preparation and preview scaling recognize the new operations; High Pass always uses preparation, including zero radius (neutral gray).

Registry parameters include booleans. Glow exposes Radius (serialized legacy `size` stays intact). Shadow Angle/Distance/Spread and Stroke Width/Rounded corners are accessible in the same 46-point contextual bar. Shadow quick fields appear only when space permits; narrow layouts retain all parameters through the selector. Defaults select the requested geometric property. Draft editing remains source-preserving and cancellation-ticketed.

`processing::round` independently implements a separable lower envelope of squared-distance parabolas. Nonzero alpha contributes a subpixel site cost `(1-alpha/255)^2`; the resulting Euclidean contour has a one-pixel coverage ramp. Complement growth supplies erosion for inner strokes. Square mode retains fractional box morphology; center splits width between its two planes. The distance transform is linear in pixel count and uses a full f32 distance plane, byte coverage planes and line buffers. Existing retained-style preflight and LRU limits still apply; they are not a total-process memory cap.

Native v5 uses the same manifest/assets layout and adds Stroke `rounded`. Missing fields in old projects default false; new editor strokes default true. Version dispatch rejects v5 in older builds that would otherwise silently ignore this boolean on a known variant. Readers still accept v1–v4. This supersedes historical v4 writer statements above; see [current validation](docs/STYLE_FILTER_CONTROLS_VALIDATION.md).


### Conventional menu and further spatial kernels

`processing::menu` owns the researched branch/name ordering independently from operation availability. Each entry resolves through `REGISTRY` using category/name; only actual typed operations can activate. All implemented filters must resolve exactly once (tested). This replaces implementation-order menu construction; Add lists still enumerate only registered implementations. The disabled Gallery entry heads the menu; unavailable conventional operations stay disabled. No correction, Liquify, Vanishing Point or plugin sections are created. Long submenus have bounded scroll areas and visible rails, sized to the current window.

`processing::conventional` evaluates Average, per-channel Sobel Find Edges, directional grayscale Emboss, thresholded Gaussian Unsharp Mask, separable RGB extrema and inverse-mapped geometric operations. Solarize remains point-local. Spatial evaluation checks cancellation by row/column; extrema use monotonic queues and six-byte/pixel intermediate storage. Alpha is preserved for contrast/extrema operations; entirely transparent extrema sites are excluded, and gradient/emboss samples composite through center color to avoid hidden-RGB fringes. Average and geometric sampling deliberately evaluate premultiplied RGBA. Offset and Wave serialize named Wrap/Repeat/Transparent boundary policies; Twirl uses clamped bilinear sampling around source center. Preview scaling includes signed offsets, wavelength, amplitude and Emboss height with correct minimum/integer bounds.

All ten new variants fit the existing v5 native schema; older v5 readers reject unknown variants explicitly. Source buffers, masks, history and cache boundaries remain unchanged. [Current validation and exact limits](docs/FILTER_MENU_VALIDATION.md).


### Creative and additional blur filters

`processing::creative` adds Surface Blur (Gaussian spatial/linear color-range weighting with alpha-weighted RGB and retained center alpha), premultiplied Spin/Zoom Radial Blur and four-copy Fragment. Poster Edges reuses tested Sobel and minimum-neighborhood stages to darken quantized source colors. Spatial jobs poll cancellation at rows/stages and every 64 pixels in expensive sampling loops. Gaussian/range taps and radial transforms are precomputed once per evaluation; no full window allocation occurs per pixel.

Clouds are five-octave smooth value noise using source-local coordinates and stored seeds, scale and RGB endpoints. Difference Clouds performs an opaque Difference blend over the original RGBA; both introduce coverage and deliberately use prepared spatial layers, including premultiplied filter-mask mixing. They remain within source bounds and participate in existing masks, cache/history and styles. App creation captures current brush/background RGB; editing an existing filter preserves its saved colors.

Grain is point-local and coordinate-seeded, with typed Regular/Soft/Speckle/Horizontal/Vertical styles, amount, contrast, scale and optional color noise. Soft samples between lattice sites so it differs from Regular at default scale. Grain retains alpha and is consistent across point and staged evaluation, including fractional filter masks. Preview scaling now includes procedural scale, grain size and edge thickness, clamped/rounded to each parameter's domain. New enum variants fit the v5 native manifest; older readers reject unknown variants. [Current evidence](docs/CREATIVE_FILTERS_VALIDATION.md).


### Noise cleanup and additional distortions

`processing::retouch` evaluates Despeckle, thresholded Dust & Scratches, luminance/chroma Reduce Noise, and fixed Sharpen/More. Relative-coverage neighbor lookup retains the same RGB response at uniform partial alpha while fading missing/less-covered samples to center color. Despeckle blends Gaussian smoothing through a Sobel guard; Dust reuses premultiplied median evaluation, copies RGB only where differences exceed threshold and keeps original alpha. Reduce Noise precomputes Gaussian taps and separate range weights, reconstructing RGB from blended Rec.709 luma/chroma; all output coverage remains original. Sharpening uses four-neighbor high boost with fixed 0.25/0.5 amounts. Jobs poll cancellation at rows and within costly neighborhoods.

`processing::distort` supplies circular Pinch and Spherize inverse mappings plus concentric Ripple. The source-bound center is unchanged; circular effects preserve pixels at/outside the support. Each destination samples premultiplied RGBA bilinearly with clamped boundaries. Neutral controls bypass; radii, ripple amplitude and wavelength participate in reduced-preview scaling. Typed v5 enum variants preserve all parameters without changing the native container. The existing registry/menu/preview/history/cache interfaces are reused. [Current validation](docs/RETOUCH_DISTORT_VALIDATION.md).


### Movable origins and next conventional filter batch

`Filter::center` and `source_center` define normalized source-local sample centers. Existing geometric evaluators use this shared mapping; Wave subtracts the origin before computing phase. The UI's `processing::origin` maps logical pointer coordinates through the canvas zoom and layer offset, then subtracts the half-pixel center offset. A primary press on the visible layer starts placement; drags clamp at source bounds, final release coordinates are retained, and Space/middle-drag remains navigation. Popups/toolbars suppress placement. The clipped crosshair is presentation only. Parameter edits and mouse moves increment the same cancellation ticket, leaving source/history untouched until Apply. Cancel, tab/layer changes and stale-generation fencing use the existing transaction. Numeric X/Y and reset use the registry.

`processing::studio` provides independent Polar Coordinates inverse conversion, radial ZigZag displacement with smooth support, anisotropic seeded Fibers and Gaussian-guided per-channel Sobel Glowing Edges. All route through spatial preparation and premultiplied filter-mask mixing; generators create coverage, so point-only alpha-preserving evaluation is inappropriate. Fibers captures foreground/background colors on creation and uses normalized coordinates to retain structure in reduced previews. Geometric sampling uses premultiplied bilinear RGBA. Glowing Edges preserves source alpha and relative neighbor coverage, avoiding hidden-RGB contamination.

Native **v6** protects custom origin fields from older readers. Existing projects default circular centers to the midpoint and Wave phase to the top-left. New variants, parameters, source/mask assets and stack identities remain fully editable. This supersedes historical v5/current-center statements above. [Scope, references and test evidence](docs/ORIGIN_FILTERS_VALIDATION.md).


### Artistic, print and texture batch

`processing::cutout` builds a 32³ encoded-RGB histogram weighted by original alpha, then learns a maximum eight-color palette using deterministic population/farthest-point seeds and six bounded Lloyd iterations. A Gaussian guide and fidelity blend govern boundary simplification. Training excludes hidden RGB; output retains original coverage. Histogram storage is about 1 MiB; each pass polls cancellation and palette work is bounded independently of document dimensions.

`processing::artistic` shares coordinate-seeded Film Grain/Craquelure between fast point composition and staged processing. Film Grain shapes signed noise and suppresses it smoothly in highlights. Craquelure shades thin cracks from local jittered-site distance gaps. Accented Edges and Ink Outlines use alpha-aware Sobel gradients; Photocopy combines tonal darkness with detail-dependent edges. Halftone Pattern colorizes Dot/Line/Circle screen coverage, using a radial area CDF for dot tones and four subpixel samples. Patchwork computes coverage-weighted RGB cell means and a shallow bevel. All eight preserve input alpha, so point-only filter-mask RGB interpolation is appropriate for the two point operations; the remaining six route through spatial preparation.

Sketch low/high color fields capture foreground/background only for new instances, using the same workflow as Clouds/Fibers. The common numeric/color/choice controls, cancellation ticket, source/mask/history/cache boundaries and native v6 manifest are reused. New enum variants cause explicit errors in older readers, requiring no layout/version change. Radius/width/length/grid settings scale in reduced previews with valid lower/integer bounds; implicit tiny kernels and point noise remain approximations. [Current scope and evidence](docs/ARTISTIC_FILTERS_VALIDATION.md).


### Pixelate, cells and diffusion

`processing::cellular` evaluates seeded Crystallize, Pointillize and Stained Glass. Site geometry uses global source coordinates and true partial-cell extents. Source colors are alpha-weighted means per site. Each 512×512 output block retains only its relevant cell accumulators (about 2.1 MiB maximum at size 2), revisits source pixels through a two-cell training halo and recomputes output labels. This avoids a full-image u32 label plane and keeps working-cell storage independent of image dimensions. Source-local coordinates and halo contributors prevent processing-tile seams. Stained Glass measures candidate bisector distances for borders; Pointillize renders round antialiased dots over a saved background color. All retain original coverage.

`processing::pixelate` evaluates four-angle subtractive Color Halftone screens, fixed minimum-variance Facet quadrants, square Mosaic Tiles/grout and seeded RGB Diffuse modes. Relative neighbor coverage avoids hidden-RGB contamination. Anisotropic mode uses one stable four-neighbor range-conductance step; seed input is disabled while that mode is selected. Mezzotint shares a point/staged evaluator with ten named mark footprints and stored seed/scale/monochrome. The shared dot-area CDF lives in `processing::artistic` and retains previous Halftone Pattern output. Pointillize captures the drawing background color, Stained Glass captures the foreground only at creation; subsequent edits preserve saved colors.

All new variants use native v6 without layout changes. Common parameter domains, cancellation, preview/history/cache and mask compositing remain reused. Cell/tile/radius/mark dimensions scale with valid preview minima. The native-backup guard precedes baked Paint export. `examples/filter_inventory.rs` counts implemented/remaining entries directly from the conventional menu and registry; it separates effects/Gallery and imposes no future catalog cap. [Current implementation details](docs/PIXELATE_FILTERS_VALIDATION.md).

### Color adjustments and local tonal correction

`processing::adjustments` shares point evaluation between direct composition and ordered spatial pipelines. Color Balance stores a 3×3 tonal RGB matrix; Photo Filter attenuates linear-light RGB. A neutral-axis shift plus gamut compression preserves encoded Rec.709 luma when requested. Channel Mixer stores three RGB-plus-constant rows; monochrome evaluates the first row while retaining all rows. Gradient Map stores two RGB stops, midpoint and reversal; drawing colors are captured only when adding a new instance. Selective Color stores all nine conventional color-range CMYK coefficient rows simultaneously, using soft original-pixel memberships and relative/absolute subtractive RGB corrections.

Shadows/Highlights evaluates two independently scaled Gaussian neighborhood guides (sharing one result for equal active radii), then adjusts tone while preserving original coverage. Hidden RGB does not enter the premultiplied guides. Inactive corrections bypass their guide; cancellation is checked in Gaussian stages and output rows. Fixed Blur/Blur More reuse the bounded 3×3/5×5 premultiplied box implementation. These independent equivalents do not claim proprietary algorithm parity. New variants use the existing v6 layout and generic history/mask/native-save paths.

Registry domains drive numeric/color/boolean controls. A percentage slider can report an idle display-rounding change; only an actual accepted model change invalidates its preview ticket. This prevents repeated cancellation/starvation of live preview workers. Compact luminosity controls retain their parameter caption and tooltip without overflowing the 46-point toolbar. [Validation](docs/ADJUSTMENT_FILTERS_VALIDATION.md).

### Traditional-media painting filters

`processing::painting` evaluates eight Artistic/Brush Strokes equivalents through the existing ordered spatial pipeline and evaluated-layer cache. Source-coordinate patterns, bounded scratch and repeatable seeds avoid animation or processing-tile seams. Every output retains its original destination alpha; premultiplied bilinear samples fade low-coverage neighbors toward the center color, while least-variance brush sectors exclude hidden RGB through alpha weighting.

Dry Brush and Paint Daubs choose among four overlapping 3×3 sampled sectors. An RGB-scale variance tolerance stabilizes mathematically equal sectors despite cancellation in second-moment arithmetic. Three `DaubStyle` geometries are serialized by named enum variants. Colored Pencil uses finite crossed segments plus local edge retention and saved paper RGB/brightness; Dry Brush quantizes common colors and adds stored-seed dry texture. Smudge Stick, Angled Strokes and Dark Strokes share a bounded nine-tap diagonal sampler with different tonal rules. Crosshatch uses three progressively weighted finite hatch orientations. Spatter samples a deterministic disk-displacement field, then optionally smooths the recolored result without replacing original alpha.

Registry domains validate all scalars, integral seeds, colors and enum choices. Brush/stroke dimensions scale with valid minima in reduced previews; source patterns may shift under resampling. New descriptors append to the existing registry so established toolbar Add entries keep their order; conventional menu order remains catalog-driven. New instances capture pencil paper from the straight-RGB drawing background. Editing an existing instance retains saved paper and seeds. Native v6, masks, history, Apply/Cancel and worker cancellation reuse their existing paths. [Scope, references and measurements](docs/PAINTING_FILTERS_VALIDATION.md).

### Directional spray, ink and paper

`processing::drawing` evaluates eight source-preserving equivalents using the existing ordered spatial pipeline and layer cache. Sprayed Strokes combines nine axis-aligned, range-weighted RGB samples with a deterministic perpendicular displacement. Sumi-e uses an alpha-weighted Gaussian color guide and pressure/contrast/edge terms. Both retain destination alpha. The relative-alpha bilinear sampler is shared with `processing::painting`, avoiding hidden-RGB contamination and keeping uniform partial-alpha colors stable.

Sketch instances capture foreground/background as saved straight-RGB ink/paper only on creation. Graphic Pen uses four subpixel samples of finite parallel marks; Charcoal adds blur, edge retention and discrete ink coverage; Chalk & Charcoal combines independent dark/light tonal windows over an ink/paper midpoint. Stamp evaluates a smoothed two-color threshold with exact balance endpoints. Note Paper adds seeded grain and a fixed-direction height bevel to soft tonal holes. Bas Relief evaluates luminance-derived normals against a precomputed azimuthal light at fixed elevation. Normalized luminance is clamped before fractional powers to keep saturated or rounded samples finite.

`StrokeDirection` stores four named axes shared by spray and pen; its numeric UI index is not serialized. Registry domains validate numbers, seeds, colors and direction choices. Zero-length/zero-radius spray remains an exact bypass under preview scaling. New variants retain native v6 layout, history, masks, Apply/Cancel and cancellable worker paths. Sampling work per output pixel is bounded; full-layer CPU evaluation and preview resampling limits remain. [References, tests and measurements](docs/DRAWING_FILTERS_VALIDATION.md).

### Complete Sketch catalog and generated surfaces

`processing::sketch` evaluates Chrome, Conté Crayon, Plaster, Reticulation, Torn Edges and Water Paper. It shares relative-alpha neighborhood sampling with painting/drawing, and shares bounded luminance, gradient and color mapping helpers with drawing. Every operation retains destination alpha; transparent hidden RGB does not enter guides or materials. Source-coordinate seeds and material coordinates remain stable across raster tiles and redraws.

Chrome evaluates a grayscale folded reflective height/slope ramp. Plaster forms raised dark regions from a soft threshold of inverted luminance, then shades their normals using precomputed light azimuth at fixed elevation. Conté Crayon stores a named `SurfaceTexture` (Canvas, Brick, Burlap or Sandstone); procedural heights drive tooth coverage and directional relief. Four material functions avoid asset licensing/deployment issues and leave imported textures for a future extension. Texture size is an image-space period scaled for previews; relief, emphasis and angle are dimensionless/degrees.

Reticulation combines coherent clumps and fine grain with monotonic foreground/background emphasis. Torn Edges perturbs a finite tonal band around a soft cut, with exact all-paper/all-ink balance endpoints. Water Paper uses a Gaussian guide, a source-anchored fiber orientation field, nine broad range-weighted samples and fiber/blotch modulation, followed by contrast and bounded brightness. It preserves RGB colors; Chrome stays grayscale. The other four capture drawing colors only for new instances and retain stored colors during later edits.

All variants reuse native v6, typed registry validation, masks, history, cache, preview cancellation and Apply/Cancel. New descriptors append without changing established Add ordering. Full-layer CPU processing and reduced-preview differences remain explicit limits. [Reference algorithms, tests and measurements](docs/SKETCH_COMPLETION_VALIDATION.md).

### Complete Artistic catalog

`processing::media` evaluates eight independent painted-media filters through the ordered spatial stack. It reuses alpha-weighted, bounded stratified sector means and relative-alpha bilinear sampling from painting; luminance/gradients from drawing; procedural surface heights and normal lighting from sketch. Fresco uses rounded low-variance pigment dabs; Palette Knife uses elongated sectors; Sponge uses coherent pores and local contrast. Watercolor simplifies color, increases edge saturation and pools dark pigment. Plastic Wrap adds a fixed-light specular term from a smoothed luminance height field. Rough Pastels combines diagonal chalk with material tooth; Underpainting combines coarse paint with material coverage and relief. Every output retains original alpha, excluding hidden RGB from neighborhoods.

Neon Glow captures foreground/background only at creation, retaining editable saved colors alongside its glow color. Its signed size moves a tonal band and is explicitly excluded from image-space preview scaling. Rough Pastels/Underpainting reuse named SurfaceTexture values, size, relief, azimuth, inversion and u16 seed. New descriptors append to preserve Add ordering. Native v6 layout remains unchanged; unknown variants fail explicitly in older readers. Workers retain cancellation fences and Apply/Cancel transactions; CPU full-layer/cache and reduced-preview limits remain. [Algorithms and validation](docs/ARTISTIC_COMPLETION_VALIDATION.md).

### Optical/texture and edge-processing filters

`processing::optical` adds three spatial families to the ordered stack. Glass and Ocean Ripple inverse-map pixel centers using premultiplied bilinear RGBA; named boundary modes can repeat, wrap or clear outside the source. Their generated height/phase fields use normalized source-local origins, shared mouse placement and source-coordinate seeds. Diffuse Glow uses the same origin mechanism for a highlight veil/falloff, retaining coverage and capturing the current background RGB only on creation. Texturizer shares generated SurfaceTexture materials and normal lighting with Sketch.

Shape Blur builds a normalized disk/regular-polygon aperture from four coverage samples per kernel cell, then convolves premultiplied RGBA. A cancellable alpha-bounds scan conservatively skips transparent work outside the expanded kernel support. Smart Blur uses alpha-weighted compact spatial/range weights; its Normal, EdgeOnly and OverlayEdge modes retain original coverage. Sharpen Edges gates a four-neighbor high-frequency term; Smart Sharpen performs bounded, damped Gaussian back-projection with a soft residual threshold and final amount blend. RGBA8 estimates bound memory per iteration but impose quantization/clipping limits.

GlassSurface, ApertureShape and SmartBlurMode serialize stable names; parameter indices do not enter the wire format. Radius, distortion and wavelength scale for previews; Glass smoothness is dimensionless and excluded. All new variants retain v6 layout, shared history/filter masks, caching and cancellation. [Algorithms, validation and CPU limits](docs/OPTICAL_FILTERS_VALIDATION.md).

### Stylize and render catalog completion

`processing::stylize` handles projected extrusion, oil-like paint, displaced tiles, level contours and wind. Extrude walks source-cell rings far-to-near with constant auxiliary geometry memory, computes alpha-weighted cell means and composites antialiased convex side/front faces. This painter ordering is not a z-buffer. Pyramids have four shaded triangles; blocks have two visible side faces and a front. Oil Paint blends bounded edge-tangent/range-weighted samples and least-variance paint, then adds generated directional bristle lighting. Tiles copies seeded displaced patches over an explicit backing without double-compositing their alpha. Trace Contour detects channel-level crossings; Wind samples premultiplied rows, limiting work to their conservatively expanded occupied spans.

`processing::cinematic` gathers a fixed stratified aperture for Lens Blur, using optional alpha/luminance circle-of-confusion data. Neighbor-depth support checks prevent focused silhouettes from leaking into gaps. Alpha-as-depth is itself interpolated with coverage weights, rather than confusing a silhouette's interpolated coverage with a different depth. Highlights and seeded noise are separate parameters. Lens Flare generates bounded core/halo/rays/rings/ghosts and screens them over original coverage. Lighting Effects combines a luminance-derived normal field with one Point, Spot or Directional light, diffuse illumination, a half-vector gloss term and material reflection tint.

Extrude, Lens Flare and Lighting Effects share normalized source-local origins and existing mouse transactions. Directional light placement, irrelevant material colors and other inactive settings are disabled contextually. Native v6 stores named option enums and all numeric/color/boolean parameters without changing its container. Full-layer CPU, reduced-preview, main-job cancellation and cache limits remain. [Algorithms and evidence](docs/STYLIZE_RENDER_VALIDATION.md).

### Data-driven filters and embedded processing assets

`processing::editable` owns strict bounded 3D Cube parsing, raster-map preflight, trilinear RGB lookup, RG inverse displacement, five-knot shear interpolation and signed 5×5 convolution. LUT samples are shared Arc vectors, maps are existing COW rasters. Asset validation participates in document validation. Lookup uses the point path; displacement/shear/convolution use spatial preparation and premultiplied sampling. Normalized curve/origin values remain resolution-independent; pixel displacement scales for previews. Convolution optionally filters alpha; its default color-only path preserves coverage and excludes hidden RGB.

Native v6 FilterRecord descriptors optionally reference LUT/map assets. The writer removes large inline data from its cloned filter descriptors, stores map tiles through existing pointer deduplication and LUT floats as little-endian binary triples, and deduplicates shared LUT storage. The reader validates variant/reference/size consistency before decompression, restores shared tables/map tiles and validates finite table/domain values before use. Legacy descriptors default to no extra assets; unknown variants are rejected by older readers. History's unique allocation accounting and prepared-layer cache budgeting include embedded filter data.

Processing drafts own their import receiver; workers load bounded assets outside the UI thread and request repaint on completion. Tab/layer/draft changes drop the receiver, preventing stale results from entering a new editor. Apply is blocked while import is pending. Errors retain the previous asset; first-frame/page notices reach the status bar. Curve and kernel popups mutate the same transactional draft and ticket as parameter sliders, so previews remain live and one Apply is one undoable document change. [Wire format](docs/FILE_FORMATS.md) and [validation](docs/EDITABLE_FILTERS_VALIDATION.md).


### Spatially varying blur and procedural/video extensions

`processing::gallery` evaluates bounded field-pin interpolation and rotated iris/band focus masks through six Gaussian scales, retaining at most two evaluated scales and one output. It also precomputes quadratic path stations/shutter trajectories and normalized elliptical spin trajectories; all spatial taps and transition mixes are premultiplied. Fixed pin/handle arrays are validated even when inactive. Preview scaling visits every pin radius; normalized coordinates and angles stay unchanged.

`processing::generators` owns original deterministic Flame/Picture Frame/Tree algorithms. Analytic coverage and seeded texture create RGBA content without source mutation. The tree emits bounded recursive branch capsules and leaf clusters before one final layer-opacity mix. `processing::video` reconstructs retained interlace fields and implements point-local YIQ gamut limiting and RGB/HSB/HSL channel conversion. Registry metadata supplies all parameter domains; new enum variants retain native v6 framing and ordinary mask/history/cache behavior. Origins and geometry guides use the existing transactional placement path. [Algorithms, limits and validation](docs/GALLERY_VIDEO_RENDER_VALIDATION.md); [external coverage boundary](docs/PHOTOSHOP_FILTER_AUDIT.md).


## Layer panel and style extension

Document backing RGBA and visibility are independent of editable raster layers. The CPU compositor starts with the effective backing. Selected IDs and a range anchor belong to each session. The active layer alone expands beneath its thumbnail; raster rows scroll while Background stays docked. Merge builds a validated replacement off-thread and publishes one history snapshot. New typed style variants reuse the registry, transactional live editor, prepared-layer bounds and worker cache. See [layer architecture and validation](docs/LAYER_EFFECTS_PANEL_VALIDATION.md).

## Extended styles and native v8

`Effect::Configured` decorates legacy typed effects without reinterpreting them. Common options and bounded material/curve vectors use the registry and existing transactional toolbar. Imported patterns share the binary tile asset store and history allocation accounting. Shared document lighting and resolved material phase participate in prepared-layer cache identity. Exterior style blending receives the real backdrop and resolves effective group RGBA before layer opacity/mode. See [settings validation](docs/LAYER_STYLE_SETTINGS_VALIDATION.md).

## Layer movement frames

Prepared appearance excludes placement and UI metadata; document-anchored material phase remains part of the key. Cached alpha support bounds feed a worker-owned, bounded `MovementCache`, which recomposes old/new visible regions with the reference CPU blend pipeline and retains complete COW pixels. `app/rendering.rs` uploads region images only against their exact base generation/epoch, with complete-frame fallback. Move generations progress without cancelling every pointer sample; edit/tab/end/cancel epochs still fence jobs. Reference-dependent changes and cache limits use full frames. See [movement measurements and validation](docs/LAYER_MOVEMENT_PERFORMANCE.md).

## GPU document presentation

`src/gpu` uses eframe's existing wgpu device. A cancellable worker packages core `Appearance` planes after publishing the CPU fallback. Persistent texture arrays hold evaluated layer appearances; a compute pass per visible layer blends into alternating straight-alpha RGBA8 targets, followed by a premultiplied display conversion for egui. Non-Normal exterior effects retain separate planes and blend against the actual backdrop before group opacity. All 19 layer blend modes match the CPU reference within the documented channel tolerance.

Ordinary linked-layer moves update placement uniforms, without filter evaluation, image uploads or CPU render requests while the pointer is held. Edit/tab/epoch cancellation runs before this fast path; stale scene-only results cannot replace a different active tab. Appearance input keys include referenced layers and resolved document-anchored material phases. Budgets bound GPU resources, retained source keys and CPU staging independently. CPU region rendering handles unsupported or position-dependent cases. The core model, history, export and native v8 format remain device-independent. [Detailed contract and evidence](docs/GPU_COMPOSITOR_VALIDATION.md).

## Unified Move and floating selection presentation

`Tool::Move` lives in Tools. A present selection routes to the existing `SelectionGesture` transaction; otherwise it starts `LayerDrag`. A selection can be picked up anywhere inside the canvas. Source snapshots remain immutable, original contour Arcs translate with the requested delta and viewport-edge coverage samples the original mask with inverse translation. Both ordinary selection tools' interior drags and Move share the GPU path.

Eligible raster selections add one immutable R8 coverage texture to the scene. The existing blend shader reconstructs cut/paste source pixels using the CPU reference's old/new coverage partition, including feather overlap and quantized alpha. Only displacement uniforms change during the drag. Scene staging can warm from the gesture's base even after a CPU preview has arrived. Release/escape/history continue through the device-independent transform transaction. Live masks/filters/effects and reference-dependent appearances retain CPU evaluation rather than changing source semantics. [Selection contract and tests](docs/UNIFIED_MOVE_VALIDATION.md).

## Dependency reduction

Neural Filters and AVIF support were intentionally removed on 2026-10-20 to reduce dependencies, binary size and build requirements. All twelve operations formerly under Neural Filters are removed, including conventional approximations. PNG, JPEG, BMP, GIF, TIFF, WebP, `.paint` and `.palette` remain supported. Older projects containing a retired filter fail with an explicit diagnostic, including disabled filters; open them in an older build and remove or rasterize those filters before saving a compatible copy. No filter is silently discarded.

The workspace contains the application and palette-core. Conventional layer styles retain a small shared sRGB/Lab conversion module; no model loader, model payload, inference executor or AV1 codec remains. Release builds use Thin LTO and one code-generation unit. GPU presentation, CPU reference rendering, masks, live conventional filters and native asset storage remain shared across platforms.


## Layer-panel drag reordering

`app/layers/reordering.rs` shares presentation for raster cards and live filter/
effect rows. Stable list scopes include the session, layer and stack type; item
IDs remain stable across order changes. Original-order slots and measured row
heights determine insertion independently of animated hit targets, preventing
hover oscillation. A paint-only tooltip layer draws a translucent name/thumbnail
preview, while the destination gap has an accent insertion line and neighbors
animate for 160 ms. Expanded properties move with their active layer.

Only an accepted release changes the document. Filters/effects use a remove/insert
transaction, preserving intermediate order, IDs, masks, settings and source
pixels; layer insertion converts displayed top-to-bottom positions to model
bottom-to-top indices. Existing invalidation, processing cancellation and history
handle the final render and single undo entry. Escape, outside/no-op drops and
foreign scopes do not mutate the document. Long layer lists scroll at the owning
viewport edge; their Background control remains outside the scroll area.
No dependency, renderer algorithm or native-project format change is needed.
