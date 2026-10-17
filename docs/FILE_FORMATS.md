# File compatibility

## Current support

| Format | Open | Write | Meaning |
|---|---|---|---|
| `.palette` v1/v2/v3/v4 | Yes | Save/Save As writes v4 | All implemented editable model state; older versions migrate on explicit save |
| PNG | Yes | Export | Lossless RGBA; alpha round-trip tested |
| JPEG | Yes | Export | Lossy RGB; transparency composited over white |
| BMP | Yes | Export | Raster; alpha behavior depends on BMP variant |
| GIF | First frame | Still export | Indexed colors; limited transparency; no animation preservation |
| TIFF | Raster | Export | RGBA round-trip tested; multipage/metadata preservation not supported |
| WebP | Raster | Export lossless | RGBA round-trip tested; no animation preservation |
| AVIF | Still image | Export with quality/speed | Real 8-bit RGBA/RGB, 10-bit decode, dimensions/alpha/quality tested on Windows |
| Microsoft `.paint` | Verified subset | Experimental layered export | Two editable specimen layers locally; user confirms generated output opens/looks correct in Paint |
| HEIF/HEIC | No | No | No libheif dependency linked yet |

Raster imports create a single editable source layer. They do not recreate layers absent from a flat format. Raster export writes the rendered composite; the project path and dirty state remain unchanged. Save creates a `.palette` project, even when the tab originated from PNG/JPEG. Unsupported output extensions fail before touching the target.

The six ordinary raster formats use image 0.25.10 with explicitly enabled codecs. Still AVIF uses avif-decode 3.0/rav1d and ravif 0.13/rav1e. Tests exercise dimensions for all six formats and exact RGBA for PNG/TIFF/WebP. Images are converted to straight-alpha RGBA8 in assumed sRGB. Ordinary decoders now apply Exif orientation and retain original ICC/Exif bytes in the native document. Working profile conversion is not implemented; flattened exports and AVIF imports do not preserve metadata. Animation is not preserved.

## `.palette` version 4 — current writer (versions 1–3 migration)

| Offset | Encoding | Value |
|---|---|---|
| 0 | 8 bytes | ASCII `PALETTE` followed by NUL |
| 8 | little-endian u32 | `4` |
| 12 | little-endian u32 | UTF-8 JSON manifest byte length |
| 16 | bytes | JSON manifest |
| After manifest | repeated records | little-endian u64 compressed length, then one zlib stream |

The manifest contains `width`, `height`, `background` (four straight-alpha RGBA8 canvas-backing channels), `active_layer`, `next_id`, `layers`, optional `selection`, ICC/Exif asset indices (`icc`, `exif`) and `asset_lengths`. Each layer retains all v1 properties. A source descriptor contains width, height and a row-major vector of tile asset indices; each tile is exactly 65536 bytes (128 x 128 straight RGBA8), including defined padding. A mask descriptor contains width, height, enabled and one asset index; its asset contains exactly width x height coverage bytes. Filter records retain ID, enabled, typed filter and optional mask descriptor; effect records retain existing typed style data. ICC/Exif assets contain original opaque bytes, with an empty asset when absent. Pixels use the existing RGBA8/sRGB working interpretation; storing a profile does not apply it.

Asset records follow `asset_lengths` order, zero-based. Shared Arc backing buffers have one asset, preserving tile sharing on load. Zlib checksums, exact decoded lengths, consumed compressed lengths and trailing-file rejection detect malformed/truncated data. The reader preflights references, dimensions and counts before decompression, then validates the reconstructed model. Unknown versions or enum variants fail explicitly. Limits: 4 MiB manifest, 65536 assets, 512 MiB summed unique decoded bytes, 1 GiB file, 4 MiB each ICC/Exif field, existing 64-megapixel plane and model count limits. These are format/allocation policies rather than a total process-memory cap.

Saving streams assets directly to the same-directory temporary file, backfilling each compressed length with seeks. It does not serialize pixels to JSON or hold every compressed asset in memory. Metadata descriptors remain a small bounded JSON buffer. Existing v1 projects are read into the current model without rewriting the input; an explicit successful Save writes v4 atomically. Version 2 uses the same binary layout without `background`; its reader defaults this field to transparent. Version 3 introduced backing color. Version 4 additionally stores optional `selection_base`, a canvas-sized coverage asset containing the original contour before feathering. Existing versions default it to absent. Older builds reject v4 at version dispatch, preventing silent loss of the retained original during re-save. No future-format placeholders or silent downgrade are used.

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

The shape and polygon/curve tools commit raster pixels and selection coverage; a native project preserves those pixels and coverage, not a persistent editable path. Selection transforms and arbitrary rotation preserve current layer processing properties without adding new format structures. Whole-layer movement changes the existing signed layer offset and preserves source, masks, filters and effects. The underlying selection/layer model remains compatible with the v1 migration reader; the current writer stores it in v4. A translated layer with a mask and live filter has a real `.palette` round-trip test. These geometry changes do not establish `.paint` interoperability. AVIF support is independently provided by the codec path above.

## Still AVIF scope

Export settings: RGB quality 1–100 (default 80), speed 1–10 (default 8), 8-bit output, maximum alpha quality, up to four encoder threads. Opening verifies coded sequence dimensions (maximum 16384 per axis and 64 megapixels), one sequence header per primary/alpha item and the existing 256 MiB input bound. Decoder output is converted to RGBA8; 8/10-bit inputs are tested, while the implemented 12-bit downconversion still needs an external fixture. Non-still/animation streams and HEIF irot/imir/clap properties are rejected explicitly because this decoder adapter does not apply them. HDR/profile-aware conversion and broader derived-image conformance remain open. RGB is lossy; use native project saving or PNG for lossless pixels.

## Recent files and recovery

State directory: `%APPDATA%/Palette` on Windows, `~/Library/Application Support/Palette` on macOS, `$XDG_STATE_HOME/palette` or `~/.local/state/palette` on Linux. `recent.json` atomically stores at most ten successfully opened/imported/saved source paths (flattened exports are not added). Missing recent paths are disabled. Multiple instances have last-writer-wins recent-file state.

`recovery/` holds independent `.palette` snapshots and JSON sidecars named from a unique instance ID and tab ID. A per-instance `.lock` file uses an OS file lock; another live instance's snapshots are skipped. Each sidecar holds document name, original native path when any, and a validated sibling project filename. No path traversal/archive extraction occurs. Metadata reads/writes are bounded to 64 KiB. A damaged sidecar reports a recovery-list error; the sibling native project can still be opened manually. Old lease files remain harmless evidence of ownership.

Stable committed changes are captured about every 30 seconds, one job at a time. Long active strokes/drafts defer snapshots. Save/Export dirty state and original files are untouched by autosave. Startup Restore creates a dirty tab; Discard removes that snapshot; Later retains it and enables File > Recover unsaved documents. Restored originals remain until an explicit save or replacement recovery succeeds. Normal close/discard and confirmed clean saves remove owned recovery files; a late result checks whether the document has closed or become clean. OS close waits for a recovery write before resuming save/close handling. Recovery does not include undo history, viewports, uncommitted text/construction drafts, or guaranteed capture of the final interval before a crash. File sync/atomic replacement is used, with the existing filesystem/power-loss durability limits.

## Phase 4 preservation

All 19 blend modes serialize as named enum variants in the layer descriptors. Older builds reject unfamiliar variants explicitly. Wand/rectangle results, fractional selection coverage and local layer masks are stored as ordinary coverage assets; no flattening is required. Version 4 also preserves original feather contours; pre-v4 files migrate without this optional field. Selection-tool options are per-tab UI state, not file data. Filters/styles and source RGB/alpha stay intact when masks are created or edited. Export to `.paint` continues to require native preservation before baking unsupported blending/masks.
