# Microsoft `.paint` investigation

Status: design research only. The repository initially contained no project files and the user message attached two PNGs, not `Untitled.paint`. A narrow check of attached-file locations found no `.paint` specimen. No claim about this particular file has been independently verified.

## Evidence and dependency choice

The user's preliminary observations are a hypothesis to test: ISO BMFF/HEIF, `mif1` with `gcmi`/`isoa`/`miaf`, two 819 × 594 RGBA `unci` items, an `iovl` primary composite and Exif metadata. Do not infer layers simply from all top-level image IDs; derived inputs, alpha auxiliaries, thumbnails and metadata have different relationships.

Upstream [libheif](https://github.com/strukturag/libheif) supports uncompressed image coding via `WITH_UNCOMPRESSED_CODEC`, with zlib and optional Brotli. This is a credible decoding path for the proposed specimen, subject to the actual item properties. Its [security policy](https://github.com/strukturag/libheif/security) says only the latest release receives fixes. The release page resolved to [v1.23.6](https://github.com/strukturag/libheif/releases/tag/v1.23.6) during research; recheck before integration. Recent releases fix overlay offset/clipping/reference problems, so old system packages are not a sound default.

The [libheif-rs 3.0 HeifContext API](https://docs.rs/libheif-rs/3.0.0/libheif_rs/struct.HeifContext.html) provides image handles, encoding, metadata, security limits and writing. Its public method list does not expose an overlay constructor. The lower-level [libheif-sys API](https://docs.rs/libheif-sys/latest/libheif_sys/) exposes `heif_context_add_overlay_image`. This establishes an API candidate, not Paint compatibility. Prefer an upstream safe-wrapper contribution or a small isolated, audited adapter around that function; do not spread unsafe calls through the model/UI.

The [release history](https://github.com/strukturag/libheif/releases) also documents that writing a context read directly from a file is rejected in recent releases. A writer must construct a new encoding context and explicitly preserve supported metadata/properties; reading then blindly writing the same context is not a valid round-trip strategy.

Native integration would add a C++20 toolchain, current libheif, zlib and any enabled codec dependencies. libheif is LGPL-3.0-or-later; Rust binding and codec licensing must be reviewed alongside the chosen linking/distribution method. None is linked into this milestone, keeping Phase 1 portable without an unvalidated native dependency.

## Concrete next experiment

1. Obtain `Untitled.paint`; record hash, byte size and native Paint version. Preserve it unchanged as a test fixture.
2. Dump validated ISO BMFF box boundaries and brands. Inspect `meta`, `pitm`, `iinf/infe`, `iloc`, `iref`, `iprp/ipco/ipma` and item properties. Unknown nonessential properties must be retained or tolerated; unknown essential properties require a precise error.
3. Resolve the primary `iovl` and its `dimg` references in order. Record output bounds, overlay background, signed offsets, source item dimensions, alpha auxiliaries and color properties. Traverse with cycle/depth/item-count limits.
4. Decode each referenced content item separately through current libheif with bounded security limits. Inspect uncompressed layout/component definitions and `cmpC`/compressed-unit properties when present. Do not assume interleaved RGBA, 8-bit channels or whole-item Deflate.
5. Verify the two reported source layers, their transparent regions, red-below-smiley order and 819 × 594 composite. Compare both source pixel probes and the composite against Paint's exported PNG/native preview.
6. Construct a fresh context, encode supported RGBA layers, call the overlay constructor with verified order/offset/background, set primary image and attach required metadata/brands. Save to a new temporary `.paint` target.
7. Reopen in Palette, inspect bytes and compare source layers/composite. Open the generated file in actual Microsoft Paint; verify editable layers, not merely a successful flattened preview. Save from Paint again and compare relationships/pixels.
8. Add offset, partial-alpha, hidden-layer and malformed-input fixtures; fuzz container boundaries and decompression limits. Only then expose `.paint` in open/save dialogs.

Layer naming, visibility, lock state, ICC/Exif preservation and unknown Microsoft-specific metadata are open questions. HEIF overlay capability alone does not prove they round-trip. The acceptance workflow remains blocked on the specimen and actual Microsoft Paint testing.
