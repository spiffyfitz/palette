# Development and packaging

## Shared workflow

Use Rust stable, version 1.98 or later. Open the root Cargo.toml in RustRover. The repository has no generated UI files and does not need Node, a browser engine or a web server. Cargo.lock is committed.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo run --locked
cargo build --release --locked
```

The executable is `target/debug/palette.exe` on Windows or `target/debug/palette` elsewhere. Release artifacts use `target/release`. The Windows release build uses the GUI subsystem, avoiding a console window. `cargo run -- path/to/image.png` opens a file; multiple file arguments are accepted, up to the concurrent file-operation limit.

## Windows

Install Visual Studio Build Tools with Desktop development with C++, the MSVC toolchain and Windows SDK; install the Rust MSVC target. The UI runs through wgpu's available native backend. A modern graphics driver is required. AVIF uses Rust codec libraries without a runtime codec DLL, but rav1d assembly requires NASM (2.14+) on PATH for x86/x64 builds. Install NASM from https://www.nasm.us or `choco install nasm`, then restart RustRover so its Cargo environment sees PATH. This environment used NASM 3.01 from the official win64 ZIP, isolated under ignored `target/build-tools/nasm-3.01` (SHA-256 E0BA5157007ABC7B1A65118A96657A961DDF55F7E3F632EE035366DFCE039CA4). `./scripts/dev.ps1 build --locked` or `./scripts/dev.ps1 run --locked` uses that existing local tool if present; it does not install or download anything.

Windows smoke capture:

```powershell
cargo build --locked
.\target\debug\palette.exe --screenshot target/window.png
cargo test --test screenshot -- --ignored --test-threads=1
```

Run this from an interactive desktop. Native dialogs, GPU presentation and screenshot behavior cannot be established by a headless core test. Run screenshot tests serially to avoid multiple capture windows interfering.

Debug capture scenes select actual interface states for repeatable visual review:

```powershell
.\target\debug\palette.exe --screenshot target/colors.png --screenshot-scene colors
.\target\debug\palette.exe --screenshot target/colors-small.png --screenshot-scene colors --screenshot-size 780x520
```

Supported scenes: `file-menu`, `brushes`, `colors`, `text`, `properties`, `layers`, `shapes`, `selection`, `catalog`, `rotate-custom`, `selected-shape`, `polygon-preview`, `curve-preview`, `selection-resize-preview`, `selection-rotate-preview`, `avif-export`, `recovery`, `close-dialog`, `paint-project`, `paint-export`, `selection-options`, `wand-settings`, `mask-layer`, `feather-selection`, `width`. Scene selection requires `--screenshot` and does not add editor controls. The regression suite now covers the default window plus 151 scenes (see tests/screenshot.rs for the complete list) against `tests/fixtures/*-windows.png`, all at 1386 x 963. `--screenshot-size` accepts 780-3840 by 520-2160 logical points. Review changed images before replacing baselines. Capture scenes are developer diagnostics, not file-format or native-input acceptance tests.

If Windows reports that `palette.exe` is locked by a running session, leave that session open and build/test separately with `--target-dir target/transform-milestone`. This milestone used that fallback initially; the ordinary target build also succeeded after the lock cleared.

## macOS

Install Xcode Command Line Tools (`xcode-select --install`) and Rust for the host architecture. Install `nasm` with Homebrew for Intel codec builds; the Apple Silicon decoder uses its Rust fallback. Build natively on Apple Silicon and Intel. A future universal distribution can combine independently built `aarch64-apple-darwin` and `x86_64-apple-darwin` binaries with `lipo`; this has not been performed here. No platform-specific model code is needed.

The eventual `.app` needs a bundle identifier, icon, Info.plist, document associations, signing and notarization. This milestone only supplies a native Cargo executable; it does not claim a signed distributable.

## Linux

For Ubuntu/Debian, the intended development environment is:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libx11-dev libxkbcommon-dev libwayland-dev libdbus-1-dev nasm libvulkan1 mesa-vulkan-drivers xdg-desktop-portal xdg-desktop-portal-gtk zenity
cargo run --locked
```

Both eframe X11 and Wayland features are enabled. Native file dialogs use rfd's default XDG portal backend. Install an appropriate GTK/GNOME/KDE portal for the desktop; wlroots alone may not provide the file-picker interface. See [rfd's runtime requirements](https://docs.rs/rfd/0.17.2/rfd/). Verify the app separately in native Wayland and X11 sessions. These commands are documented setup, not a locally tested Linux result.

Future packages should bundle required license notices, declare portal/Zenity/runtime graphics requirements and test a clean installation. AppImage/deb/rpm artifacts are not produced yet.

## CI

`.github/workflows/ci.yml` checks formatting, Clippy, unit/integration tests and builds on Windows, Ubuntu, Apple Silicon macOS and Intel macOS. Runner labels were checked against [GitHub's runner documentation](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). It has not been executed remotely in this session. The desktop screenshot test is explicitly ignored in normal CI; run it on a controlled interactive Windows desktop.

The CI matrix installs NASM and executes the same AVIF/native/recovery tests on each target. This configuration has not been run remotely here; local validation is Windows only. See [roadmap gates](../ROADMAP.md).

Current Phase 5 desktop suite covers 56 states, including the six additional processing editors and Pixelate/Other menus; see [style/filter validation](STYLE_FILTER_CONTROLS_VALIDATION.md). Cold CPU diagnostics for this batch: `cargo run -p palette-core --example processing_extended_benchmark --locked`.

The conventional-menu follow-up expands Windows desktop regression to 66 states. See [menu/filter validation](FILTER_MENU_VALIDATION.md); the extended CPU benchmark also includes the ten new operations.

The creative-filter batch expands desktop regression to 75 states. `processing_extended_benchmark` includes the new filters; expensive Radial Blur is measured at 1280 × 720 only. Exact scope is in [creative filter validation](CREATIVE_FILTERS_VALIDATION.md).

Noise/distortion follow-up: Windows desktop regression covers 83 states. The extended processing benchmark includes the new operations at 1280 × 720 and 4K; see [retouch/distortion validation](RETOUCH_DISTORT_VALIDATION.md).

Movable-origin follow-up: native Windows regression includes 88 states, plus compact-window visual review. The benchmark includes Polar Coordinates, ZigZag, Fibers and Glowing Edges. [Validation](ORIGIN_FILTERS_VALIDATION.md).

Artistic/print/texture follow-up: Windows native regression covers 98 states, including the first Brush Strokes/Sketch menus, plus compact editor review. The extended benchmark measures the eight new filters at 1280×720 and 4K. [Evidence and algorithm limits](ARTISTIC_FILTERS_VALIDATION.md).

Pixelate/cell follow-up: Windows native regression covers 106 states; eight new previews and three changed menus were reviewed, plus compact Color Halftone/Mezzotint. Benchmark and scope: [validation](PIXELATE_FILTERS_VALIDATION.md). Recount the catalog with `cargo run -p palette-core --example filter_inventory --locked`; its stdout generates [FILTER_INVENTORY.md](FILTER_INVENTORY.md).

Adjustment follow-up: 114 native Windows states include eight new previews and updated Adjustments/Blur menus. Compact Channel Mixer/Selective Color controls were also reviewed. Run `cargo run -p palette-core --example processing_extended_benchmark --locked -- --adjustments` for this batch's cold CPU composites. See [validation](ADJUSTMENT_FILTERS_VALIDATION.md). Formatting, strict all-target Clippy, ordinary tests, executable build and native screenshot regression are checked on Windows; other platforms remain external gates.

Traditional-media follow-up: native Windows regression covers 122 states, including eight new filter previews and changed Artistic/Brush Strokes menus. Colored Pencil/Angled Strokes were also reviewed at 780×520. Measure this batch with `cargo run -p palette-core --example processing_extended_benchmark --locked -- --painting`. [Validation](PAINTING_FILTERS_VALIDATION.md) records 259 ordinary tests and the current CPU/preview limits. No additional native dependencies were introduced.

Drawing follow-up: native Windows regression covers 130 states, including eight new filter previews and updated Brush Strokes/Sketch menus. Graphic Pen/Chalk & Charcoal were also reviewed at 780×520. Measure this batch with `cargo run -p palette-core --example processing_extended_benchmark --locked -- --drawing`. [Validation](DRAWING_FILTERS_VALIDATION.md) records 269 ordinary tests and CPU/preview limits. No additional native dependencies.

Sketch completion: native Windows regression covers 136 states, including six new previews and the completed Sketch menu. Conté Crayon/Water Paper were also reviewed at 780×520. Measure this batch with `cargo run -p palette-core --example processing_extended_benchmark --locked -- --sketch`. [Validation](SKETCH_COMPLETION_VALIDATION.md) records 278 ordinary tests and current limits. No new native libraries or texture assets are required.

Artistic completion: native Windows regression covers 144 states, including eight new previews and the completed Artistic menu. Neon Glow/Rough Pastels were also reviewed at 780×520. Measure this batch with `cargo run -p palette-core --example processing_extended_benchmark --locked -- --artistic`. [Validation](ARTISTIC_COMPLETION_VALIDATION.md) records 288 ordinary tests, CPU timings and preview limits. No new dependencies or proprietary texture assets.

Optical/texture follow-up: Windows native regression covers 152 states, including eight new previews and updated Blur/Distort/Sharpen/Texture menus. Glass/Smart Blur were also reviewed at 780×520. Run `cargo run -p palette-core --example processing_extended_benchmark --locked -- --optical` for isolated cold CPU measurements. [Validation](OPTICAL_FILTERS_VALIDATION.md) records 299 ordinary tests and limits. No new dependencies or native libraries.
