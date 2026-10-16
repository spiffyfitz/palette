# Known limitations

- This is a Phase 1 foundation, not complete Paint/Photoshop equivalence. Disabled controls state which later milestone they require.
- `Untitled.paint` was not available. `.paint`, HEIF/HEIC and AVIF are rejected; no Microsoft Paint interoperability is claimed.
- Only Windows in this environment has been built and launched. The CI matrix is configured, not evidence of successful macOS/Linux runs. Windows 10 is not tested here.
- The UI has been compared visually with the provided screenshot but is not pixel-identical. Native title bars vary by OS. The supplied Paint image is 1309 × 962 and displays 100%; Palette defaults to the requested 75% at 1386 × 963, with an added tab strip and third slider.
- Most editing tools, selections, text, clipboard, transforms, masks UI, additional blend modes, Gaussian Blur and layer-style dialogs remain pending. Model masks/effects must not be described as finished UI workflows.
- Invert and Brightness are live controls with undo; transactional preview Apply/Cancel and common parameter descriptors are not implemented.
- Brush opacity accumulates where consecutive segment footprints overlap. No pressure, spacing, tablet-specific input or brush variants yet. Outlined shapes share this limitation. Pencil is a one-pixel hard footprint, not a dedicated integer-line rasterizer.
- `.palette` version 1 stores tile byte arrays in compressed JSON. It preserves all currently represented state, not future unimplemented features. Both input and decompressed JSON are capped at 256 MiB. Large/many-layer projects may exceed this sooner than a streamed PNG-asset format would; saving refuses them without overwriting the destination.
- Images are limited to 64 megapixels; raster imports also limit each dimension to 16384. Images exceeding the GPU texture dimension use a reduced display texture while retaining full source/composite data; high-zoom detail on those images needs tiled display textures. This is not a production-scale 8K editor yet.
- History has a conservative 128 MiB logical-state budget per tab. Shared tiles reduce actual copying, but accounting can discard history early or entirely for very large layered documents. Improve this before claiming robust 4K/8K editing.
- The compositor reprocesses the full document after a mutation. Only one render can be queued; stale results are rejected, but running jobs are not cancelled. Rendering is CPU; wgpu currently presents the result.
- No autosave/recovery, recent-files persistence, profile management or Exif/orientation preservation yet. Imports convert to RGBA8, assumed sRGB. Preserve originals when metadata matters.
- GIF imports one frame and exports a still image with palette/transparency limitations. TIFF multi-page behavior, animated WebP and unusual metadata require explicit future notifications. JPEG export uses white for transparency. Ordinary codec tests use small fixtures; they are not a comprehensive conformance/fuzz suite.
- Native dialogs are synchronous modal dialogs; decoding/encoding/rendering run in background workers. Four concurrent file jobs are allowed; further requests report an error.
- Canvas resizing expands/crops full-canvas, unshifted layer buffers. Existing masks/selection coverage and translated layer buffers are retained at their original sizes; advanced transform/mask-boundary editing remains pending.
- Integer zoom uses nearest sampling. Fractional coordinates are correct, but smooth downsampling, comprehensive DPI and keyboard/screen-reader testing remain pending.
- Distribution installers, macOS app signing/notarization and Linux packages are not produced by this milestone.
