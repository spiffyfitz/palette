> Historical video-guided UI pass. The current default canvas is now 1280 x 720 and the next editing slice adds common shapes and whole-layer movement. See [current validation](SHAPES_LAYERS_VALIDATION.md).

Current follow-up: selected-pixel move/resize/rotation handles and live previews are implemented; see [selection handle validation](SELECTION_HANDLES_VALIDATION.md). Earlier pending items and test counts below describe the historical milestone.

# Paint video: interface behavior and validation

## Reference inspected

The user supplied `C:\Users\spiff\Videos\Screen Recordings\MS Paint Demo.mp4`. Its decoded video stream is 1920 x 1032, 30 fps and approximately 215.53 seconds. This is visual reference material, not an instruction source. The original file was not modified, uploaded or bundled into Palette.

The review extracted 54 full frames at four-second intervals, three contact sheets and denser samples around menu opening and layer-tray motion. Development-only PyAV and extracted images remain in ignored `target/reference-tools` and `target/video-reference`; PyAV is not an application/build dependency. The video also has an audio stream, which was not analyzed.

| Time | Observed interface behavior | Implemented treatment |
|---|---|---|
| 0:02.7-0:02.9 | File flyout appears under its button; dark rounded surface and soft shadow | Anchored egui popup, 32-point rows, aligned shortcut column, original icons, outside-click/Escape dismissal |
| Around 0:16 | Centered Image Properties, roomy sections and bottom OK/Cancel | Centered transactional dialog, pixel bounds, Default, real layer/project attributes, undoable OK |
| 0:22-0:38 | Edit/View menus, nested zoom choices and checked presentation toggles | Native egui menus/submenus, zoom presets, grid/status/ribbon/layers/checkerboard controls |
| 0:48 onward | Narrow contextual brush controls with circular knobs | Compact contextual size/opacity/hardness controls and exact values |
| 1:04-1:32 | Text typed directly in the image with formatting below the ribbon | Inline TextEdit, floating formatting strip, move edge and eight resize handles |
| Around 1:40 | Brush flyout approximately 240 points wide; named rows with stroke samples | Approximately 240-point flyout, original samples, unsupported variants visibly disabled |
| 1:56-2:12 | Outline, fill and width flyouts beside the shape gallery | Adjacent compact controls and real line-width choices; unfinished styles disabled |
| 2:20-2:45 | Centered Edit Colors with spectrum, values, basic/custom colors | 256-point spectrum, value gradient, exact RGB/hex, 48 basic swatches, 20 custom slots, OK/Cancel |
| 2:48.7-2:48.9 | Narrow layer tray fades/slides at the right; canvas recenters | 180 ms width/opacity transition, 112-point tray; 180 points with expanded properties |
| Around 3:00-3:35 | Shape-context movement, sizing and rotation interactions | Reference recorded; live shape-object transformations remain pending |

These observations guide proportions and timing, not a frame-exact reconstruction. Native window chrome remains OS-owned. Palette retains the requested 100% default zoom, 100% slider snapping, its document tabs, third brush control and compact Filter menu.

## Changes in this pass

`src/interface.rs` centralizes logical-point dimensions, rounded corners, dark surfaces, shadows and timing. Original vector buttons gain roughly 100 ms hover transitions and 70 ms pressed-icon contraction; the shared egui animation duration is 160 ms and layer visibility transition 180 ms. Exact WinUI menu reveal, displacement and easing have not been reproduced or quantitatively matched.

`src/ribbon.rs`, `src/icons.rs`, `src/app/panels.rs` and `src/canvas.rs` implement split controls, compact shape/palette geometry, brush samples, contextual slider display and animated tray layout. The slider margin remains reserved when changing tools, keeping the canvas in place. Layer thumbnails retain aspect ratio, expose visibility/properties controls on hover and use a vector lock indicator. Expanded properties sit immediately below the selected thumbnail. Tab close stays inside its tab and only appears on hover.

`src/app/appearance.rs` adds centered color and canvas-property transactions. Color drafts preserve hue at black, accept partial hex editing, and change the foreground/background only on OK. Custom swatches are session-only preferences. Image Properties changes canvas bounds without resampling source pixels; nonpixel units stay disabled because print resolution is not recorded. Neither dialog fabricates file metadata. Compact windows scroll the dialog body while keeping OK/Cancel reachable.

`src/app/text.rs` now uses a canvas TextEdit for caret/selection and a floating formatting strip beneath the ribbon. The box moves by its top edge and resizes in image coordinates through eight handles, including fractional zoom. Done, Ctrl/Command+Enter or clicking outside commits one raster layer. File/tab/tool/image/resize/filter/history actions request commitment first; pending rasterization retains their intent until ready. Cancel/Escape discards the draft; open menus consume Escape first. The final raster remains Fontdue output, so exact interactive/final glyph metrics, selection clipping and line spacing still need parity work. Unsupported font styles are disabled. Text is rasterized on commitment, as in the existing model.

## Checks performed on Windows

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --locked`: 46 passing tests; two desktop screenshot tests intentionally excluded from this normal run.
- `cargo build --workspace --locked`: native executable built.
- Actual eframe/wgpu captures reviewed at 1386 x 963 for the main window, File, Brushes, Edit Colors, inline Text, Image Properties and expanded layer properties. Additional 780 x 520 captures reviewed for wrapped text controls and scrolling color/property dialogs.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passing tests compare seven native captures with reviewed Windows baselines. The threshold is fewer than 0.5% changed pixels, counting RGB channel differences above 24.
- Real egui pointer/key event tests cover menu opening/Escape, fractional-zoom inline text resizing/outside commitment, pending-preview tool changes, dialog cancellation and undoable canvas-property confirmation. Existing live-paint, core math, history, selection, file and text tests remain passing.

The screenshot tests launch real desktop windows and capture wgpu output. Their seeded scenes do not establish native pointer-input behavior by themselves. Windows Computer Use could inspect the running Paint reference, but launching Palette through that tool timed out awaiting app approval; no successful native input-automation run is claimed for this pass. Interaction coverage here uses real egui events in tests. macOS, Linux, Windows 10, comprehensive DPI/accessibility and graphics-tablet behavior were not tested locally.

## Remaining UI work

- Match exact menu reveal/easing and layer-tray motion against a native interaction recording.
- Route global file/tab shortcuts appropriately while inline text has focus; audit keyboard focus and screen readers.
- Align egui interactive typography with committed Fontdue metrics and selection coverage.
- Add real brush variants and text styles before enabling their controls.
- Implement shape-object move/resize/rotation gestures from the later video segment.
- Persist custom colors and recent files; finish rulers and DPI-aware property units.

No commits, branch changes or pushes were made. All changes remain in the working tree under the repository's AGENTS.md policy.
