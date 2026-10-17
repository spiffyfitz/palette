# Master roadmap review

This review follows the original phases. Functional source code and passing Windows tests count as local implementation; a configured CI job, disabled menu entry or model field alone does not establish a completed workflow. The selection-handle slice is sufficient for now, per the user's direction.

| Phase | Current evidence | Remaining gate / next work |
|---|---|---|
| 0 — inspection/design | Workspace, references/video, UI-independent model and libheif research documented | Supplied specimen inspected, hashed and decoded; broader Paint variants remain unverified |
| 1 — runnable interface | Native Paint-style Windows app, tabs, layers, zoom, sliders, ordinary raster files and reviewed captures | Exact WinUI nuance, accessibility, Windows 10/macOS/Linux execution |
| 2 — core painting | Live strokes, eraser, fill, shapes/polygon/curve, text, clipboard, selections, transforms and history work | Brush variants/tablet/mask painting, text metric parity, property gesture coalescing and persistent vectors remain explicit deferrals |
| 3 — file compatibility | Real still AVIF codec/options; `.palette` v4 streamed shared assets, v1/v2/v3 migration, backing color and opaque metadata; genuine two-layer `.paint` subset import/export; import layer, recents and recovery | User confirms Paint opening/appearance; edit/save-back, broader variants/platforms and metadata/color/conformance remain |
| 4 — advanced layers/selections | 19 blend modes; Wand/freeform/edge-aware brush, live feather previews, canvas toolbars, coverage combination and basic layer-mask properties | Merges, polygon selection, morphology and mask painting/view/apply/link workflows |
| 5 — filters/styles | 12 live filters, six styles, typed registry/common parameters, on-canvas previews, three-icon layer controls, filter masks, evaluated-layer cache and native preservation | Remaining catalog/styles, Filter Gallery, per-stage/tile caching, main-job cancellation and GPU processing |
| 6 — platform polish | CI configuration and Windows native captures | Actual platform matrix, performance benchmarks on dense 4K/8K, accessibility, packaging and signing |

## Phase 3 acceptance status

Implemented and locally validated: ordinary raster I/O; real AVIF opening/encoding with alpha and quality/speed options; lossless native preservation of all currently represented editable data and original ICC/Exif bytes; explicit v1/v2/v3-to-v4 migration; atomic saves; detected animation/page import notices; image-as-layer import; persistent recent files; periodic recovery with startup restore/discard. See [Phase 3 validation](docs/PHASE3_VALIDATION.md) and [file compatibility](docs/FILE_FORMATS.md).

Phase 3's documented subset is implemented. The user has now confirmed that the generated `.paint` opens and looks correct in Microsoft Paint. That is recorded as manual opening/appearance validation and accepted for progression to Phase 4; it is not an assertion that native layer edits and a Paint-save-back cycle were tested. Those broader checks and the platform matrix remain tracked. The adapter still labels its limited export experimental. No commits, branch changes or pushes were performed.

AVIF Workflow I is tested locally on Windows with actual rav1e encoding and independent rav1d decoding. The four-target CI configuration includes codec tests and NASM setup, but no successful remote platform run is claimed. Workflow H preserves currently implemented model fields; future filters/styles require their own feature work.

## Next work

At the user's request, progress has moved to Phase 5. Its first complete vertical slice is implemented and validated: [live processing validation](docs/PHASE5_VALIDATION.md). This advances the roadmap without declaring all earlier deferrals complete.

1. Continue the expanded catalog with Levels/Curves, Motion Blur, noise generation, High Pass and Mosaic. Brightness/Contrast, Exposure, Hue/Saturation, Vibrance, Black & White, Threshold, Posterize, Box Blur and Median are implemented and tested.
2. Reduce full-resolution styled-layer costs through alpha-only padded assets, per-stage/tile caching and cancellable main jobs before claiming smooth dense 4K/8K editing.
3. Continue styles with Bevel/Emboss, Satin and Gradient/Pattern Overlay; build Filter Gallery around implemented entries. Inner Shadow/Glow and inside/outside/centered Stroke are implemented.
4. Retain the open Phase 4 work: polygon selection, source-preserving mask painting/view/link/apply, selection morphology, selected/visible merges and confirmed flattening with backdrop-dependent blend tests.
5. Broaden `.paint` evidence with native layer editing/save-back and execute the platform codec/build matrix as environments become available.

The deferred Phase 2 polish remains in FEATURES.md. It has not been silently declared complete or used to delay the requested move to file compatibility.

Selection display/navigation follow-up: original and feather-extent ants, clipped inverted perimeters, per-axis canvas rails, and native v4 preservation are implemented. Evidence and remaining limits: [selection/navigation validation](docs/SELECTION_NAVIGATION_VALIDATION.md).


Latest slice: [brush selection validation](docs/BRUSH_SELECTION_VALIDATION.md). Phase 3 local code gaps identified in this review (animated WebP/APNG/TIFF import notices) are resolved and tested. Native Paint layer editing/save-back was attempted again through Computer Use, but the native pipe is unavailable (os error 2). The user-confirmed opening/appearance result remains valid; save-back and unavailable platform execution remain external acceptance gates, not completed checks. Working-profile conversion and broader metadata/codec conformance are tracked future capabilities rather than silently represented as implemented.

Latest UI follow-up: [three layer controls and on-canvas processing](docs/LAYER_PROCESSING_UI_VALIDATION.md). This replaces the floating stack window/modal parameter preview with Paint-height contextual bars; it does not expand the supported filter catalog.

Latest Phase 5 batch: [expanded catalog validation](docs/PROCESSING_CATALOG_VALIDATION.md), including algorithm ranges, native preservation, independent references and measured Box/Median costs. Phase 5 remains in progress.
