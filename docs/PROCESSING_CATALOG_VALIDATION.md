# Phase 5 — expanded filter and style catalog

Windows, 2026-10-17, Rust 1.99.1. Nine filters and three effects join the previous processing slice, for **12 implemented filters and six effects**. Phase 5 remains in progress. No dependency/native-library requirement changed; all implementations are independent Rust.

## New operations

| Category | Operation | Parameters / implementation |
|---|---|---|
| Adjustments | Brightness/Contrast | Brightness −100…100%; contrast −99…99%. Additive brightness and contrast gain around encoded mid-gray. The original Brightness operation remains readable/editable. |
| Adjustments | Exposure | −8…8 EV, 0.01 steps. Decode sRGB, apply a linear-light power-of-two gain, clip and re-encode. |
| Adjustments | Hue/Saturation | Hue −180…180°, saturation/lightness −100…100%. HSL hue wrap, saturation multiplication and lightness offset. |
| Adjustments | Vibrance | Amount −1…1. HSL saturation-dependent gain favors less-saturated colors; no proprietary skin-tone protection. |
| Adjustments | Black & White | Strength 0…100%. Rec.709-weighted encoded RGB mixed with original RGB; alpha is unchanged. |
| Adjustments | Threshold | Integer 0…255. Weighted encoded RGB at/above the threshold becomes white; below becomes black. |
| Adjustments | Posterize | Integer 2…32 levels per channel. Nearest evenly spaced level, preserving alpha. |
| Blur | Box Blur | Integer radius 0…32 px. Separable average over premultiplied 16-bit RGBA with clamped boundaries. |
| Noise | Median | Integer radius 0…5 px. Independent channel medians in premultiplied 16-bit RGBA, clamped boundaries. Zero radius bypasses either neighborhood filter exactly. |
| Layer styles | Inner Shadow | Color, opacity, light angle, distance, spread and size. Shifted/blurred complementary silhouette recolors existing masked foreground coverage. |
| Layer styles | Inner Glow | Color, opacity, spread and size. Complementary blurred/eroded silhouette recolors the interior boundary. |
| Layer styles | Stroke | Color, opacity, width 0…64 px and Outside/Inside/Center placement. Fractional square dilation/complement dilation provides subpixel width; joins are square/miter-like. Center splits the width equally between inside/outside. |

Existing Invert, Brightness, Gaussian Blur, Color Overlay, Drop Shadow and Outer Glow remain available. Inner effects preserve the filtered/masked foreground alpha; they cannot reveal hidden source pixels. Outer contributions extend the appearance without resizing or replacing source data. Final layer opacity/blending is applied once after styles. Color overlays retain their existing fixed pipeline position before the mask; other style contributions follow stack order within their foreground/background roles.

## Interface and preservation

The Filter menu builds implemented category entries directly from the same registry as the canvas toolbars. Adjustments, Blur and Noise are functional; empty categories stay disabled. Add lists scroll for compact windows. Parameters use common numeric/color/choice controls, appropriate numeric precision and validated integer fields. Stroke placement is a real typed enumeration, not a numerical stand-in.

Every new operation uses the existing live on-canvas draft, Preview, Apply/Cancel, stack enable/disable/reorder/duplicate/remove, undo and source-preserving architecture. All layer/source/filter masks continue to participate. Scaling a large preview scales spatial settings and rounds whole-pixel radii. Native `.palette` v4 descriptors preserve the typed enum variants and named stroke position without changing the container. Older readers reject unfamiliar variants explicitly. Compatible `.paint` export uses a separately baked copy after the existing native-save guard; direct native Microsoft Paint validation of these new outputs has not been performed.

## Tests and native captures

- Formatting and strict workspace/all-target Clippy pass; the Windows executable builds and launches.
- `cargo test --workspace --locked`: **176 tests** (82 app, 94 core); the two desktop functions are ignored in this ordinary run.
- Five new core tests independently check adjustment values/neutral settings, alpha preservation, naive box/median references at image/tile boundaries, hidden RGB, cancellation, inner-style coverage, all three stroke placements, subpixel width, layer opacity, source changes, invalid parameters/atomic-save protection and center-stroke allocation accounting.
- Every one of the 18 registered operations is exercised through native source/mask/property reopening, region/full-composite equality and baked `.paint` pixel comparison. Earlier mathematical, file, selection and history tests remain passing.
- Two new UI tests exercise actual Filter > Adjustments/Noise navigation, Hue/Saturation slider input/live canvas preview, Stroke position selection, Apply/Cancel and undo. The compact-toolbar test now covers all registry entries at 1×/1.5×/2× DPI.
- Native regression covers **48 states at 1386 × 963**, including seven new category/operation scenes. The 780 × 520 stroke toolbar was also visually inspected. Captures were reviewed before updating baselines.

## Measured costs

`cargo run -p palette-core --example processing_benchmark --locked`, one local optimized-development run (core opt-level 2):

| Raster | Box Blur, radius 4 | Median, radius 1 | Gaussian 8 + shadow/glow, cold / repeat |
|---|---:|---:|---:|
| 1280 × 720 | 30 ms | 91 ms | 367 / 69 ms, one cache hit |
| 3840 × 2160 | 236 ms | 832 ms | 3.29 / 3.42 s, over-budget stack recomputed |

The workload is a full-sized transparent raster with a solid rectangle in its central quarter, so Median mostly visits uniform windows. These are single CPU observations including output construction and excluding UI, GPU upload, file operations and presentation. They are not noisy-image, maximum-radius, full multistyle 4K/8K responsiveness guarantees. Source/output/intermediate retained rasters still consume full-sized storage. Box tile scratch is at most about 1.6 MiB; Median window scratch is under 1 KiB at maximum radius, excluding allocation metadata. Median work grows with the number of neighborhood samples and can be costly on noisy images.

Existing 128 MiB/eight-entry prepared-layer caching remains conservative. Stroke Center stores two style planes, both counted by the 512 MiB per-layer preflight. Disabled styles do not consume that rendering estimate. These bounds do not cap total process memory. Main render jobs still run to completion; only modal-independent parameter-preview processing supports in-flight cancellation.

## Remaining conventional equivalents

These are implementation candidates, not active menu entries or completed features:

| Conventional operation | Independent implementation approach |
|---|---|
| Levels / Curves | Validated ordered input points and sampled transfer lookup tables, with explicit working-space policy |
| Motion / Radial Blur | Premultiplied directional/rotational sampling with bounded support and preview scaling |
| Add Noise / Clouds | Documented deterministic seeded distributions or multiscale value noise |
| High Pass | Original minus a Gaussian low-pass, centered on neutral gray, with defined alpha handling |
| Mosaic / Color Halftone | Block-area premultiplied averaging / independently generated screen patterns |
| Emboss / Find Edges | Directional derivative / gradient convolution with explicit edge policy |
| Gradient / Pattern Overlay | Parameterized ramps or embedded repeatable assets through the source alpha |
| Bevel and Emboss | Alpha-derived height gradients, normals and directional light shading |
| Satin | Shifted blurred-silhouette differences with editable color/opacity/offset |

Filter Gallery, Levels/Curves, remaining filters/styles, round-distance stroke joins, profile-aware processing, per-stage/tile caching, GPU evaluation and execution on unavailable platforms remain open. This batch does not claim Photoshop-identical algorithms or color-management parity.

Public references consulted: [W3C Filter Effects](https://www.w3.org/TR/filter-effects-1/), [Adobe's conventional filter catalog](https://helpx.adobe.com/photoshop/using/filter-effects-reference.html) and [public layer-style workflow descriptions](https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/add-layer-styles.html). Algorithms, tests and icons are independently implemented; no proprietary code or resources were copied. Changes remain uncommitted.
