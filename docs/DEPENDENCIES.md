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
