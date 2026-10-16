# Visual reference and regression testing

Current follow-up: selected-pixel move/resize/rotation handles and live previews are implemented; see [selection handle validation](SELECTION_HANDLES_VALIDATION.md). Earlier pending items and test counts below describe the historical milestone.

Both provided PNGs were opened and examined before implementation. The Paint image is 1309 × 962, with a 30-pixel native title bar, approximately 41-pixel menu row, 108-pixel ribbon, dark #1e1e27 workspace, centered white 819 × 594 canvas, left-side brush controls and compact bottom status bar. Its zoom indicator reads 100%, which differs from the brief's approximate resolution/75% description.

Palette uses a 1386 x 963 content capture and 100% initial zoom. The current default canvas is 1280 x 720, updated after the video-guided UI pass. The zoom slider snaps within five percentage points of 100%; wheel zoom remains continuous. The added 31-point tab strip sits between the menus and ribbon. Each tab reserves an internal close-button slot; the button appears only while hovering that tab, without shifting its label or neighbors. Menu, ribbon and status proportions retain the reference layout; account, feedback, share and Copilot controls are absent. See [current milestone](SHAPES_LAYERS_VALIDATION.md).

The three left controls are stacked in the workspace, with exact numerical values. The layers panel floats in the right workspace margin, with thumbnails and property controls directly beneath the selected layer, ahead of any lower layers. On smaller windows the ribbon scrolls horizontally and the layer list scrolls vertically. Unimplemented tools remain visibly dim and have explanatory tooltips. Shapes and toolbar icons are drawn with original geometry to avoid missing-font glyphs or copied assets. Phase 2 activates the existing Selection, Fill and Image controls; compact rotation/flip menus stay in the same ribbon group. A resize dialog and a contextual tolerance value reuse the established layout. Selection contours and dimensions appear only when relevant. The baseline was visually reviewed again after these controls became functional.

The Filter menu preserves the reference category order. Only Adjustments > Invert and Brightness currently work. Other categories are disabled and identified as unimplemented; there is no fake Gaussian Blur or Gallery command.

The Text icon is functional. The latest pass replaces the earlier compact editor with direct canvas typing, eight resize handles, a move edge and a floating formatting strip beneath the ribbon. Historical live-stroke validation remains in LIVE_PAINT_TEXT_VALIDATION.md. The supplied video now guides menu spacing, split controls, centered dialogs, animations and the narrow layer tray; see [current behavior reference and validation](UI_BEHAVIOR_REFERENCE.md).

## Recorded comparison

An initial actual wgpu capture was visually reviewed. Missing Unicode shape glyphs were replaced with drawn icons, oversized ribbon icons were reduced, the layer opacity control was constrained to the panel, and brush sliders gained cyan circular handles and numerical input. The final reviewed Windows baseline lives in `tests/fixtures/window-windows.png`.

`tests/screenshot.rs` launches the actual executable, asks the wgpu backend for a screenshot, verifies 1386 × 963 dimensions and compares against the baseline. A pixel is changed when any RGB channel differs by more than 24; fewer than 0.5% changed pixels are permitted to tolerate small rasterization/hover differences. The screenshot callback waits until the canvas texture has rendered and initial animations have settled. Thirteen additional scene baselines cover File, Brushes, Colors, Text, Image Properties, expanded layer properties, common shapes with a moved layer, selection holes, the extended shape catalog, custom rotation, selected shapes and polygon/curve previews. The current gallery follows the supplied toolbar crop in 7/7/7/2 rows; see [selection/shape follow-up](SELECTION_SHAPES_VALIDATION.md). It does not use eframe's Glow-only screenshot environment variable.

```sh
cargo test --test screenshot -- --ignored --test-threads=1
```

This guards regressions against Palette's reviewed layout. It does not establish pixel equality with Microsoft Paint or equivalent platform presentation. Keep the Windows baseline on Windows; capture separately reviewed baselines on macOS/Linux when available. Test changed baselines visually before replacing them.

The original user-provided screenshots are retained in [Paint UI](references/paint-ui.png) and [Filter menu](references/filter-menu.png) for development comparison. These third-party reference images are not Palette source assets and are excluded from the source-code MIT license; no permission to redistribute Microsoft's or Adobe's proprietary resources is implied. They are not loaded by the application.
