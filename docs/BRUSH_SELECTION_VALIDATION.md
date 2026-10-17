# Brush selection — 2026-10-17

## Implemented behavior

Choose **Selection > Selection brush**. The existing canvas strip keeps its 46-point height and dynamic width: tolerance, feather with exact fields, four combine icons, inversion and options. The three existing left sliders become size, strength and hardness; drawing-brush settings remain independent. Brackets adjust diameter. Options select color matching, current/all-visible sampling and a cyan coverage overlay. Disabling matching gives ordinary soft selection painting and disables tolerance/sampling controls.

A brush gesture shows live coverage, original ants and 40%-opacity feather ants without editing image pixels or committing selection coverage. Release waits for the exact final preview and creates one undo command. Escape discards the draft. Replace switches to Add after commitment; entering this tool with an existing Replace selection chooses Add for refinement. Alt temporarily subtracts, Shift adds and Shift+Alt intersects; icons support explicit modes. Subtract/intersect without an existing selection do nothing. Read-only sampling works on locked layers. Selection gestures/settings belong to their tab, and committed coverage/original contours survive native v4 saving.

Adobe's [Selection Brush guide](https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/create-quick-selections-with-selection-brush-tool.html) documents brush size/hardness/opacity, overlay and add/subtract behavior. Its [Quick Selection guide](https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/paint-a-selection-with-quick-selection-tool.html) documents edge-following painting and all-layer sampling. Palette combines these interaction patterns with the requested tolerance and feather controls. This is an independent implementation; it does not claim Photoshop's proprietary segmentation or combined brush/lasso gesture parity.

## Algorithm and limits

Continuous capsules use the shared round brush rasterizer. Maximum coverage during a gesture makes strength independent of sample density/retracing; a separate additive stroke can increase coverage. Matching compares alpha-weighted Oklab plus alpha against the initial color, ignoring hidden RGB in fully transparent pixels. A four-connected scanline fill within the swept footprint prevents crossing a contrasting barrier into a disconnected matching island. Higher tolerance admits more colors; 100 bypasses color rejection. A new stroke resamples its starting color. This follows edges within the footprint rather than automatically segmenting a whole object outside it. Plain coverage mode bypasses matching/connectivity.

A bounded worker incrementally processes appended samples, feathers incoming coverage before combination, and builds contours/overlay pixels. Requests coalesce without losing path samples; generation and gesture tokens reject cancelled/stale results. Worker scratch state is released after gestures. Explicit tool/tab/save/undo actions finish an outstanding target synchronously if its exact final worker frame is unavailable. Running computation is not interrupted. Paths are capped at 16384 samples, with an explicit error/cancel at the limit. Large dense masks still require full canvas byte planes, and overlay upload/presentation can lag. Pressure, oval tips, automatic edge enhancement and selectable spacing remain future work.

Sparse feathering now crops coverage with conservative padding beyond the kernel support, restores full-size output, and preserves canvas-edge clamping. Directed contour scans stay within nonzero bounds. Tests compare cropped feathering byte-for-byte with the full-canvas reference at fractional, large and boundary radii. Dense/disabled masks use the reference path. An empty-mask bounds underflow found by these tests was fixed; zero-strength strokes complete without panicking a worker.

## Validation

Windows local checks: formatting, strict workspace/all-target Clippy, 156 workspace tests, executable build and native wgpu launch pass. The two desktop regression tests cover 34 reviewed 1386 × 963 states; the two new brush scenes and updated Selection/Wand menus were visually reviewed. Compact 780 × 520 brush preview/options captures were reviewed, and toolbar geometry input tests include 100/150/200% egui scale with layer properties open/closed. These are not physical-tablet, multi-monitor or other-OS execution claims.

New tests exercise real pointer-held strokes at fractional zoom/high DPI, live extent growth and overlay, one-command undo/redo, cancellation/stale results, pending-target completion on tab change, all combine modes/Alt, exact numeric options, settings independence, empty coverage, source preservation, connected barriers, monotonic tolerance, hidden transparent RGB, local/merged sampling and native saving. Existing editing, masks, codecs, Paint and recovery tests remain passing.

`cargo run -p palette-core --example selection_benchmark --locked` measures an 80-sample, 40-pixel edge-aware stroke on a uniform opaque canvas, plus 4-pixel feather and both contour sets. One optimized-dev run:

| Canvas | Raw stroke coverage | Including feather/contours |
|---|---:|---:|
| 1280 × 720 | 2.76 ms | 17.32 ms |
| 3840 × 2160 | 2.60 ms | 37.97 ms |

The earlier full-canvas path took about 168 ms and 1.49 s respectively. These are CPU measurements, excluding input, set-combination, source composition, overlay conversion, GPU upload, presentation and peak memory. They do not establish dense 8K interaction latency.

## Phase 3 review

Local remaining ordinary-import notices are implemented and tested for GIF, animated WebP, APNG default images and TIFF first pages, including renamed files and image-as-layer. No dependencies or native format version changed. The supplied Paint specimen's SHA-256 remains B9B8547C9DEFB4B86182CF1BF2FC96F16D6E3ABE2E220617FB5F3EC73D3A1BC9.

Native Paint editing/save-back remains an external acceptance gate. The installed Computer Use skill was initialized; app discovery returned a missing native pipe (os error 2), so no native input could be performed. The user's earlier open/appearance validation is retained. Windows 10/macOS/Linux execution, broader codecs/Paint variants, full profile conversion and packaged release notices remain explicitly tracked in ROADMAP.md/KNOWN_LIMITATIONS.md. Nothing was committed, branched or pushed.

At a one-pixel selection-brush diameter, the tip snaps to pixel centers and uses a hard antialiased footprint; this prevents empty soft dabs and disconnected diagonal strokes at that scale.
