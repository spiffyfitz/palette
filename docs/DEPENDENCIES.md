# Dependencies and licenses

Direct versions were resolved using Cargo; the lockfile is authoritative. `dependency-licenses.json` is a Cargo metadata snapshot covering all resolved packages, including conditional dependencies for other platforms. Actual linkage depends on target and feature selection. This is an inventory, not a distribution-notice generator or legal conclusion.

| Dependency | Purpose | Declared license |
|---|---|---|
| eframe/egui 0.37 | Native interface, winit integration, accessibility | MIT OR Apache-2.0 |
| wgpu 30 (through eframe) | Native graphics presentation | MIT OR Apache-2.0 |
| image 0.25.10 | PNG/JPEG/BMP/GIF/TIFF/WebP codecs | MIT OR Apache-2.0 |
| rfd 0.17.2 | Native open/save dialogs | MIT |
| arboard 3.7.0 | System image clipboard, with Wayland data-control support | MIT OR Apache-2.0 |
| serde / serde_json | Versioned native document serialization | MIT OR Apache-2.0 |
| thiserror | Structured file errors | MIT OR Apache-2.0 |
| flate2 | Zlib project compression | MIT OR Apache-2.0 |
| tempfile | Same-directory atomic file replacement | MIT OR Apache-2.0 |
| tract-onnx 0.23.9 (default features disabled) / tract-linalg 0.23.9 (multithread-mm) | Bundled offline CPU model inference in Rust | MIT OR Apache-2.0 |
| sha2 0.10.9 / anyhow | Exact model identity and inference error context | MIT OR Apache-2.0 |
| avif-decode 3.0.0 / rav1d 1.1.0 | Real AVIF decoding / AV1 codec | BSD-3-Clause / BSD-2-Clause |
| ravif 0.13 / rav1e 0.8 | AVIF encoding | BSD-3-Clause / BSD-2-Clause |
| avif-parse 2.1 | AVIF container and AV1 metadata validation | MPL-2.0 |
| fontdue 0.9.4 | GUI-independent text layout and grayscale rasterization | MIT OR Apache-2.0 OR Zlib |
| epaint_default_fonts 0.37 | Reuse bundled Ubuntu Light/Hack font bytes | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 |

Native runtime requirements are in BUILD.md. AVIF codecs are compiled into the application; x86/x64 builds require the NASM assembler. No libheif, proprietary Microsoft resources, Adobe code or third-party filter plugins are bundled. Palette icons are original Rust geometry. A future HEIF integration requires a separate LGPL/codec dependency review and license notices for its chosen distribution.

Text uses the existing egui font bundle, with no platform font service or new native-library requirement. Fontdue's [layout](https://docs.rs/fontdue/0.9.4/fontdue/layout/) and [rasterization API](https://docs.rs/fontdue/0.9.4/fontdue/struct.Font.html) were checked before implementation. Font-specific notices are retained in [Ubuntu license](font-licenses/Ubuntu-UFL.txt) and [Hack notices](font-licenses/Hack-Regular.txt); the fonts remain under their own licenses. The default-font bundle also contains other egui UI fonts whose complete distribution notices still need to accompany packaged releases.

Regenerate the inventory with PowerShell:

```powershell
$metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
$metadata.packages | Sort-Object name,version | Select-Object name,version,license,repository | ConvertTo-Json -Depth 3 | Set-Content docs/dependency-licenses.json
```

Before shipping binaries, generate the actual target's third-party notices and include bundled-font and transitive dependency license texts. Check the lockfile against current vulnerability advisories. Those release tasks remain pending.

AVIF APIs and licenses were checked against [avif-decode 3.0](https://docs.rs/avif-decode/3.0.0/avif_decode/), [ravif 0.13](https://docs.rs/ravif/0.13.0/ravif/), [avif-parse 2.1](https://docs.rs/avif-parse/2.1.0/avif_parse/) and their downloaded Cargo manifests. No unsafe Rust was added to Palette; the decoder uses upstream Rust/assembly code. Keep BSD notices and MPL source/license obligations with distributions. These are not a replacement for the pending packaged-release notices audit.

The dedicated `.paint` subset adds no native codec or new Cargo dependency: its independent safe-Rust container handling uses existing flate2. Windows close dialogs load installed Segoe UI/Segoe UI Semibold at runtime, falling back to bundled egui families elsewhere; no Microsoft fonts or icons are redistributed. A temporary validation-only Pillow-HEIF 1.8.1 wheel (libheif 1.23.6) was unpacked under ignored target/paint-validation; it is not an application dependency. That build lacks WITH_UNCOMPRESSED_CODEC and rejects both original and generated unci files, so it supplies no independent decode acceptance result.

Phase 4 adds no Cargo/native dependencies. Blend equations were independently implemented using [W3C Compositing Level 1](https://www.w3.org/TR/compositing-1/#blending). Wand uses the published Oklab/sRGB matrices from [Björn Ottosson’s Oklab article](https://bottosson.github.io/posts/oklab/), whose conversion example is offered as public-domain/MIT. No proprietary Photoshop code is used. Numeric vector/alpha tests and a generated reference sheet accompany the implementation.

`palette-neural` now uses Tract 0.23.9 (transformer features disabled), sha2 0.10.9 and anyhow in the native application. No additional native inference library, Python installation or hosted service is required. The 6.7 MB Mosaic network is bundled under its upstream declared BSD-3-Clause terms; [provenance](../crates/palette-neural/models/README.md) and [full notice](../crates/palette-neural/models/LICENSE-BSD-3-Clause.txt) accompany it, and the notice is available in the filter information popup. Tract and model dependencies remain subject to the packaged-release transitive-notice task above.

`tools/neural-probe` remains an isolated development workspace. `tools/neural-reference.py` uses ONNX Runtime 1.23.2 solely to generate the checked-in independent numerical fixture from the original graph. This reference engine is not linked, launched or installed by Palette, and cargo tests need neither Python nor network access.

The JPEG restoration batch bundles a 2.67 MB ONNX export of the author’s JPEG-trained DnCNN-3 weights, retaining the KAIR MIT notice in `crates/palette-neural/models/LICENSE-KAIR-MIT.txt` and in-app model information. [Provenance and exact hashes](JPEG_RESTORATION_VALIDATION.md). Tract-linalg enables `multithread-mm` for a shared pool of at most four CPU threads. PyTorch 2.9.0+cpu and ONNX 1.19.1 are local development/reference tools only, not linked or launched by Palette. All application inference remains inside the native executable.


Colorize bundles a 64.5 MB compact export of the ECCV16 author-hosted weights,
under the retained BSD-2-Clause author notice in
`crates/palette-neural/models/LICENSE-Colorize-BSD-2-Clause.txt`. Large kernels
use fp16 storage with float32 computation. `palette-neural` now declares the
combined SPDX expression `MIT AND BSD-2-Clause AND BSD-3-Clause` to reflect its
code and bundled model licenses. [Exact provenance and reference evidence](COLORIZE_VALIDATION.md).
A public-domain NASA photograph is used for tests/debug captures with credits
in the fixture README. scikit-image 0.25.2 is a local development-only Lab/photo
reference tool, not an application dependency. No new native inference runtime
or user-side download is introduced.
Face-guided Skin Smoothing embeds unmodified BlazeFace and Face Mesh ONNX ports
under Apache-2.0 (about 2.94 MB combined). `NOTICE-Face.txt` records upstream
Google/port attribution, source revision, release URLs and exact hashes;
`LICENSE-Face-Apache-2.0.txt` contains the full license. Both are available in
the application's information popup. No runtime dependency or native-library
requirement was added. The inference crate's SPDX expression includes Apache-2.0
for these assets. [Model distribution and independent evidence](SKIN_SMOOTHING_VALIDATION.md).

Cropped Super Zoom reuses Tract with a 240,078-byte unmodified ONNX Model Zoo
ESPCN graph. `NOTICE-SuperZoom.txt` records exact pinned provenance, SHA-256,
Apache-2.0 model-distribution terms and PyTorch BSD-3-Clause attribution. Full
Apache/BSD notices are already retained with the assets and shown in the app.
No Cargo dependency or system library was added. The existing inference-crate
SPDX expression covers these assets. [Contract and references](SUPER_ZOOM_VALIDATION.md).

Depth Blur embeds the official unmodified MiDaS v2.1 small ONNX artifact
(66,764,249 bytes), distributed by Intel ISL under MIT. `NOTICE-Depth.txt` retains
the exact release URL, SHA-256, paper citation and full license, available offline
in the app. The existing SPDX expression already includes MIT; no Cargo or native
runtime dependency was added. [Contract and references](DEPTH_BLUR_VALIDATION.md).

Statistical Color Transfer adds no crate, native library or pretrained model.
It reuses existing Rust Lab conversion and image codecs. The existing `serde_json`
dependency enables its `float_roundtrip` feature so native reference profiles
preserve float64 moments exactly. Independent NumPy/scikit-image/Pillow checks
run only in the development environment. [Contract and references](COLOR_TRANSFER_VALIDATION.md).
