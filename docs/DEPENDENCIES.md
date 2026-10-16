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

Native runtime requirements are in BUILD.md. No libheif, AVIF native encoder/decoder, proprietary Microsoft resources, Adobe code or third-party filter plugins are bundled. Palette icons are original Rust geometry. A future HEIF integration requires a separate LGPL/codec dependency review and license notices for its chosen distribution.

Regenerate the inventory with PowerShell:

```powershell
$metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
$metadata.packages | Sort-Object name,version | Select-Object name,version,license,repository | ConvertTo-Json -Depth 3 | Set-Content docs/dependency-licenses.json
```

Before shipping binaries, generate the actual target's third-party notices and include bundled-font and transitive dependency license texts. Check the lockfile against current vulnerability advisories. Those release tasks remain pending.
