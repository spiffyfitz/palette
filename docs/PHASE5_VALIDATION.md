# Phase 5 — first live-processing slice

Validated locally on Windows, 2026-10-17, Rust 1.99.1. The subsequent [layer-control UI update](LAYER_PROCESSING_UI_VALIDATION.md) replaces the initial dialog/window presentation described in this milestone with contextual canvas toolbars. This completes the first working vertical slice of Phase 5, not the entire filter/style catalog. Earlier Phase 4 deferrals remain listed in FEATURES.md and ROADMAP.md.

## User workflow

Filter > Gaussian Blur (also under Blur) opens a centered parameter dialog with a live composite preview, radius slider/numeric input, Preview toggle and Apply/Cancel. Brightness uses the same transactional controls. The Effects ribbon control, Filter > Edit live filter stack or layer properties > Filters / effects opens the active layer's processing stack. Add offers implemented filters/styles only.

Filters and effects can be edited, toggled, reordered within their respective stacks, duplicated with fresh stable IDs or removed. Filter Mask > From selection converts current canvas coverage to a local filter mask without replacing the layer mask; enable/disable/remove controls work. All changes are undoable. Drop Shadow exposes color, opacity, light angle, distance, spread and size; Outer Glow exposes color, opacity, spread and size. Color Overlay has editable color/opacity. Editing source pixels automatically regenerates the live appearance and spatial thumbnail. Source pixels and processing parameters remain separate.

Apply commits full-resolution parameters in one history command. Cancel/Escape discards pending settings. A changed tab/layer/generation or OS close cancels an unapplied draft. Preview processing has a separate bounded worker and cancellation ticket; obsolete results cannot mutate or replace another document's preview. The original preview was a reduced composite up to 512 px; the contextual UI now uses up to 2048 px, with spatial parameters scaled appropriately. It is an approximation of full-resolution processing.

Native `.palette` preserves the source, IDs, order, enabled states, masks and all new parameters. New enum variants fit the existing v4 descriptor layout; old readers reject unknown variants. Compatible `.paint` export uses a separately rendered/baked copy and still requires complete native preservation first. Flat exports use the same rendered composite. No new Microsoft Paint edit/save-back evidence is claimed.

## Correctness and verification

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --locked`: 166 tests, comprising 77 app tests and 89 core tests; two desktop test functions are ignored in the ordinary run.
- `cargo build --workspace --locked`.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two desktop test functions cover 39 reviewed states at 1386 × 963; both pass. Five new scenes cover Gaussian Blur, shadow, glow, stack and Filter menu. Native wgpu captures launch the executable and exit automatically. The shadow and stack windows were also visually inspected at 780 × 520.

Six new core tests cover independent small-sigma Gaussian reference values, premultiplied transparent-edge handling, constant images, symmetry across tile boundaries, in-pass cancellation, shadow/glow extents and source updates, masks, layer opacity, filter ordering, native preservation, baked Paint pixel comparison, all 19 blend modes, region/full equivalence, cache invalidation and invalid-save protection. Four new app tests exercise actual parameter slider/Apply/Cancel input, native reopen, undo/redo, stack/mask mutations, cancelled/tab-stale previews and live painting with spatial processing. Existing raster codec, recovery, selection, history and Paint tests still pass.

## CPU costs and limits

Run `cargo run -p palette-core --example processing_benchmark --locked`. One local optimized-development run (`opt-level=2` for core, not a release benchmark):

| Layer size | Gaussian, sigma 8 px | Blur + shadow + glow, cold | Repeat composite | Cache hits / preparations |
|---|---:|---:|---:|---:|
| 1280 × 720 | 77 ms | 373 ms | 71 ms | 1 / 1 |
| 3840 × 2160 | 697 ms | 3.02 s | 3.10 s | 0 / 2 |

The workload is one full-sized transparent raster with an opaque rectangle over its central quarter, Gaussian 8 px and default shadow/glow. It includes CPU output construction, excludes disk I/O, UI events, GPU upload and presentation, and is a single-run observation, not a latency guarantee. Repeat pixels are checked against the cold output. A complete 4K prepared stack exceeds the conservative 128 MiB LRU budget and is deliberately not retained. This is an exposed performance limitation, not smooth 4K editing evidence.

Gaussian scratch uses 256 × 256 output tiles plus complete halos (about 6.5 MiB maximum for two fixed-point RGBA planes at sigma 64). Source/output/style rasters are still full-sized. The cache has at most eight entries within 128 MiB, counting retained sources and masks conservatively. Enabled padded style rasters have the normal 64-megapixel individual limit and a 512 MiB per-layer output estimate. These limits do not cap total process memory.

Main canvas jobs run off the UI thread and accept progressively newer stroke snapshots. Spatial documents use full background composition instead of incorrect synchronous partial filtering. Only parameter-preview processing supports in-flight cancellation; its resize stage and main full-resolution render work are fenced but not interrupted. There is no per-stage/tile-result cache, GPU compositor, general dirty-region neighborhood evaluation or dense 8K benchmark yet.

## Algorithm scope and next work

The independent Rust Gaussian uses a sampled kernel for sigma < 2 and a conventional three-box approximation otherwise. Inputs/outputs are straight RGBA8; processing intermediates are premultiplied fixed-point RGBA. Blur clamps original source boundaries and does not enlarge the source. Styles use transparent padding, square max-dilation for spread and bilinear fractional shadow offsets. Filters run in order, then Color Overlay, layer mask, shadow/glow underlays, translation and final layer opacity/blending. Underlays compose in effect-list order beneath the foreground. The working RGB space remains sRGB. These are conventional equivalent operations, not Photoshop-identical rendering.

Next: core adjustments and additional blur/noise operations through the registry; alpha-only padded style assets, per-stage/tile caches and cancellable main jobs; then inner styles, Stroke and an implemented-only Filter Gallery. Other styles/catalog categories, individual effect masks, effect-specific blend modes, profile-aware processing and additional platform execution remain pending. No dependency, native-library requirement or license changed.

References consulted for the independent implementation: [W3C compositing](https://www.w3.org/TR/compositing-1/), [SVG filter effects and Gaussian approximation](https://www.w3.org/TR/SVG11/filters.html), and [Adobe's public layer-effect option descriptions](https://helpx.adobe.com/uk/photoshop/desktop/create-manage-layers/apply-layer-effects/layer-style-effects-and-options-overview.html). No proprietary implementation was copied. All changes remain uncommitted; no branch or remote operation was performed.
