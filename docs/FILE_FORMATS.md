# File compatibility

## Current support

| Format | Open | Write | Meaning |
|---|---|---|---|
| `.palette` v1 | Yes | Save/Save As | All currently implemented editable model state |
| PNG | Yes | Export | Lossless RGBA; alpha round-trip tested |
| JPEG | Yes | Export | Lossy RGB; transparency composited over white |
| BMP | Yes | Export | Raster; alpha behavior depends on BMP variant |
| GIF | First frame | Still export | Indexed colors; limited transparency; no animation preservation |
| TIFF | Raster | Export | RGBA round-trip tested; multipage/metadata preservation not supported |
| WebP | Raster | Export lossless | RGBA round-trip tested; no animation preservation |
| AVIF | No | No | Disabled until actual decode/encode validation |
| Microsoft `.paint` | No | No | Specimen and Microsoft Paint validation missing |
| HEIF/HEIC | No | No | No libheif dependency linked yet |

Raster imports create a single editable source layer. They do not recreate layers absent from a flat format. Raster export writes the rendered composite; the project path and dirty state remain unchanged. Save creates a `.palette` project, even when the tab originated from PNG/JPEG. Unsupported output extensions fail before touching the target.

All supported raster operations use image 0.25.10 with only the explicitly listed codec features enabled. Tests exercise dimensions for all six formats and exact RGBA for PNG/TIFF/WebP. Images are converted to straight-alpha RGBA8 in assumed sRGB. Color profiles, orientation, Exif and animation are not preserved by this milestone.

## `.palette` version 1

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

The next format revision should separate metadata from streamed lossless raster assets, add document metadata/profiles and migrate v1 at the reader boundary. Do not change v1's meaning to add incompatible storage.

## Atomic writes

Both project saves and raster exports encode into a temporary file in the target's directory, sync it, then use tempfile's replacement operation. A codec, serialization, validation or disk-write failure before replacement leaves the old target intact. Temporary files are removed by RAII. Filesystem durability across power loss still depends on the OS/filesystem; directory fsync is not implemented.

## Microsoft Paint gate

No `.paint` file is emitted or renamed from an unrelated container. A future writer must retain editable raster layers that Paint supports and explicitly offer `.palette` preservation before baking unsupported masks, blends, styles or filters. See [research and test plan](PAINT_RESEARCH.md). The specimen's reported brands, `unci`, `iovl`, dimensions and ordering are unverified until its bytes are available.

## Shape and layer movement preservation

The shape and polygon/curve tools commit raster pixels and selection coverage; a native project preserves those pixels and coverage, not a persistent editable path. Selection transforms and arbitrary rotation preserve current layer processing properties without adding new format structures. Whole-layer movement changes the existing signed layer offset and preserves source, masks, filters and effects. This uses the existing version 1 model, so no file-version change is required. A translated layer with a mask and live filter has a real `.palette` round-trip test. These changes do not enable `.paint` or AVIF support.
