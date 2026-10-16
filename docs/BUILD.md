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

Supported scenes: `file-menu`, `brushes`, `colors`, `text`, `properties`, `layers`, `shapes`, `selection`, `catalog`, `rotate-custom`, `selected-shape`, `polygon-preview`, `curve-preview`, `selection-resize-preview`, `selection-rotate-preview`, `avif-export`, `recovery`, `width`. Scene selection requires `--screenshot` and does not add editor controls. The regression suite covers the default window plus seventeen scenes against `tests/fixtures/*-windows.png`, all at 1386 x 963. `--screenshot-size` accepts 780-3840 by 520-2160 logical points. Review changed images before replacing baselines. Capture scenes are developer diagnostics, not file-format or native-input acceptance tests.

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
