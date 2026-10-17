# Selection outlines and canvas navigation — 2026-10-16

Implemented the requested selection display and scrollbar follow-up. The live-feather follow-up below supersedes the earlier preview, opacity and slider limitations. The original Phase 4 slices remain historical evidence in PHASE4_VALIDATION.md.

## Behavior

- Wand tolerance is monotonic: dragging left reaches exact matching; dragging right reaches all colors. New UI tests exercise the actual slider, then click the same grayscale-ramp seed and inspect resulting coverage. Both contiguous/noncontiguous and antialiased/non-antialiased core paths retain or expand coverage as tolerance increases. No inverted distance comparison was reproduced, so the correct comparison was retained. Slider and sampling tooltips clarify the range and active-layer transparency behavior.
- Inversion retains interior holes and displays marching ants around selected runs on the visible canvas perimeter. When zoom clips the actual perimeter, a border continuation outlines the visible selected region within the viewport. It samples physical display pixels, respects zoom/pan/DPI, preserves animation phase and does not alter selection data.
- Feathering retains original coverage and opaque original ants. A separate set of black/white ants at 25% opacity traces nonzero feather coverage. Radius remains Gaussian sigma; the outer extent can be farther than that numeric radius. The existing-selection dialog adjusts from the original, so repeated settings do not compound blur and zero restores the original. Source pixels remain unchanged. Toolbars continue to set feathering for newly created regions.
- Original and evaluated coverage both participate in combination, inversion, selected/whole-image transforms, resize, undo/redo and native saving. Deselect/plain replacement removes old provenance. The native writer is now v4 and reads v1–v3; old builds reject v4 rather than silently losing the original contour. Legacy feathered files preserve their existing coverage but cannot reconstruct a missing original.
- Canvas rails appear independently on overflowing axes, including when one rail causes the other axis to overflow. Horizontal rails sit above the status bar; vertical rails stay left of the layers tray and its expanded properties. Thumb dragging preserves the grab offset, rail clicks seek, and arrows/page/Home/End work while focused. Wheel/pan share bounded per-tab state; nonoverflowing axes center and Fit removes the rails. Rails cannot start image editing gestures.

## Validation

Windows local validation: `cargo fmt --all --check`, strict workspace/all-target Clippy, workspace tests (140 passed, two desktop tests separately), workspace build and native wgpu launch. Screenshot regression covers 32 reviewed 1386 × 963 states. New states: feather outline, inverted selection at 175% and both canvas rails with layer properties. Reviewed compact 780 × 520 versions and Wand toolbar spacing as well. Existing affected baselines were visually reviewed and updated for the reduced canvas viewport.

Added mathematical/round-trip tests cover feather origin preservation, noncumulative changes, zero restoration, inversion/history, actual feather extent, original-contour transforms, native v4 round trip, v3 migration, invalid/orphan origins and failed-save destination protection. Existing v1/v2, raster/AVIF/Paint, painting and geometry tests pass. UI tests cover real slider direction, opaque/25%-opacity ant colors, all four clipped canvas borders, both rail drags without source changes, per-tab pan, Fit and rail/tray separation. Minimum-window toolbar geometry is checked with properties both open/closed at 100/150/200% egui scale. These are not physical multi-monitor or macOS/Linux validation claims.

## Files and remaining limits

Changed core document/selection/contour/geometry/native I/O modules; app session, canvas overlays/navigation, toolbar, edit routing and captures; core/input/screenshot tests and Windows baselines; architecture, file compatibility, checklist, README and limitations documentation. No new dependencies or unsafe Rust. All edits are uncommitted.

No live feather-slider preview of the current selection, preserved vector lasso path, edge-aware selection brush or cancellation of running workers was added. Large contour-cache construction still runs on the UI thread. Layer-mask feathering remains a cumulative coverage edit. These do not block the requested selection display and navigation behavior.

## Subpixel precision follow-up

General UI refinement is deferred until feature completion or an explicit user request. This follow-up changes only the requested parameter behavior and feather-outline opacity.

Feather values below 5 px display exactly one decimal place, including 0.0 and 2.0. Numeric input and sliders use 0.1 steps. The same logarithmic slider configuration is shared by toolbar tolerance/feather, the Wand options submenu and selection/mask feather dialogs; zero and maximum remain reachable. At the physical midpoint the value is below 5, giving small values much more travel. Outer feather ants now use alpha 64/255 (approximately 25%); original selection ants remain opaque.

The former three-box approximation rounded tiny radii to whole-pixel kernels. Below 5 px, separable Gaussian weights now integrate both source and destination pixel footprints, so even 0.1 and 0.2 px yield distinct coverage. Coefficients use second differences of x*Phi(x/sigma) + sigma*phi(x/sigma), a convergent positive erf series, a four-sigma support bound plus one pixel, normalization and clamped image edges. Processing retains the existing two temporary byte planes and immutable original coverage. Larger radii retain the running-sum approximation; 8-bit rounding and the algorithm transition at 5 px remain limits, rather than claims of unlimited subpixel precision.

Tests compare centered impulse profiles at eight fractional radii against independently evaluated Python math.erf references. They also verify symmetry, constant/disabled/tiny masks, source preservation, numeric precision, slider midpoint/endpoints and 25% ant colors. Formatting, strict Clippy, 140 workspace tests, build and the 32-state native Windows screenshot suite pass. Seven targeted scene baselines were reviewed for these requested display changes. No dependencies or native format version changed in this follow-up; edits remain uncommitted.


## Live feather and milder slider follow-up

Outer feather ants now use alpha 102/255 (40% opacity), retaining opaque original ants. Toolbar feather changes preview the current selection while the pointer remains held; release commits one undo step. The Feather selection dialog previews live, Apply commits, and Cancel/Escape discards the pending change. Preview processing and extent-contour construction run on a bounded worker; rapid changes coalesce, and stale generation/gesture results are discarded. Changing tools/tabs or saving can synchronously finish a pending toolbar preview if its final worker frame is not ready. Running jobs finish rather than being interrupted. Layer-mask feather remains Apply-only.

Tolerance and feather share a milder shifted-log curve with a 10-unit offset: their slider midpoints are about 23 and 27 units respectively, rather than below 5. Endpoints, actual-unit numeric fields and 0.1 steps remain available, including one decimal below 5 px.

New input tests hold and drag the real toolbar/dialog controls, check live extent growth without document mutation, commit/undo/redo, modal cancellation, coalescing, cancelled-result fencing and tab-switch completion. Updated tests check 40% ant colors, milder midpoints and actual dialog units. No dependency or file-format changes are required. General UI refinement remains deferred.


Validation for this follow-up: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (143 tests passed; two desktop tests run separately) and `cargo build --workspace --locked` pass. A running user instance initially locked the normal executable; an isolated diagnostic build from identical source successfully launched and passed all 32 Windows screenshot comparisons without disturbing that instance. After the lock cleared, the normal executable was rebuilt successfully and `cargo test --test screenshot --locked -- --ignored --test-threads=1` passed both desktop tests covering all 32 scenes. Feather outline/dialog and Wand-toolbar captures were visually reviewed and their three baselines updated. Temporary diagnostic source copies were removed. macOS/Linux were not available for validation. Changes remain uncommitted.
