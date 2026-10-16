# Architecture

## Scope and decisions

Phase 0 found an empty Git repository, two screenshot attachments and no `Untitled.paint`. The implementation uses a two-package Cargo workspace: `palette-core` owns editing mathematics and serialized documents; the root `palette` binary owns eframe/egui interaction and wgpu presentation. Core never depends on egui, winit or the operating system.

Current stable releases were checked with Cargo and primary documentation: [eframe 0.37](https://docs.rs/eframe/0.37.0/eframe/), [image 0.25.10](https://docs.rs/image/0.25.10/image/) and [rfd 0.17.2](https://docs.rs/rfd/0.17.2/rfd/). Cargo.lock pins the complete resolution. The toolchain here is Rust 1.99.1; the declared minimum is 1.95, required by this eframe version, and has not been independently tested.

egui permits one shared native interface across the three target operating systems. Original vector geometry keeps icon shapes consistent at different DPI settings. Native title bars retain OS window management; title-bar appearance is intentionally OS-owned. wgpu renders the UI and composite texture. Image compositing remains on the CPU initially so the correctness oracle exists before shader optimization.

## Model and ownership

`Document` contains canvas bounds, bottom-to-top raster layers, an active stable ID and an optional grayscale selection. Each layer independently owns source RGBA, visibility, lock, opacity, blend mode, signed translation, optional coverage mask, ordered filter instances and ordered style instances. IDs also identify filters and effects. Duplicating a layer allocates new nested IDs.

Raster uses 128 × 128 RGBA8 tiles behind `Arc<Vec<u8>>`. Cloning a document shares pixels. The first modification to a shared tile copies just that tile. Partial boundary tiles have defined padding; ordinary export emits only canvas pixels. Coverage masks are grayscale byte planes behind Arc, ready for selection/mask editing in Phase 4. A mask does not alter source pixels.

The per-layer transform field currently stores translation only. Whole-image rotate/flip/resize operations transform source rasters and related coverage planes. Per-layer affine transforms, ICC profiles, higher bit depths and effect variants remain future model work. Version 1 supports an actual Color Overlay style in the core but has no style-editing UI. Blur and shadow operations require neighborhood/padded image stages; they must not be shoehorned into the current point-filter evaluator.

## Alpha and pipeline

Stored RGB is straight (unassociated) sRGB; alpha is linear coverage. Per-layer order is source → enabled filters in order (individual coverage masks mix original and filtered RGB) → enabled color-overlay styles → layer mask → translation → layer opacity and blend mode → source-over document composite. Both transparent and partly transparent input pixels use W3C-style premultiplied arithmetic internally, followed by safe unpremultiplication. Transparent RGB cannot contaminate the output. Current blend modes operate in sRGB, not linear-light RGB. Profile conversion is not implemented.

The GUI converts the straight-alpha reference composite through `ColorImage::from_rgba_unmultiplied`, so egui receives the representation it expects. Checkerboard is presentation-only and never enters saved pixels. JPEG export explicitly composites on white.

## Commands and document sessions

`History::begin` records a named pre-command COW snapshot and assigns a unique revision. A new branch clears redo. Undo/redo swap complete model states, preserving layers, filters, masks, properties and source pixels together. A stroke creates one command, rather than one per sample. `finish` trims history after a gesture/command. The bound is 256 commands and 128 MiB of uniquely retained tile/coverage buffers beyond the current live document; repeated references and buffers still reachable from the live document count once or not at all, respectively. Metadata, rendering and live buffers are separate from this budget. Huge destructive operations can still exceed the budget and lose undo; small edits on an 8K canvas retain their undo entry.

Each UI `Session` owns a document, independent history, saved revision, native project path, texture, zoom and pan. Dirty state compares saved and current revisions, so undoing to a saved revision becomes clean. Raster imports have no native project destination; Save prompts for `.palette`. Export never marks the project clean. A save worker captures the original tab ID and revision; a later result cannot accidentally mark new edits or another tab saved. A pending close proceeds only after that tab's successful save and clean-revision check.

## Rendering and jobs

The active document is submitted to a single CPU rendering worker through a capacity-one queue. The job shares COW source data. Results carry session ID and generation; obsolete or closed-tab results are ignored. GPU-ready image conversion runs on the worker. Images exceeding the reported GPU texture dimension receive a reduced display texture while retaining full editing/composite data. Main-thread texture uploads occur only on accepted results. Unchanged documents reuse the texture. Small layer thumbnails are cached per session/layer/generation. File operations run off the UI thread, with at most four outstanding operations, and return through a separate result channel.

Current rendering recomputes a complete composite after changes, and obsolete jobs finish rather than being interrupted. Source storage is tiled, but dirty-region rendering, filter-stage caches, cancellable preview jobs, partial texture uploads and GPU compositing are Phase 5/6 work. No GPU-accelerated compositing claim is made.

## Interaction coordinates

Canvas origin is the viewport center minus half the scaled canvas plus pan. Image coordinates are `(pointer - origin) / zoom`. egui supplies logical display points after OS DPI conversion; applying DPI again would be wrong. The display origin is aligned to physical pixels. Integer zoom uses nearest texture sampling. Fractional zoom mapping is tested; filtered texture sampling and advanced trackpad behavior need later polish.

Brush segments rasterize the union of a circular footprint along a line, avoiding gaps during fast motion. Hardness defines the core and linear edge falloff. `PaintStroke` retains a COW pre-stroke raster and sparse float coverage tiles; each pixel is evaluated from its original value at maximum stroke coverage, so event density and retracing within one stroke do not accumulate opacity. Outlined shapes share this accumulator. A new gesture can build opacity. Painting and erasing multiply by canvas-space selection coverage after accounting for layer offsets. Pressure/spacing controls remain future work.

## Core editing workflow

`editing` implements selection coverage/bounds/contours, active-layer copy, clear, paste, move and iterative scanline fill. Clipboard output bakes the active layer's evaluated appearance into flat RGBA; source filters/masks remain untouched. Paste creates a layer and expands canvas bounds if needed. Move clears selected source coverage and composites it at the new canvas location, preserving layer processing properties. The active source buffer can expand to the union of its old extent and the canvas; local masks are reanchored alongside it. Existing hidden pixels survive canvas cropping and bounds resizing.

`transform` owns whole-document crop, exact orthogonal pixel permutations and nearest/premultiplied-bilinear resizing. Layer positions, selection and local layer/filter masks follow the geometry. Layer filters/effects are retained and reevaluated after the transform. These whole-document operations include locked layers; locks protect local pixel tools. Size/offset errors leave the original document unchanged, and output allocations are preflighted against per-raster limits and an aggregate 512 MiB output-plane budget.

The app's editing controller runs expensive edits on a shared snapshot in a background worker. The result identifies its originating session/generation and commits one command only if still current. No-op jobs do not dirty the document; stale results cannot overwrite newer edits. A retained arboard clipboard object handles native image copy/cut/paste and X11 ownership; eframe image-paste events are also routed when text input has no focus. Clipboard write failure prevents cut deletion. Each session owns its in-flight selection gesture and a pointer-identity cache of selection bounds/contours. Its cached Arc prevents pointer-address reuse from returning a stale contour.

## File boundaries

`.palette` has its own magic, explicit format version, zlib-compressed JSON and validated model invariants. The reader limits compressed and decompressed input before rendering. Encoding completes in a same-directory temporary file, syncs it, then atomically replaces the target. Unsupported versions and enum variants fail explicitly, preserving the existing file. Version migration belongs at this boundary.

Microsoft `.paint` needs a dedicated HEIF item-graph adapter; primary-image raster decoding is insufficient. [Research and required experiments](docs/PAINT_RESEARCH.md) identify libheif's uncompressed and overlay capabilities, binding gaps and validation gates. Nothing writes a `.paint` extension today.

## Next boundaries

The next Phase 2 slices are text, additional shapes, whole-layer positioning, brush variants and gesture-coalesced properties. Shape geometry should move fully out of the canvas controller into reusable core tools. Advanced selections and mask-painting UI follow later. Phase 3 should migrate native assets from JSON byte arrays to streamed lossless assets, add explicit migrations, exercise AVIF on all targets and implement `.paint` only against real fixtures. Filter descriptors, common parameter controls, cancellable staged previews and spatial effects follow in Phase 5.
