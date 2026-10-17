# Microsoft `.paint` specimen and interoperability

> Follow-up: the user confirmed the generated `.paint` opens and looks correct in Microsoft Paint. This is manual opening/appearance evidence; native layer edits and saving back were not reported. Development has advanced to [Phase 4](PHASE4_VALIDATION.md). Earlier connection failures below describe the agent’s previous run.


Inspected locally on 2026-10-16. This is a verified subset, not a complete Microsoft file-format specification. Palette imports and exports genuine HEIF projects; actual Microsoft Paint acceptance remains untested.

## Supplied specimen

Original `Untitled.paint` remains unchanged. A byte-identical test copy is `crates/palette-core/tests/fixtures/microsoft-paint.paint`.

- Size: 9854 bytes.
- SHA-256: `B9B8547C9DEFB4B86182CF1BF2FC96F16D6E3ABE2E220617FB5F3EC73D3A1BC9`.
- `ftyp`: primary `mif1`, minor 0, compatible `mif1`, `gcmi`, `isoa`, `miaf`.
- Not ZIP; top-level `ftyp` @0/32 bytes, `meta` @32/521, `free` @553/64, `mdat` @617/9237.
- `pitm` selects item 3, type `iovl`. Its `dimg` references are `[1, 2]`, bottom to top. Both inputs are type `unci`, 819 x 594, each 1,945,944 decoded bytes. No separate alpha auxiliary is used.
- Item 1: file extent @625/3990 bytes. Item 2: @4615/5181. `cmpC` declares `defl`, compression unit 0: whole-item **raw Deflate**, not zlib-wrapped data.
- `cmpd`: R/G/B/A component types 4/5/6/7. `uncC` version 0, profile `gene`, component indices 2/1/0/3, unsigned 8-bit, pixel interleave, no row/tile padding. Payload is **BGRA**; it is explicitly converted to straight RGBA. `pixi` confirms four 8-bit channels.
- `colr`: nclx primaries 1, transfer 13, matrix 0, full range: sRGB RGB. There is no ICC item/profile in this specimen.
- Overlay item 3 uses construction method 1: the 22-byte `idat` payload at @531. Version 0, flags 0, white opaque 16-bit fill `(65535,65535,65535,65535)`, output 819 x 594, two signed offsets `(0,0)`.
- Item 4: hidden `Exif`, @9796/58 bytes, `cdsc` references primary item 3. Four-byte offset 0 followed by 54-byte little-endian TIFF; X/Y resolution entries are DOUBLE 96. Palette retains the original TIFF bytes; its property UI does not yet expose print resolution.

Exactly two raster content layers are created. The overlay's white backing is `Document::background`, not a fabricated third raster layer. The layer names are empty in Paint's specimen; Palette supplies Layer 1/Layer 2. The sample supplies no proven visibility/lock/naming extension. Do not infer preservation of those Microsoft-specific properties.

| Source | Transparent pixels | Partial alpha | Opaque pixels | Content |
|---|---:|---:|---:|---|
| Item 1, bottom | 338184 | 0 | 148302 | Red rectangle, RGBA (237,28,36,255) |
| Item 2, top | 454052 | 1486 | 30948 | Black smiley with antialiased edges |

Independent raw-Deflate/BGRA decoding and Pillow alpha-compositing over white produces an RGBA SHA-256 of `0bb07de787c44f5b48383408546ea42e979274b6d37dfdfbae7b46daba22e513`. Channel totals are `(113866085,89472810,90406422,124053930)`. Palette's exported PNG decoded to exactly those same pixels. The independent script is `scripts/verify_paint_reference.py`; Pillow is a validation-only dependency.

## libheif evaluation and implementation choice

Upstream [libheif](https://github.com/strukturag/libheif) has uncompressed support behind `WITH_UNCOMPRESSED_CODEC`, with zlib and optional Brotli. Its [security policy](https://github.com/strukturag/libheif/security) limits fixes to the latest release; the current release checked was [1.23.6](https://github.com/strukturag/libheif/releases/tag/v1.23.6). Recent overlay offset/clipping/reference fixes make stale system packages unsuitable.

The [libheif-rs 3.0 context API](https://docs.rs/libheif-rs/3.0.0/libheif_rs/struct.HeifContext.html) does not expose safe overlay creation. [libheif-sys](https://docs.rs/libheif-sys/latest/libheif_sys/) exposes `heif_context_add_overlay_image`. Reusing that path would require an isolated unsafe wrapper or upstream binding work, C++ build/distribution setup, codecs and LGPL review. Writing a context read from an existing file is also rejected by recent libheif; output must be rebuilt.

For an independent experiment, the official PyPI Pillow-HEIF 1.8.1 cp312/win64 wheel was checksum-verified and unpacked under ignored `target/paint-validation`. It reports libheif 1.23.6, but rejects both specimen and generated output with `Image item of type 'unci' is not supported`: this build lacks uncompressed codec support. It is not a successful independent decoder test. Wheel SHA-256: `ce0ff957ad901a5a6bf8cd22ea26c4304bab7cf2f93d0a2f03046487e5711910`. Nothing was installed globally or linked into Palette.

The selected implementation is a small independent safe-Rust adapter for the **observed packed RGBA subset**, reusing existing flate2. It separates box/location parsing, document interpretation and output graph construction. This avoids claiming generic HEIF support and adds no native runtime dependency. General libheif integration remains an option for broader codecs/layouts after a patched WITH_UNCOMPRESSED_CODEC build and safe overlay wrapper are validated.

## Local validation and limits

`cargo test --workspace --locked` includes fixture-derived dimensions/order/pixels/alpha checks; source-pixel/composite round trips; partial-alpha and hidden RGB; different layer sizes; negative/positive and wide signed offsets; Exif/ICC retention; unknown optional versus essential properties; malformed box/extents/dimensions/reference cycles/compression; box-count and exact Deflate stream checks; unsupported-state target preservation and explicit compatibility baking. Native v3 saves preserve the white canvas backing; v1/v2 migrate without it and retain their original transparent backing.

`cargo run -p palette-core --example paint_roundtrip --locked -- Untitled.paint target/paint-roundtrip.paint` generates an inspectable `.paint`, `.png` and complete `.palette`. It verifies both source pixel buffers, order, offsets and composite on reopen. This local round trip is necessary but cannot establish acceptance by Microsoft's application.

Supported layout and budgets are specified in [FILE_FORMATS.md](FILE_FORMATS.md). Unknown nonessential boxes/properties are tolerated, not retained on rewrite. Other codecs, tiled/planar/high-depth pixels, encrypted/external data, transformed/nested derived inputs, mixed color profiles and unknown essential properties fail explicitly. Names/locks/visibility beyond the verified subset are not asserted compatible. Profile bytes are retained, not color-managed.

## Remaining native acceptance gate

Windows Computer Use was tried through the installed computer-use skill, then retried/reinitialized as instructed. It reports `Computer Use native pipe is unavailable ... os error 2`. No native Paint interaction was possible. Seeded Palette GPU captures and egui input tests do not substitute for it.

Once that connection is restored (or by manual review in Paint):

1. Open the unchanged original in Paint and record its version and two-layer appearance.
2. Open `target/paint-roundtrip.paint`; verify 819 x 594, **two independently editable layers**, red below black smiley, transparent source regions and white canvas backing.
3. Toggle/edit each layer separately, save as a new Paint project, and reopen that file in Palette. Compare source RGBA/order/offsets and composite against the saved result.
4. Repeat with partial alpha and nonzero source offsets; test whether Paint honors exported names. Retain exported fixtures and exact application/version results.
5. Validate other Microsoft Paint variants and a current libheif build with uncompressed support before broadening compatibility claims.

Until those checks succeed, `.paint` remains labeled experimental and Phase 3's full Workflow G gate remains open. Windows 10/macOS/Linux codec runs and broader AVIF/color/metadata conformance are separate outstanding validation work.
