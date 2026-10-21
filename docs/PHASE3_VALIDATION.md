# Phase 3: file compatibility and document management

Historical build evidence below used NASM. Current builds disable codec assembly
and require no NASM; see the [dependency audit](DEPENDENCY_AUDIT.md).

> Follow-up: the user confirmed the generated `.paint` opens and looks correct in Microsoft Paint. This is manual opening/appearance evidence; native layer edits and saving back were not reported. Development has advanced to [Phase 4](PHASE4_VALIDATION.md). Earlier connection failures below describe the agent’s previous run.


## Implemented

- `.palette` v3 writes bounded descriptors and independent binary zlib assets, deduplicates shared COW tiles and preserves tile padding/hidden RGB, layers, masks, filters, effects, selection, offsets, canvas backing color, IDs and opaque ICC/Exif metadata.
- The v1 JSON and v2 binary readers migrate without touching the input; explicit Save writes v3 atomically. Version 2 defaults backing to transparent, preserving its appearance. Malformed geometry/references, decoded-size amplification, corruption, truncation and unknown versions fail explicitly.
- Real still AVIF uses ravif/rav1e encoding and independent avif-decode/rav1d decoding. Export exposes RGB quality/speed, uses maximum alpha quality and leaves native project state dirty. Tests cover RGBA alpha, opaque RGB, 1 x 1, 10-bit input, dimensions, quality changes, cancellation and failed-export target preservation. Unsupported animation and unapplied HEIF rotation/crop metadata are rejected.
- Ordinary image decoders apply Exif orientation and retain original ICC/Exif bytes in native projects. No working-profile conversion or flattened-export metadata preservation is claimed.
- File > Import to canvas inserts a raster as a selected new layer in one history command, expanding canvas bounds when needed. A changed/closed originating document rejects stale decode results.
- Up to ten recent opened/imported/native-saved paths persist atomically. Recovery snapshots committed stable edits about every 30 seconds without overwriting originals or marking tabs saved. Startup Restore/Discard/Later, live-instance leases, retained original recovery until replacement/save, and late-write cleanup are implemented.

## Validation on the available Windows environment

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 105 passing tests; two desktop tests excluded from the ordinary run.
- `cargo build --workspace --locked`: passed; executable launched through native wgpu capture mode.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passing tests cover twenty-one reviewed native states at 1386 x 963. Reviewed File, AVIF/recovery, close prompt, imported Paint project and Paint export scenes; compact 780 x 520 recovery and close prompt reviewed.
- Paint source/composite round trips, raw/zlib/Deflate decoding, offsets/partial-alpha/unknown properties and malformed containers; native v1/v2 migration, sparse 4K tile sharing, metadata/native preservation, malformed-file/size checks and AVIF/Exif tests pass. App tests cover origin-tab import/undo, stale imports, recovery across lost lease ownership, dirty-state preservation, subsequent save cleanup, recents persistence, discard during pending autosave, AVIF Cancel/export semantics, close Save/Don’t save/Cancel/Escape/centering, and Paint export backup/Cancel/revision/gated-write semantics. Existing editing and history tests pass.

Build environment: Rust 1.99.1, Windows/MSVC; declared minimum raised to 1.98 for avif-decode 3. NASM 3.01 was downloaded from the official NASM distribution into ignored target/build-tools and prepended to the build commands' PATH. scripts/dev.ps1 can reuse it. No system codec DLL is required. CI installs NASM and runs the same checks on Windows, Linux, Apple Silicon macOS and Intel macOS; those remote jobs and Windows 10 were not executed here.

A reproducible sparse-project benchmark is available with `cargo run -p palette-core --example project_io --locked`. One optimized-dev run, two layers with a 40-pixel soft stroke, excluding UI/startup/compositing, measured:

| Canvas | File bytes | Save | Open |
|---|---:|---:|---:|
| 1280 x 720 | 14247 | 5.18 ms | 6.89 ms |
| 3840 x 2160 | 16050 | 4.59 ms | 6.45 ms |

These results reflect sparse shared tiles and a single local run. They do not establish dense-image or 8K/AVIF latency, peak memory or smooth painting performance.

## Changed files

Core document/raster/IO modules, io/project.rs and io/avif.rs, new io/paint/{mod,boxes,writer}.rs, specimen fixture, compatibility integration tests, paint_roundtrip example, independent specimen oracle and project_io benchmark. App/session file routing in app.rs, app/files.rs and persistence.rs, new app/close.rs, installed Windows dialog typography, menus/text-action routing, diagnostic capture scenes and interaction tests. Cargo manifests/lockfile, CI NASM setup, development helper, reviewed fixtures, dependency inventory and roadmap/architecture/compatibility/checklist/limitations/build documentation were updated. All changes remain uncommitted.

## Open gates

The supplied specimen is now verified: ISO BMFF/HEIF with two 819 x 594 unci sources, a white-backed iovl primary, explicit dimg order/offsets and Exif. The original stays unchanged. A dedicated safe-Rust subset handler writes actual HEIF, retains independent pixels/order/alpha/offsets and metadata, and never labels native Palette bytes as `.paint`. The independent Deflate/BGRA/Pillow oracle matches Palette's composite pixel for pixel. A temporary current libheif 1.23.6 build was tested but lacks uncompressed decoding; it rejects both original and generated files and supplies no external decode acceptance evidence. See [specimen evidence](PAINT_RESEARCH.md).

The close prompt reproduces the supplied screenshot's 340 x 188 logical-point box, 8-point corners, two gray surfaces, cyan Save, three 92 x 32 buttons, installed Segoe typography and undimmed surroundings. Geometry/body alignment and actual Save/Discard/Cancel routing are tested; normal and compact GPU captures were reviewed. No exact WinUI animation or comprehensive accessibility parity is asserted.

Phase 3's local implementation is complete for the documented subset, but **Workflow G acceptance remains open**: actual Microsoft Paint must open the generated file with two editable layers and save it back. The Windows Computer Use native connection is unavailable (`os error 2`); no such interaction was performed. `target/paint-roundtrip.paint`, `.png` and `.palette` are ready to review. Restore the connection or manually follow the checks in PAINT_RESEARCH.md. Windows 10/macOS/Linux execution and broader independent AVIF/color/metadata fixtures remain separate gates. The next development phase after native validation is Phase 4: blend modes and coverage-based selection/mask workflows.

Recovery snapshots omit uncommitted text/path drafts, history and viewports, and can lose the last interval after a crash. Running codec/recovery jobs are not fully cancellable. Color management/HDR, AVIF HEIF transforms/animation, original AVIF metadata and packaged license notices remain future work. See KNOWN_LIMITATIONS.md and FILE_FORMATS.md for exact limits and storage locations.


## Local completion follow-up — brush-selection milestone

Native writing is now v4 (reads v1–v4), as documented in FILE_FORMATS.md. This review closed the remaining identified ordinary-import notification gap: detected animated WebP/APNG and TIFF imports report exactly which frame/default image/page was retained, including image-as-layer. GIF notices now follow the detected codec rather than just the filename. Generated two-color fixtures verify first/default-image pixels and misnamed-file handling; app tests verify both open-tab and layer-import notices. Still PNG/WebP files stay quiet. These changes add no dependencies or format-version change.

The local documented Phase 3 subset is implemented and its codec/native/Paint round-trip and atomic-save tests pass. The actual Microsoft Paint layer-edit/save-back gate remains unverified: the Computer Use skill was initialized and app discovery failed with a missing native pipe (os error 2). No native application input was attempted after that failure. The existing user confirmation that exported output opens and looks correct is retained. Windows 10/macOS/Linux execution remains unavailable here; no commits, pushes or remote CI triggers were used. Full working-profile conversion, AVIF metadata/12-bit external conformance, broader Paint variants and packaged release notices remain explicit future/release work. They have not been relabeled as completed.

Current Windows validation for this follow-up: formatting, strict Clippy, 156 workspace tests and build pass; two native desktop tests cover 34 screenshot states. The specimen hash remains unchanged. See [brush-selection evidence](BRUSH_SELECTION_VALIDATION.md) for exact checks and performance scope.
