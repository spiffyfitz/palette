# Dependencies and licenses

Direct versions were resolved using Cargo; the lockfile is authoritative. `dependency-licenses.json` is a Cargo metadata snapshot covering all resolved packages, including conditional dependencies for other platforms. Actual linkage depends on target and feature selection. This is an inventory, not a distribution-notice generator or legal conclusion.

| Dependency | Purpose | Declared license |
|---|---|---|
| eframe/egui 0.37 | Native interface, winit integration, accessibility | MIT OR Apache-2.0 |
| wgpu 30 (through eframe) | Native graphics presentation and cached-layer compute compositor | MIT OR Apache-2.0 |
| pollster 1.0.1 | Resolve GPU pipeline validation during initialization | MIT OR Apache-2.0 |
| image 0.25.10 | PNG/JPEG/BMP/GIF/TIFF/WebP codecs | MIT OR Apache-2.0 |
| rfd 0.17.2 | Native open/save dialogs | MIT |
| arboard 3.7.0 | System image clipboard, with Wayland data-control support | MIT OR Apache-2.0 |
| serde / serde_json | Versioned native document serialization | MIT OR Apache-2.0 |
| thiserror | Structured file errors | MIT OR Apache-2.0 |
| flate2 | Zlib project compression | MIT OR Apache-2.0 |
| tempfile | Same-directory atomic file replacement | MIT OR Apache-2.0 |
| fontdue 0.9.4 | GUI-independent text layout and grayscale rasterization | MIT OR Apache-2.0 OR Zlib |
| epaint_default_fonts 0.37 | Reuse bundled Ubuntu Light/Hack font bytes | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 |

Native platform requirements are in BUILD.md. Neural Filters, all bundled models, Tract, direct sha2/anyhow inference dependencies, AVIF codecs and the vendored AVIF wrapper patch have been removed. NASM and model inference toolchains are not required. The remaining graph uses native GUI/GPU/clipboard integration, Rust image codecs, font rasterization and Rust zlib compression. No libheif or third-party filter plugins are bundled. See [the dependency audit](DEPENDENCY_AUDIT.md).

Text uses the existing egui font bundle, with no platform font service or new native-library requirement. Fontdue's [layout](https://docs.rs/fontdue/0.9.4/fontdue/layout/) and [rasterization API](https://docs.rs/fontdue/0.9.4/fontdue/struct.Font.html) were checked before implementation. Font-specific notices are retained in [Ubuntu license](font-licenses/Ubuntu-UFL.txt) and [Hack notices](font-licenses/Hack-Regular.txt); the fonts remain under their own licenses. The default-font bundle also contains other egui UI fonts whose complete distribution notices still need to accompany packaged releases.

Regenerate the inventory with PowerShell:

```powershell
$metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
$metadata.packages | Sort-Object name,version | Select-Object name,version,license,repository | ConvertTo-Json -Depth 3 | Set-Content docs/dependency-licenses.json
```

Before shipping binaries, generate the actual target's third-party notices and include bundled-font and transitive dependency license texts. Check the lockfile against current vulnerability advisories. Those release tasks remain pending.


The dedicated `.paint` subset adds no native codec or new Cargo dependency: its independent safe-Rust container handling uses existing flate2. Windows close dialogs load installed Segoe UI/Segoe UI Semibold at runtime, falling back to bundled egui families elsewhere; no Microsoft fonts or icons are redistributed. A temporary validation-only Pillow-HEIF 1.8.1 wheel (libheif 1.23.6) was unpacked under ignored target/paint-validation; it is not an application dependency. That build lacks WITH_UNCOMPRESSED_CODEC and rejects both original and generated unci files, so it supplies no independent decode acceptance result.

Phase 4 adds no Cargo/native dependencies. Blend equations were independently implemented using [W3C Compositing Level 1](https://www.w3.org/TR/compositing-1/#blending). Wand uses the published Oklab/sRGB matrices from [Björn Ottosson’s Oklab article](https://bottosson.github.io/posts/oklab/), whose conversion example is offered as public-domain/MIT. No proprietary Photoshop code is used. Numeric vector/alpha tests and a generated reference sheet accompany the implementation.
