# Milestone validation

Recorded locally on 2026-10-15 (America/New_York). The repository began empty; this milestone implements the Phase 0 design and a runnable Phase 1 foundation with several early Phase 2 editing slices. Phase 0's specimen analysis remains incomplete because `Untitled.paint` was not supplied.

## Environment

- Windows NT 10.0.26200.0, x86-64; native MSVC executable.
- Rust 1.99.1; Cargo 1.99.1.
- eframe/egui 0.37.0 with actual wgpu presentation.
- Default application content size: 1386 × 963; default canvas: 819 × 594 at 75%.

## Final checks

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | 14 passed; desktop screenshot test explicitly ignored in this command |
| `cargo build --workspace --locked` | Passed; runnable `target/debug/palette.exe` |
| `cargo test --test screenshot --locked -- --ignored` | Passed; actual native window launched/captured/exited |

The shell/Cargo environment emits a path-canonicalization warning for `C:\Users\spiff`; it does not prevent the checks. There are no Clippy source warnings.

The automated tests cover:

- Source-over semi-transparent edges, transparent hidden RGB, Multiply and Screen.
- Source-preserving masks and ordered filters, disabled filters and extreme signed offsets.
- Continuous soft brush coverage without gaps, source-preserving COW snapshots and destination-out erasing.
- Undo/redo pixel/state restoration, saved revisions and redo invalidation on a new branch.
- `.palette` exact model/composite round trip with two layers, mask, filter, Color Overlay style, offsets, lock, opacity and blend properties; atomic replacement of an existing file.
- Real PNG/JPEG/BMP/GIF/TIFF/WebP export/decode dimensions and exact RGBA for PNG/TIFF/WebP.
- Invalid project versions and malformed active-ID/tile invariants.
- A deliberately failed partial encoder that leaves the existing file unchanged and cleans up its temporary file.
- UI pointer painting at 75%, layer isolation, undo/redo, independent tabs and unsaved-close protection.
- Actual canvas handle resizing and undo; positive workspace bounds at the minimum window size.
- Native wgpu screenshot comparison with the visually reviewed Windows baseline.

Native file-dialog navigation, full keyboard accessibility, all gestures at multiple OS scaling factors, actual tablet pressure and broad image codec conformance have not been exhaustively manually tested.

## Visual review

Both supplied screenshots were examined. Actual Palette captures were reviewed at 1386 × 963. Original geometric icons replaced missing Unicode glyphs; brush controls, panel sizing and ribbon spacing were refined. The final reviewed baseline is [window-windows.png](../tests/fixtures/window-windows.png). The test compares a second independently launched capture against this baseline. This is a regression result, not an assertion of pixel-perfect Microsoft Paint reproduction.

## CPU timing probe

Command: `cargo run -p palette-core --example compositor_bench --locked`. This uses the dev profile (`opt-level=1`, dependencies at 2), not a release benchmark. Each document has an opaque background and a full-canvas half-transparent color layer. Results are the median of three full CPU composites on the local machine.

| Canvas | Median CPU composite |
|---|---:|
| 819 × 594 | 24.79 ms |
| 3840 × 2160 | 420.59 ms |
| 7680 × 4320 | 1749.05 ms |

These timings exclude GUI texture upload and interactive input. Blank/solid source tiles share allocations; this is not a memory benchmark for a fully painted image. No GPU-compositor, smooth 4K/8K painting or general hardware performance claim follows from these results. Full-frame rendering is the principal identified bottleneck; implement dirty-region rendering and stage caches before treating large-image editing as complete.

## Not validated

No macOS, Linux, Windows 10, minimum-Rust-version or remote CI run occurred here. The matrix is supplied for subsequent platform validation. AVIF and Microsoft `.paint` are disabled and untested. No native Paint export/open workflow or specimen-derived pixel/order/transparency verification occurred.

## Recommended next milestone

Complete Phase 2 as working vertical slices: transactional stroke coverage/history accounting, fill and rectangular coverage selections, clipboard, crop/resize/rotate/flip, then text. Validate each through real pointer/keyboard workflows under fractional zoom and transparent layers. In parallel with that future work, obtain `Untitled.paint` and execute PAINT_RESEARCH.md's libheif/overlay experiments; the Paint compatibility gate must remain closed until source-layer and native-application round trips pass.
