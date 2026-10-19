# File compatibility

## Current support

| Format | Open | Write | Meaning |
|---|---|---|---|
| `.palette` v1/v2/v3/v4/v5/v6 | Yes | Save/Save As writes v6 | All implemented editable model state; older versions migrate on explicit save |
| PNG | Yes | Export | Lossless RGBA; alpha round-trip tested |
| JPEG | Yes | Export | Lossy RGB; transparency composited over white |
| BMP | Yes | Export | Raster; alpha behavior depends on BMP variant |
| GIF | First frame | Still export | Indexed colors; limited transparency; no animation preservation |
| TIFF | First page, with notice | Export | RGBA round-trip tested; multipage editing/metadata preservation not supported |
| WebP | First frame if animated, with notice | Export lossless | RGBA round-trip tested; no animation preservation |
| AVIF | Still image | Export with quality/speed | Real 8-bit RGBA/RGB, 10-bit decode, dimensions/alpha/quality tested on Windows |
| Microsoft `.paint` | Verified subset | Experimental layered export | Two editable specimen layers locally; user confirms generated output opens/looks correct in Paint |
| HEIF/HEIC | No | No | No libheif dependency linked yet |

Raster imports create a single editable source layer. They do not recreate layers absent from a flat format. Raster export writes the rendered composite; the project path and dirty state remain unchanged. Save creates a `.palette` project, even when the tab originated from PNG/JPEG. Unsupported output extensions fail before touching the target.

The six ordinary raster formats use image 0.25.10 with explicitly enabled codecs. Still AVIF uses avif-decode 3.0/rav1d and ravif 0.13/rav1e. Tests exercise dimensions for all six formats and exact RGBA for PNG/TIFF/WebP. Images are converted to straight-alpha RGBA8 in assumed sRGB. Ordinary decoders now apply Exif orientation and retain original ICC/Exif bytes in the native document. Working profile conversion is not implemented; flattened exports and AVIF imports do not preserve metadata. Animation is not preserved. `open_with_report` returns a transient notice based on the detected codec, including misnamed raster files. Animated WebP, APNG (PNG default image), GIF (first frame) and TIFF (first page) notices are presented for both opening and importing layers. Still PNG/WebP imports have no animation notice; a TIFF notice describes the first-page policy even for a single-page file.

## `.palette` version 6 — current writer (versions 1–5 migration)

| Offset | Encoding | Value |
|---|---|---|
| 0 | 8 bytes | ASCII `PALETTE` followed by NUL |
| 8 | little-endian u32 | `6` |
| 12 | little-endian u32 | UTF-8 JSON manifest byte length |
| 16 | bytes | JSON manifest |
| After manifest | repeated records | little-endian u64 compressed length, then one zlib stream |

The manifest contains `width`, `height`, `background` (four straight-alpha RGBA8 canvas-backing channels), `active_layer`, `next_id`, `layers`, optional `selection`, ICC/Exif asset indices (`icc`, `exif`) and `asset_lengths`. Each layer retains all v1 properties. A source descriptor contains width, height and a row-major vector of tile asset indices; each tile is exactly 65536 bytes (128 x 128 straight RGBA8), including defined padding. A mask descriptor contains width, height, enabled and one asset index; its asset contains exactly width x height coverage bytes. Filter records retain ID, enabled, typed filter and optional mask descriptor; effect records retain existing typed style data. ICC/Exif assets contain original opaque bytes, with an empty asset when absent. Pixels use the existing RGBA8/sRGB working interpretation; storing a profile does not apply it.

Asset records follow `asset_lengths` order, zero-based. Shared Arc backing buffers have one asset, preserving tile sharing on load. Zlib checksums, exact decoded lengths, consumed compressed lengths and trailing-file rejection detect malformed/truncated data. The reader preflights references, dimensions and counts before decompression, then validates the reconstructed model. Unknown versions or enum variants fail explicitly. Limits: 4 MiB manifest, 65536 assets, 512 MiB summed unique decoded bytes, 1 GiB file, 4 MiB each ICC/Exif field, existing 64-megapixel plane and model count limits. These are format/allocation policies rather than a total process-memory cap.

Saving streams assets directly to the same-directory temporary file, backfilling each compressed length with seeks. It does not serialize pixels to JSON or hold every compressed asset in memory. Metadata descriptors remain a small bounded JSON buffer. Existing v1 projects are read into the current model without rewriting the input; an explicit successful Save writes v6 atomically. Version 2 uses the same binary layout without `background`; its reader defaults this field to transparent. Version 3 introduced backing color. Version 4 additionally stores optional `selection_base`, a canvas-sized coverage asset containing the original contour before feathering. Existing versions default it to absent. Older builds reject v4 at version dispatch, preventing silent loss of the retained original during re-save. No future-format placeholders or silent downgrade are used.

## `.palette` version 1 — legacy reader

This extension belongs only to Palette. It is unrelated to Microsoft's `.paint` container.

| Offset | Encoding | Value |
|---|---|---|
| 0 | 8 bytes | ASCII `PALETTE` followed by NUL |
| 8 | little-endian u32 | `1` |
| 12 onward | zlib stream | UTF-8 JSON serialization of `Document` |

Document JSON includes canvas width/height, bottom-to-top layers, active-layer ID, selection coverage and next-ID counter. A raster includes width/height and a row-major array of 128 × 128 RGBA byte tiles, including padding. Byte values are serialized as JSON integers. Arc ownership is not a file-format reference mechanism: shared tiles serialize as values. Coverage contains width/height, values and enabled state.

Each layer serializes its ID, name, original raster, visibility, lock, opacity, blend, signed offset, optional mask, filters and effects. Filter instances retain their ID, enabled state, typed parameters and optional mask. Style instances retain their ID, enabled state and typed parameters. All represented properties round-trip losslessly. Viewports and undo history are session state and are not stored.

The text tool commits its preview as an ordinary separate raster layer, so text pixels and position round-trip through v1 without changing the schema. Text characters, font choices and uncommitted preview transactions are not saved as editable typography. Apply or Cancel finishes the modal transaction before saving.

Version dispatch occurs before decompression. Unknown versions or enum variants fail explicitly. Readers validate dimensions, pixel/tile/coverage counts, active IDs, ID uniqueness, parameter ranges and layer/filter/effect counts. Selection coverage must match canvas dimensions. Compressed and decompressed project inputs are capped at 256 MiB. Saves also cap uncompressed serialized size, ensuring successfully saved files can reopen under the same limits. Phase 2 uses the existing selection/offset/mask fields and does not change the format version.

The v1 interpretation is unchanged. Its migration reader defaults newly introduced opaque document metadata to empty; v2 separates descriptors and binary data. Both v1 compressed input and decompressed JSON retain the 256 MiB cap.

## Atomic writes

Both project saves and raster exports encode into a temporary file in the target's directory, sync it, then use tempfile's replacement operation. A codec, serialization, validation or disk-write failure before replacement leaves the old target intact. Temporary files are removed by RAII. Filesystem durability across power loss still depends on the OS/filesystem; directory fsync is not implemented.

## Microsoft Paint projects — experimental verified subset

The dedicated safe-Rust handler reads/writes actual ISO BMFF HEIF with `mif1/gcmi/isoa/miaf`, separate `unci` sources, a primary `iovl`, ordered `dimg` references, source properties and signed offsets. The supplied 819 x 594 project imports as exactly two editable raster layers (red below black smiley), retaining transparent pixels and partial alpha. Its white overlay backing is document state, not a third raster layer. Exif TIFF and a shared ICC profile are retained. The writer constructs a fresh item graph and performs atomic replacement. No unrelated `.palette` data is renamed `.paint`.

Supported source layout: packed unsigned 8-bit RGBA components with explicit component mapping, pixel interleave, no padding/tiling, whole-item raw Deflate or zlib (or uncompressed). Overlay offsets support 16/32-bit signed fields. Reject encrypted/external/indexed data, unsupported layouts/transforms/essential properties, cycles/nested derived sources and mixed profiles. Unknown nonessential properties and unrelated boxes are tolerated, but are not preserved on export. Limits: 256 MiB input/output, 4096 boxes, 1024 items, 256 direct sources, eight extents/item, 512 MiB aggregate decoded source pixels, 16384 per dimension, existing 64-megapixel plane limit. Decompression requires exact end, consumed input and output lengths.

Open creates an imported tab; Save/Save As preserves complete state as `.palette`. File > Export includes an experimental layered `.paint` copy. Before baking masks, filter/style stacks, opacity, visibility/locks or blend state, the dialog requires a successful `.palette` save of the same document revision. Cancel writes nothing. Local processing is baked separately into source layers; backdrop-dependent blends merge the necessary bottom prefix while retaining later layers. Alpha baking may round a channel by one unit. The original document stays editable. Unsupported names/Exif or encoding failures preserve an existing destination.

Local source/composite round trips and an independent raw-BGRA/Deflate/Pillow oracle pass. The user confirms the generated file opens and looks correct in Microsoft Paint. Independently editing its layers there and saving back remain unverified. This is not a claim of compatibility with every Paint release or variant. See [specimen evidence and validation steps](PAINT_RESEARCH.md).

## Shape and layer movement preservation

The shape and polygon/curve tools commit raster pixels and selection coverage; a native project preserves those pixels and coverage, not a persistent editable path. Selection transforms and arbitrary rotation preserve current layer processing properties without adding new format structures. Whole-layer movement changes the existing signed layer offset and preserves source, masks, filters and effects. The underlying selection/layer model remains compatible with the v1 migration reader; the current writer stores it in v6. A translated layer with a mask and live filter has a real `.palette` round-trip test. These geometry changes do not establish `.paint` interoperability. AVIF support is independently provided by the codec path above.

## Still AVIF scope

Export settings: RGB quality 1–100 (default 80), speed 1–10 (default 8), 8-bit output, maximum alpha quality, up to four encoder threads. Opening verifies coded sequence dimensions (maximum 16384 per axis and 64 megapixels), one sequence header per primary/alpha item and the existing 256 MiB input bound. Decoder output is converted to RGBA8; 8/10-bit inputs are tested, while the implemented 12-bit downconversion still needs an external fixture. Non-still/animation streams and HEIF irot/imir/clap properties are rejected explicitly because this decoder adapter does not apply them. HDR/profile-aware conversion and broader derived-image conformance remain open. RGB is lossy; use native project saving or PNG for lossless pixels.

## Recent files and recovery

State directory: `%APPDATA%/Palette` on Windows, `~/Library/Application Support/Palette` on macOS, `$XDG_STATE_HOME/palette` or `~/.local/state/palette` on Linux. `recent.json` atomically stores at most ten successfully opened/imported/saved source paths (flattened exports are not added). Missing recent paths are disabled. Multiple instances have last-writer-wins recent-file state.

`recovery/` holds independent `.palette` snapshots and JSON sidecars named from a unique instance ID and tab ID. A per-instance `.lock` file uses an OS file lock; another live instance's snapshots are skipped. Each sidecar holds document name, original native path when any, and a validated sibling project filename. No path traversal/archive extraction occurs. Metadata reads/writes are bounded to 64 KiB. A damaged sidecar reports a recovery-list error; the sibling native project can still be opened manually. Old lease files remain harmless evidence of ownership.

Stable committed changes are captured about every 30 seconds, one job at a time. Long active strokes/drafts defer snapshots. Save/Export dirty state and original files are untouched by autosave. Startup Restore creates a dirty tab; Discard removes that snapshot; Later retains it and enables File > Recover unsaved documents. Restored originals remain until an explicit save or replacement recovery succeeds. Normal close/discard and confirmed clean saves remove owned recovery files; a late result checks whether the document has closed or become clean. OS close waits for a recovery write before resuming save/close handling. Recovery does not include undo history, viewports, uncommitted text/construction drafts, or guaranteed capture of the final interval before a crash. File sync/atomic replacement is used, with the existing filesystem/power-loss durability limits.

## Phase 4 preservation

All 19 blend modes serialize as named enum variants in the layer descriptors. Older builds reject unfamiliar variants explicitly. Wand/rectangle results, fractional selection coverage and local layer masks are stored as ordinary coverage assets; no flattening is required. Version 4 also preserves original feather contours; pre-v4 files migrate without this optional field. Selection-tool options are per-tab UI state, not file data. Filters/styles and source RGB/alpha stay intact when masks are created or edited. Export to `.paint` continues to require native preservation before baking unsupported blending/masks.

## Phase 5 processing preservation

Native v4 descriptors now support `GaussianBlur { radius }`, `DropShadow { color, opacity, angle, distance, spread, size }` and `OuterGlow { color, opacity, spread, size }` in the existing ordered filter/effect arrays. Radius/size/distance are source-image pixels; the blur radius is Gaussian sigma. Opacity/spread are normalized 0–1 and light angle is degrees. Stable IDs, enabled states, filter coverage masks and original source assets survive reopening. No container-layout change is required; old builds reject unknown enum variants explicitly. Values are validated before writing or rendering. New tests round-trip native sources/settings and compare exported baked Paint appearance including spatial extents. Paint cannot preserve these live operations: the existing complete native-save guard remains mandatory before compatible export. No new Microsoft Paint save-back test is claimed.

### Additional Phase 5 variants

Native v4 now also preserves `BrightnessContrast { brightness, contrast }`, `Exposure { stops }`, `HueSaturation { hue, saturation, lightness }`, `Vibrance { amount }`, `BlackWhite { strength }`, `Threshold { level }`, `Posterize { levels }`, `BoxBlur { radius }` and `Median { radius }`. Threshold/Posterize/Box/Median fields are typed unsigned bytes with operation-specific validation. Exposure uses EV; hue is degrees; brightness/saturation/lightness/vibrance use normalized signed values. Effects add `InnerShadow` (the same fields as DropShadow), `InnerGlow` (the same fields as OuterGlow), and `Stroke { color, opacity, width, position }`. Stroke position is the named enum `Outside`, `Inside` or `Center`; width is image pixels. Radius limits are listed in docs/PROCESSING_CATALOG_VALIDATION.md. No container version/layout changed; unknown variants fail explicitly in older readers. Every registered operation is exercised through source/mask/property-preserving native reopening and separate baked Paint pixel comparisons. Native Microsoft Paint validation of new exports remains an external gate.


## Version 5 processing additions

The binary manifest/asset layout is unchanged from v4. Stroke now stores `rounded: bool`; missing fields in older inputs default false, retaining their original square corners. New editor strokes default true. The version bump prevents older readers ignoring this field and silently losing the intended appearance. Versions 1–4 still load without rewriting the input. Glow UI Radius maps to the existing `size` field, with no metadata rename.

New typed filter variants are `Levels { black:u8, white:u8, gamma:f32, output_black:u8, output_white:u8 }`, `Curves { points:[u8;5] }`, `MotionBlur { distance:f32, angle:f32 }`, `HighPass { radius:f32 }`, `Mosaic { block_size:u8 }` and `AddNoise { amount:f32, seed:u16, monochrome:bool, distribution }`. Noise distribution is the named enum `Uniform` or `Gaussian`. Levels rejects black >= white; every parameter is validated against the registry before rendering or saving. Exact domains/alpha policies are in [current validation](STYLE_FILTER_CONTROLS_VALIDATION.md).

All 24 registered operations are tested with nondefault parameters through native reopen and separate baked `.paint` pixel comparisons. Version-4 Stroke input lacking the new field retains square corners and unchanged source bytes. These automated Paint comparisons do not replace Microsoft's application save-back validation. Current `.paint` layout and export warnings are unchanged.


Version 5 also supports the typed filters `Average`, `FindEdges`, `Solarize`, `Emboss { angle:f32, height:u8, amount:f32 }`, `UnsharpMask { radius:f32, amount:f32, threshold:u8 }`, `Minimum { radius:u8 }`, `Maximum { radius:u8 }`, `Offset { x:i32, y:i32, edges }`, `Twirl { radius:f32, angle:f32 }` and `Wave { amplitude:f32, wavelength:f32, vertical:bool, edges }`. `FilterEdges` uses stable named values `Wrap`, `Repeat`, `Transparent`; numeric UI choice indices are not serialized. There is no container/version change for these new enum variants; previous readers reject unfamiliar variants instead of dropping them. That 34-operation batch receives native and baked-Paint round-trip tests; the current additions are listed below. See [current filter validation](FILTER_MENU_VALIDATION.md). The disabled future menu inventory is UI metadata, not serialized placeholder operations.


Further v5 filter variants: `SurfaceBlur { radius:u8, threshold:u8 }`, `RadialBlur { amount:f32, samples:u8, zoom:bool }`, `Clouds` / `DifferenceClouds { seed:u16, scale:f32, low:[u8;3], high:[u8;3] }`, `PosterEdges { thickness:u8, intensity:f32, levels:u8 }`, `Grain { amount:f32, contrast:f32, size:f32, seed:u16, monochrome:bool, style }`, and `Fragment { distance:f32 }`. Grain style is a named enum: `Regular`, `Soft`, `Speckle`, `Horizontal`, `Vertical`. The native version/container does not change; unknown variants fail in older readers. At that milestone, all 41 registered operations were covered by nondefault native reopening and baked Paint appearance comparisons. No new native Microsoft Paint save-back check is claimed. See [creative filter validation](CREATIVE_FILTERS_VALIDATION.md).


Additional v5 filters: `Despeckle`, `DustScratches { radius:u8, threshold:u8 }`, `ReduceNoise { radius:u8, strength:f32, preserve_details:f32, color_noise:f32, sharpen:f32 }`, `Pinch { amount:f32, radius:f32 }`, `Spherize { amount:f32, radius:f32 }`, `Ripple { amplitude:f32, wavelength:f32 }`, `Sharpen`, and `SharpenMore`. Strength/detail/color/sharpen values are normalized 0–1; Pinch/Spherize amount is signed −1…1. UI percentages are presentation only. The v5 layout is unchanged; older readers reject unknown variants explicitly. At that milestone, nondefault native/mask/source/Paint comparisons covered all 49 operations. [Parameters and test scope](RETOUCH_DISTORT_VALIDATION.md).

## Version 6: movable filter origins

The manifest/assets layout is unchanged. `Twirl`, `Wave`, `RadialBlur`, `Pinch`, `Spherize` and `Ripple` now store `center: [f32;2]`. Each finite component is 0–1, relative to the source pixel-center grid: `center_x * (source.width - 1)`, `center_y * (source.height - 1)`. A one-pixel axis always resolves to zero. Layer offsets are applied only during composition/display; preview resampling retains the normalized values. Missing fields in v1–v5 migrate to `[0.5,0.5]`, except Wave's historical phase origin `[0,0]`. Inputs are never rewritten on read. Version dispatch prevents older builds silently dropping new fields of known variants.

New typed variants: `PolarCoordinates { center, to_polar:bool }`, `ZigZag { center, amount:f32, radius:f32, ridges:u8, style }`, `Fibers { seed:u16, variance:f32, strength:f32, low:[u8;3], high:[u8;3] }`, and `GlowingEdges { width:f32, brightness:f32, smoothness:f32 }`. ZigZag styles serialize as `AroundCenter`, `OutFromCenter`, `PondRipples`. Registry domains validate every field before rendering/atomic encoding. At the origin milestone, round-trip comparisons covered all **53 operations**, including individual masks and preserved source planes. [Current evidence](ORIGIN_FILTERS_VALIDATION.md).

## Additional version 6 artistic/print/texture filters

New typed variants: `Cutout { levels:u8, simplicity:f32, fidelity:f32 }`, `FilmGrain { grain:f32, highlight:f32, intensity:f32, seed:u16 }`, `AccentedEdges { width:f32, brightness:f32, smoothness:f32 }`, `InkOutlines { length:f32, dark:f32, light:f32 }`, `Photocopy { detail:f32, darkness:f32, low:[u8;3], high:[u8;3] }`, `HalftonePattern { size:f32, contrast:f32, pattern, low:[u8;3], high:[u8;3] }`, `Patchwork { square:u8, relief:f32 }`, and `Craquelure { spacing:f32, depth:f32, brightness:f32, seed:u16 }`. Pattern names are `Dot`, `Line`, `Circle`. Normalized fields use 0–1 rather than displayed percentages; pixel widths, relief, seeds and counts retain their registry domains.

The v6 manifest/asset layout is unchanged. Older readers reject unknown enum variants explicitly. Source/mask/order/property preservation, nondefault native reopening, region/full rendering and separately baked Paint pixels at that milestone covered all **61 registered operations** (55 filters plus six styles). No new native Microsoft Paint save-back validation is claimed. [Current validation](ARTISTIC_FILTERS_VALIDATION.md).

## Additional version 6 pixelate/cell/diffusion filters

New variants: `ColorHalftone { radius:f32, cyan:f32, magenta:f32, yellow:f32, black:f32 }`, `Crystallize { size:u8, seed:u16 }`, `Facet`, `Mezzotint { pattern, scale:f32, seed:u16, monochrome:bool }`, `Pointillize { size:u8, dot:f32, color:[u8;3], seed:u16 }`, `StainedGlass { size:u8, width:f32, light:f32, color:[u8;3], seed:u16 }`, `MosaicTiles { size:u8, grout:f32, lighten:bool }`, `Diffuse { radius:u8, mode, seed:u16 }`. Angles are degrees. Dot/light values are normalized 0–1. Mezzotint stores named `FineDots`, `MediumDots`, `GrainyDots`, `CoarseDots`, `ShortLines`, `MediumLines`, `LongLines`, `ShortStrokes`, `MediumStrokes`, `LongStrokes`. Diffuse modes are `Normal`, `DarkenOnly`, `LightenOnly`, `Anisotropic`.

Registry keys `cell_size`, `tile_size` and `pattern_scale` map to the typed size/scale fields; presentation labels are not wire data. The manifest/asset layout and version remain v6; older readers explicitly reject unknown variants. Nondefault source/mask/order/settings preservation, region/full composition and baked Paint comparisons now cover all **69 registered operations** (63 filters and six styles). No new Microsoft Paint application save-back validation is claimed. [Scope](PIXELATE_FILTERS_VALIDATION.md).

## Additional version 6 color adjustments and fixed blurs

New named variants: `ColorBalance` (3×3 balance matrix, preserve_luminosity), `PhotoFilter` (RGB, density, preserve_luminosity), `ChannelMixer` (3×4 matrix, monochrome), `GradientMap` (low/high RGB, midpoint, reverse), `SelectiveColor` (9×4 adjustments matrix, relative), `ShadowsHighlights` (shadows/shadow_width/shadow_radius and highlights/highlight_width/highlight_radius), `Blur` and `BlurMore`. Array ordering is documented on the model variants and in the adjustment validation note. Presentation names and selected toolbar parameters are not serialized.

The v6 manifest/asset layout is unchanged; earlier v1–v5 readers remain supported. Older application builds reject unfamiliar variants. Nondefault native reopening, original sources, filter/layer masks, properties and order, region/full rendering and separately baked Paint appearance now cover **77 registered operations** (71 filters plus six styles). This is not new Microsoft Paint application save-back validation. [Current adjustment scope](ADJUSTMENT_FILTERS_VALIDATION.md).

## Additional version 6 traditional-media filters

New variants: `ColoredPencil` (width, pressure, brightness, paper RGB, seed), `DryBrush` (radius, detail, texture, seed), `PaintDaubs` (radius, sharpness, style), `SmudgeStick` (length, highlight, intensity), `AngledStrokes` (balance, length, sharpness), `Crosshatch` (length, sharpness, strength), `DarkStrokes` (balance, black, white), `Spatter` (radius, smoothness, seed). Dimensions are f32 image-space pixels; strengths are f32 0–1; seeds are u16. `DaubStyle` serializes named `Round`, `Square` or `Diagonal` variants; UI choice indices are not stored wire data. Pencil paper is straight RGB and does not introduce additional alpha.

These additions retain the v6 bounded manifest/asset layout and v1–v5 migration readers. Older builds reject unfamiliar variants explicitly. Nondefault native reopening, source/mask/settings/order preservation, region/full rendering and separately baked Paint appearance now cover **85 registered operations** (79 filters plus six styles). No new Microsoft Paint application edit/save-back validation is claimed. [Exact scope](PAINTING_FILTERS_VALIDATION.md).

## Additional version 6 directional spray and Sketch filters

New variants: `SprayedStrokes` (length, radius, direction, seed), `SumiE` (width, pressure, contrast), `BasRelief` (detail, smoothness, angle, low/high RGB), `ChalkCharcoal` (charcoal, chalk, pressure, seed, low/high RGB), `Charcoal` (width, detail, balance, low/high RGB), `GraphicPen` (length, balance, direction, low/high RGB), `NotePaper` (balance, grain, relief, seed, low/high RGB), `Stamp` (balance, smoothness, low/high RGB). Dimensions are f32 source pixels, strengths/balance are f32 0–1, light azimuth is f32 degrees and seeds are u16. `StrokeDirection` stores named `RightDiagonal`, `LeftDiagonal`, `Horizontal` or `Vertical` variants. UI indices and drawing-color changes are not wire state.

The bounded v6 manifest/asset layout and v1–v5 migration readers remain unchanged. Older builds explicitly reject unfamiliar filter variants. Native reopening with nondefault settings, original sources, filter/layer masks, ordering, region/full composition and separately baked Paint appearance now cover **93 registered operations** (87 filters plus six styles). No new Microsoft Paint application edit/save-back validation is claimed. [Exact scope](DRAWING_FILTERS_VALIDATION.md).

## Additional version 6 Sketch completion filters

New variants: `Chrome` (detail, smoothness), `ConteCrayon` (foreground/background emphasis, texture, scale, relief, angle, invert, seed, low/high RGB), `Plaster` (balance, smoothness, angle, low/high RGB), `Reticulation` (density, foreground/background emphasis, grain_size, seed, low/high RGB), `TornEdges` (balance, smoothness, contrast, seed, low/high RGB), `WaterPaper` (length, brightness, contrast, seed). Spatial values are f32 source pixels, strengths are f32 0–1, brightness is −1–1, Water Paper contrast is −0.99–0.99, azimuth is f32 degrees and seeds are u16. Inversion is boolean. `SurfaceTexture` stores named `Canvas`, `Brick`, `Burlap` or `Sandstone` variants; UI indices are not serialized. Chrome has no color endpoints and Water Paper retains original colors.

The bounded v6 layout and v1–v5 migration readers are unchanged. Older application builds explicitly reject unknown variants. Nondefault native reopening, source/mask/settings/order preservation, region/full rendering and separately baked Paint pixels now cover **99 registered operations** (93 filters plus six styles). This is not new Microsoft Paint application edit/save-back validation. [Scope](SKETCH_COMPLETION_VALIDATION.md).

## Additional version 6 Artistic completion filters

New variants: `Fresco` (radius, detail, texture, seed), `NeonGlow` (size, brightness, color, low/high RGB), `PaletteKnife` (length, detail, smoothness), `PlasticWrap` (strength, detail, smoothness), `RoughPastels` (length, detail, texture, scale, relief, angle, invert, seed), `Sponge` (radius, definition, smoothness, seed), `Underpainting` (radius, coverage, texture, scale, relief, angle, invert, seed), `Watercolor` (detail, shadows, texture, seed). Numeric values are f32; radius/length/smoothness/scale are source pixels. Strength/detail/texture/coverage/shadows/brightness are 0–1; Neon size is −1–1 tonal placement, not pixels. Material texture stores a named SurfaceTexture, not a number; azimuth is degrees, inversion boolean, seeds u16, colors straight RGB8. Neon captures drawing colors only on creation; editing retains saved values.

The bounded native v6 layout and v1–v5 migration readers remain unchanged. Older builds reject unfamiliar variants. Nondefault native reopening, original sources, masks, settings/order, region/full rendering and separately baked Paint appearance cover **107 registered operations** (101 filters plus six styles). No new Microsoft Paint application edit/save-back test is claimed. [Scope](ARTISTIC_COMPLETION_VALIDATION.md).

## Additional version 6 optical/texture filters

New variants: `DiffuseGlow` (amount, clear, radius, grain, seed, color, center), `Glass` (distortion, smoothness, scale, texture, invert, seed, edges, center), `OceanRipple` (amplitude, wavelength, seed, edges, center), `Texturizer` (texture, scale, relief, angle, invert, seed), `ShapeBlur` (radius, shape, angle), `SmartBlur` (radius, threshold, mode), unit `SharpenEdges`, `SmartSharpen` (radius, amount, reduce_noise, iterations). Numeric values are f32 except Smart Blur radius/Smart Sharpen iterations (u8) and seeds (u16). Origins are normalized [f32;2]; colors straight RGB8; inversion boolean; pixel dimensions scale for previews. Glass smoothness is normalized, rather than pixels. Smart Sharpen amount 1 means 100%; threshold is encoded-RGB levels.

Stable new option names: GlassSurface { Frosted, TinyLens, Blocks, Canvas }; ApertureShape { Disk, Triangle, Square, Hexagon, Octagon }; SmartBlurMode { Normal, EdgeOnly, OverlayEdge }. Existing SurfaceTexture and FilterEdges names are reused. Native v6 layout and v1–v5 migration are unchanged; older builds reject unknown variants. All **115 registered operations** (109 filters, six styles) undergo nondefault native reopening, source/mask/settings/order retention, region/full equality and separately baked Paint pixel comparisons. This is not a new Microsoft Paint application edit/save-back validation. [Scope](OPTICAL_FILTERS_VALIDATION.md).

## Additional version 6 Stylize and Render completion

New variants: `Extrude` (cell_size, depth, shape, random, seed, center), `OilPaint` (radius, stylization, cleanliness, scale, bristles, lighting, angle, shine), `Tiles` (tile_size, offset, fill, seed, low/high RGB), `TraceContour` (level, upper), `Wind` (length, method, from_right, seed), `LensBlur` (radius, shape, angle, depth_source, focus, invert, highlights, threshold, noise, seed), `LensFlare` (brightness, radius, style, color, center), `LightingEffects` (kind, intensity, ambient, radius, angle, elevation, gloss, metallic, relief, color, center).

Values are f32 except u16 cell_size/tile_size/seeds and u8 contour level. Booleans are named explicitly. Source-pixel dimensions scale for previews; normalized strengths/depth, angles in degrees, encoded-level thresholds, colors and seeds do not. Origins are normalized [f32;2], colors straight RGB8, Tiles offset a fraction of its tile extent. Named option variants: ExtrudeShape {Blocks,Pyramids}; TileFill {Unaltered,Transparent,Foreground,Background}; WindMethod {Wind,Blast,Stagger}; DepthSource {Uniform,Alpha,Luminance}; FlareStyle {Zoom,Prime,Movie,Anamorphic}; LightKind {Point,Spot,Directional}. Lens Blur reuses named ApertureShape. Tile colors capture current drawing colors only at creation.

Native v6 layout and v1–v5 migration remain unchanged; older builds reject unknown variants. All **123 registered operations** (117 filters, six styles) check nondefault native reopening, sources/masks/settings/order, region/full equality and separately baked Paint pixels. No new Microsoft Paint application edit/save-back validation is claimed. [Exact scope](STYLIZE_RENDER_VALIDATION.md).

## Version 6 embedded LUT and displacement assets

New filters: `ColorLookup` (amount, lut), `Displace` (horizontal, vertical, fit, edges, center, map, map_name), `Shear` (five normalized horizontal curve offsets, interpolation, edges, center), `Custom` (25 row-major f32 coefficients, divisor, bias, alpha boolean, edges). MapFit serializes Stretch/Tile; ShearInterpolation Linear/Smooth. Lookup amount is normalized; displacement is source pixels; kernel coefficients −64–64, divisor −1024–1024 excluding magnitudes below 1e−6, bias −255–255. Curve values are −1–1; origins normalized. Boundaries reuse named FilterEdges.

A FilterRecord now optionally has `map: RasterRecord` or `lut: LutRecord`. In the manifest's cloned Filter value, the corresponding large `map`/`lut` payload is null. Displace requires exactly one map record and no LUT record; ColorLookup requires exactly one LUT record and no map record. Other filters reject extra asset records. Map records reuse source tile descriptors and 65536-byte RGBA assets, with dimensions limited to 1024×1024. LUT records store name, size, domain_min/domain_max and one asset index. Its asset is exactly size³×12 bytes of little-endian f32 RGB triples, red-fastest. Size is 2–33; values must be finite and within −16–16; each finite domain is increasing and bounded to −16–16. Names are at most 128 non-control characters. Table values are validated after decoding; references/lengths/dimensions are preflighted before decompression.

Shared map tiles and LUT tables are stored once and restored with shared backing; LUT names/domains can differ while sharing the same value table. Existing 4 MiB manifest, 512 MiB total decoded assets and 1 GiB file limits still apply. Imported source paths are not required or stored as dependencies. v6 header/asset framing and v1–v5 migrations remain unchanged; older builds reject unfamiliar filter variants rather than silently losing data. All **127 registered operations** (121 filters, six styles) retain nondefault settings, sources, masks, ordering, region/full equality and separately baked Paint appearance. [Validation](EDITABLE_FILTERS_VALIDATION.md).


## Additional version 6 photographic, procedural and Video filters

New variants: FieldBlur (eight BlurPin {center, radius} values, selected pin and active count); IrisBlur (radius, width, height, angle, feather, center); TiltShift (radius, width, transition, angle, center); PathBlur (distance, taper, centered, three normalized path coordinates, selected point); SpinBlur (angle, width, height, feather, rotation, center); Flame (dimensions, turbulence, detail, seed, RGB color, opacity, angle, center); PictureFrame (width, margin, bevel, style, color, opacity); Tree (dimensions, branching, foliage, seed, form, leaf/bark colors, opacity, angle, center); NtscColors (setup); Deinterlace (field, interpolation); HsbHsl (input, output). New named options: FrameStyle {Plain, Beveled, Wood, Ornate}; TreeForm {Deciduous, Conifer, Bare}; VideoField {Odd, Even}; FieldInterpolation {Duplicate, Interpolate}; ChannelSpace {Rgb, Hsb, Hsl}.

Pin and point indices are zero-based; video field names refer to one-based image rows. FieldBlur count is 1–8, selected pin must be active, every stored pin radius is finite 0–128 and every center is normalized 0–1. Every PathBlur coordinate is validated even when its handle is not selected. Pixel extents, angles, colors and remaining domains use the typed registry. All data fits the existing bounded JSON metadata; no new assets or framing/version migration. Earlier application builds reject unfamiliar variants. The shared all-catalog native/mask/region/Paint checks now cover 138 operations (132 filters, six styles). [Validation](GALLERY_VIDEO_RENDER_VALIDATION.md).


## Bundled local neural profile

Version 6 metadata now permits `StyleTransfer { style: "MosaicV1", strength: 0..1 }`.
`MosaicV1` identifies the immutable `onnx-mosaic-9-fa646ded-v1` profile and SHA-256
`fa646dedade881243f8d5a2ceb7de2b93675b21fc24f7482894ac4851a9a0a47`.
The model bytes are embedded in Palette, not repeated in each project. The same
profile/algorithm must be retained by future readers; new weights need new IDs.
No pixel, mask, asset framing or prior variant changed, so no version migration
is needed. Older Palette builds reject this unfamiliar variant explicitly.
Strength and model identity remain editable after save/reopen, and the existing
native source/mask pipeline stays lossless. Flat or Paint export bakes appearance
under the existing unsupported-feature warning, without discarding the native
original. No network or external inference service is involved in reopening.


Version 6 also permits `JpegArtifactsRemoval { model: "Dncnn3V1", strength: 0..1 }`.
The immutable model profile is `dncnn3-6e7dcb19-v1`, with exact graph hash
`6e7dcb19f0f37aac698e846f0364bc01e0939d0582a8f3e974d8be8fd3388f24`.
Original pixels, dimensions, alpha, mask coverage and filter order are retained;
no new assets, framing or previous variants changed. The weights are bundled in
Palette, so opening the native project requires no download or external runtime.
Older builds reject the unfamiliar variant. Incompatible flat/Paint exports use
the existing native-backup and baked-appearance workflow. [Model contract](JPEG_RESTORATION_VALIDATION.md).


Native v6 also permits `Colorize { model: "Eccv16Fp16V1", strength: 0..1,
saturation: 0..2 }`. Profile `eccv16-f16-5ce703af-v1` pins graph SHA-256
`5ce703afdc738da5a435d23369c617004abea7be54342568b3ddd0fd96ac8938`, compact
kernel storage, Lab normalization, direct chroma reconstruction and gamut
handling. Weights are bundled in Palette, not repeated in native projects.
Original source pixels, masks, geometry and complete editable parameters survive
roundtrip. No existing variant/asset/framing changed; older builds reject the
new variant explicitly. A new weight or pixel contract requires a new profile.
See [Colorize validation](COLORIZE_VALIDATION.md).

Native v6 also permits `SkinSmoothing { model: "FaceMeshV1", strength, blur,
range }`, all numbers finite in 0..1. The model identifier pins bundled face
geometry and conventional smoothing conventions. Parameters are face-relative
and unchanged when building reduced previews. Source/masks/layer properties
and filter order remain separate assets/metadata. Old variants and container
framing are unchanged; older applications reject this new enum variant rather
than silently losing it. Paint and flat-image exports bake supported appearance
using the existing native-backup policy. [Exact scope](SKIN_SMOOTHING_VALIDATION.md).

Native v6 also permits `SuperZoom { model: "Espcn3xCropV1", zoom: 1..3,
enhancement: 0..1, center: [0..1, 0..1] }`, with finite numeric values. It is an
explicit source-sized crop; canvas/source dimensions and filter-mask coordinates
remain unchanged. Model weights are bundled in Palette; parameters are unchanged
in reduced previews. No framing/asset migration occurs, and older builds reject
the new enum variant. Sources remain lossless and the existing flat/Paint
compatibility policy applies. [Exact contract](SUPER_ZOOM_VALIDATION.md).

Native v6 also permits `ColorTransfer` with `method: "LabStatisticsV1"`, Strength,
Luminance and Contrast in finite 0..1 and Saturation in finite 0..2. Its embedded
`ColorReference` contains a sanitized name, original dimensions and three float64
Lab means/deviations. This is sufficient to reproduce this statistical method;
the full reference image is not embedded or required on reopen. Native parsing
uses `serde_json/float_roundtrip` for exact float64 preservation and validates
profile names, dimensions and bounded finite statistics before use. The model
does not contain a filesystem reference or a learned graph. No container framing
changed; older readers reject the unknown enum. [Exact contract](COLOR_TRANSFER_VALIDATION.md).
