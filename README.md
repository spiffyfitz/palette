# Palette

A native Rust raster editor with a Paint-style ribbon and a UI-independent, layered editing engine. The runnable foundation now includes the first Phase 2 editing workflow; the full roadmap is still in progress.

## Run

Install stable Rust **1.95 or newer**, then:

```sh
cargo run --locked
# Open a file from the command line:
cargo run --locked -- path/to/image.png
```

Windows needs the MSVC C++ build tools and Windows SDK. macOS needs Xcode Command Line Tools. On Linux, install a C/C++ toolchain, pkg-config, X11/Wayland development packages, a working Vulkan driver, an XDG desktop portal backend and Zenity. See [build and packaging notes](docs/BUILD.md).

## Working now

New documents use a **1280 x 720** white canvas at **100% zoom**. Image Properties > Default uses the same size. Use Fit or pan when the canvas is wider than the workspace.

- Paint-style dark ribbon, split buttons, anchored flyouts, hover/press animation, compact document tabs, animated layers tray, three contextual brush controls and status bar.
- Centered Edit Colors and Image Properties dialogs with OK/Cancel, custom color slots, exact RGB/hex input and reversible canvas bounds changes. View toggles for gridlines, status bar, ribbon and layers.
- New/open documents; switch, drag to reorder and close tabs using an integrated hover-only button; independent zoom, layers and history; modified-document close prompts.
- Pencil, continuous round brush, soft eraser, color picker and magnifier. All 23 reference shape-gallery tools are functional, including free Polygon and a two-bend cubic Curve. Shapes have independent foreground outline/background fill, opacity, width, drag previews and Shift constraints. Finished shapes are selected automatically; right mouse swaps drawing colors.
- Size, opacity and hardness controls, including exact numerical input. Erasing reduces alpha.
- Brush, pencil and eraser strokes appear while dragging, with partial canvas texture updates and one undo command per stroke.
- Text: type directly on the canvas with a floating formatting strip, move the box by its top edge, or resize using eight handles. Ubuntu Light/Hack, size, wrapping, alignment, line spacing, colors and opacity work. Done, Ctrl/Command+Enter or clicking outside commits a separate raster layer; Cancel/Escape leaves pixels unchanged.
- Add, duplicate, rename, show/hide, lock, reorder and delete layers; properties directly beneath the selected layer; opacity; Normal, Multiply and Screen compositing.
- Move whole layers through Selection > Move active layer or the layer context menu: drag with live rendering, Shift to constrain an axis, arrows to nudge (Shift for ten pixels), Escape to cancel. One drag is one undo command; pixels, masks and filters remain intact.
- Pan with middle mouse or Space+drag; scroll to pan; Ctrl/Command+wheel to zoom around the pointer; 100% default zoom with slider snapping at actual size, fit and canvas corner resize.
- Native open/save dialogs, file drop, atomic `.palette` saving and flattened raster export. PNG, JPEG, BMP, GIF, TIFF and WebP have real codec round-trip tests.
- Two live filters, Invert and Brightness, with editable, enabled/disabled, duplicated and reordered stacks. Source pixels are retained.
- Polygon: click vertices with a live next-segment line; Enter, double-click or click the first vertex closes it. Backspace removes a vertex; Escape cancels. Curve: drag a baseline, then drag two bends; Enter can finish early.
- Eight selection resize handles and a rotation handle, with live processed-pixel previews for move/resize/rotation. Shift locks resize proportions, constrains movement or snaps rotation to 15 degrees; Escape cancels. A completed gesture is one undo command.
- Animated marching-ants outlines around selections, including holes, moving selections and creation previews.
- Rectangular selections, select all/deselect/invert, selection-constrained painting/erasing/shapes, clear and selection-size status. Drag selected pixels to move them; Alt+drag starts a replacement selection. Arrow keys nudge, Shift+arrows move ten pixels.
- Contiguous fill with adjustable tolerance (0–255) and opacity. Right mouse uses the background color.
- System image clipboard copy/cut/paste; paste creates a new layer and expands the canvas if needed. Copy samples the active layer's evaluated appearance.
- Crop canvas to selection, horizontal/vertical flip, exact quarter-turns and custom-angle rotation. A selection limits flips/rotations to active-layer selected source pixels; without a selection they transform the whole document. Custom rotation uses alpha-safe bilinear sampling and an OK/Cancel dialog. Resize offers nearest or bilinear sampling with optional aspect lock; layers and live processing stacks are retained.
- Consistent stroke opacity across pointer events/retracing, tile-sharing-aware undo accounting, and background fill/geometry jobs with stale-result protection.

Unavailable ribbon tools and filter categories are visibly disabled. **Persistent vector shape objects, brush variants, Microsoft `.paint`, AVIF, advanced selections, mask painting and most filters/effects are not available yet.** Text supports basic left-to-right layout with two bundled fonts and becomes raster content on Apply; system font browsing and complex-script shaping remain pending. The `.paint` specimen was not attached; its reported structure has not been verified. Recovery/autosave is still pending.

Save/Save As preserve editable state in `.palette`. Export writes a composite image and does not mark the editable project saved. GIF imports its first frame; export does not create an animation. JPEG composites transparency over white.

## Shortcuts

| Action | Shortcut |
|---|---|
| New / Open / Save | Ctrl+N / Ctrl+O / Ctrl+S |
| Save As / Close tab | Ctrl+Shift+S / Ctrl+W |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y |
| Next / Previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Copy / Cut / Paste image | Ctrl+C / Ctrl+X / Ctrl+V |
| Select all / Deselect | Ctrl+A / Ctrl+D or Escape |
| Clear selected pixels | Delete / Backspace |
| Move selected pixels | Arrows; Shift+arrows for ten pixels |

Use Command in place of Ctrl for file/edit shortcuts on macOS; tab cycling uses Control.

## Verify

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
# Requires an interactive Windows desktop:
cargo test --test screenshot -- --ignored --test-threads=1
```

Capture the actual wgpu window with `palette --screenshot output.png`. Debug builds also accept `--screenshot-scene colors` and `--screenshot-size 780x520`. Sixteen reviewed Windows states at 1386 x 963 guard the window, menus, dialogs, text and layers. See [video behavior and validation](docs/UI_BEHAVIOR_REFERENCE.md).

See [architecture](ARCHITECTURE.md), [feature checklist](FEATURES.md), [known limitations](KNOWN_LIMITATIONS.md), [file compatibility](docs/FILE_FORMATS.md), [dependency licenses](docs/DEPENDENCIES.md) and [current milestone validation](docs/SELECTION_HANDLES_VALIDATION.md).

Palette source is MIT licensed. Icons are original geometric Rust drawings; Microsoft and Adobe branding/resources are not bundled.
