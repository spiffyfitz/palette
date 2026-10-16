# Development and packaging

## Shared workflow

Use Rust stable, version 1.95 or later. Open the root Cargo.toml in RustRover. The repository has no generated UI files and does not need Node, a browser engine or a web server. Cargo.lock is committed.

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

Install Visual Studio Build Tools with Desktop development with C++, the MSVC toolchain and Windows SDK; install the Rust MSVC target. The UI runs through wgpu's available native backend. A modern graphics driver is required. There is no libheif/AVIF native dependency in this milestone.

Windows smoke capture:

```powershell
cargo build --locked
.\target\debug\palette.exe --screenshot target/window.png
cargo test --test screenshot -- --ignored
```

Run this from an interactive desktop. Native dialogs, GPU presentation and screenshot behavior cannot be established by a headless core test.

## macOS

Install Xcode Command Line Tools (`xcode-select --install`) and Rust for the host architecture. Build natively on Apple Silicon and Intel. A future universal distribution can combine independently built `aarch64-apple-darwin` and `x86_64-apple-darwin` binaries with `lipo`; this has not been performed here. No platform-specific model code is needed.

The eventual `.app` needs a bundle identifier, icon, Info.plist, document associations, signing and notarization. This milestone only supplies a native Cargo executable; it does not claim a signed distributable.

## Linux

For Ubuntu/Debian, the intended development environment is:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libx11-dev libxkbcommon-dev libwayland-dev libdbus-1-dev libvulkan1 mesa-vulkan-drivers xdg-desktop-portal xdg-desktop-portal-gtk zenity
cargo run --locked
```

Both eframe X11 and Wayland features are enabled. Native file dialogs use rfd's default XDG portal backend. Install an appropriate GTK/GNOME/KDE portal for the desktop; wlroots alone may not provide the file-picker interface. See [rfd's runtime requirements](https://docs.rs/rfd/0.17.2/rfd/). Verify the app separately in native Wayland and X11 sessions. These commands are documented setup, not a locally tested Linux result.

Future packages should bundle required license notices, declare portal/Zenity/runtime graphics requirements and test a clean installation. AppImage/deb/rpm artifacts are not produced yet.

## CI

`.github/workflows/ci.yml` checks formatting, Clippy, unit/integration tests and builds on Windows, Ubuntu, Apple Silicon macOS and Intel macOS. Runner labels were checked against [GitHub's runner documentation](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). It has not been executed remotely in this session. The desktop screenshot test is explicitly ignored in normal CI; run it on a controlled interactive Windows desktop.
