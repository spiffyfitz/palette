> Historical ants/gallery milestone. The [construction/transform follow-up](CONSTRUCTION_TRANSFORMS_VALIDATION.md) enables Polygon and Curve, auto-selects finished shapes and adds selection-aware/custom rotation.

# Selection ants and reference shape gallery

## Requested changes

This follow-up implements animated selection borders, fixes preview stretching near zero shape dimensions, and uses the exact shape ordering of the supplied toolbar crop. It preserves the existing 1280 x 720 new-document default and 100% initial zoom.

## Gallery reference

The attached crop was opened and inspected locally. No proprietary icon assets were copied into the application. Original Rust vector icons use these reference rows:

| Row | Left to right |
|---|---|
| 1 | Line, Curve, Oval, Rectangle, Rounded rectangle, Polygon, Triangle |
| 2 | Right triangle, Diamond, Pentagon, Hexagon, Right arrow, Left arrow, Up arrow |
| 3 | Down arrow, Four-point star, Five-point star, Six-point star, Rectangular callout, Oval callout, Cloud callout |
| 4 | Heart, Lightning |

Cells are 22 x 18 logical points within the existing ribbon footprint. The former duplicate star/arrow/polygon placeholders were removed. Curve and free Polygon remain explicitly disabled until their interaction engines are implemented. The other 21 entries are functional raster tools, including ten newly implemented contours: right triangle, left/up arrows, four/six-point stars, three callouts, heart and lightning. The silhouette geometry is independently implemented rather than an assertion of exact proprietary geometry equivalence.

## Thin preview fix

The previous preview used epaint closed-path strokes. The local epaint source confirms that their corner normals use an unlimited miter (`normal / length_sq`), which becomes unstable when adjacent edges approach opposite directions. This can extend a nearly flat ellipse or polygon far past the drag endpoints.

`src/canvas/overlays.rs` removes consecutive duplicate points and uses an open, bevel-limited loop for previews. Exactly flat core contours become line segments; a single visible point becomes a bounded circular footprint. The CPU rasterizer already uses bounded round brush footprints. Fill triangles continue to use the shared contour. Preview bevels and CPU round footprints still differ slightly at edges/opacity; exact preview/final styling parity remains future work.

Regression coverage checks both the helper and actual app preview after tessellation, rather than only checking path coordinates. Zero, near-zero and narrow width/height cases must emit finite vertices within a small stroke-width margin of the drag bounds. CPU tests also verify that committed outline pixels stay bounded for every implemented closed shape.

## Animated selections

Directed coverage edges link into ordered loops, including holes and diagonally touching islands. The existing selection cache now retains those loops. Animation is presentation-only: a one-point white outline plus black four-point dashes, moving twelve logical display points per second with 50 ms repaint requests while visible. Phase continues around corners and through clipping; holes follow their own directed contour. Drag creation and selected-pixel movement share this treatment.

Offscreen portions are clipped before dash generation, keeping high-zoom or panned selections from allocating invisible primitives. No model, source pixels, selection coverage, renderer generation or history revision is changed by the clock. Diagnostic screenshot mode freezes phase; ordinary editing animates.

## Changed files

- `crates/palette-core/src/editing.rs`: directed boundaries and closed contour tracing.
- `crates/palette-core/src/shapes.rs`: new reference shapes and degenerate contour handling.
- `src/canvas.rs`, `src/canvas/overlays.rs`, `src/session.rs`: bounded previews, animated/clipped ants and cached selection loops.
- `src/ribbon.rs`, `src/icons.rs`: reference gallery order and original matching icons.
- Core/app regression tests; native capture scenes and ten reviewed Windows screenshot baselines.
- README, architecture, checklist, limitations and build/UI notes.

## Validation

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo test --workspace --locked`: 62 passing tests; two desktop tests excluded from this normal run.
- `cargo build --workspace --locked`: passed.
- `cargo test --test screenshot --locked -- --ignored --test-threads=1`: two passing desktop tests compare ten native eframe/wgpu captures.
- Reviewed the native main window, new catalog, selection with a hole, existing dialogs/menus/text/layers, and compact 780 x 520 layout. The gallery remains inside the ribbon and the compact window retains horizontal ribbon scrolling.

New tests verify reference ordering, actual thin-preview mesh bounds, flat CPU output for all 20 closed shapes, finite helper tessellation, dash motion/corner phase/offscreen clipping, hole/diagonal-island topology, and UI-time animation without any document or history mutation. Existing painting, layer movement, alpha, selection, file and undo tests remain passing.

Native captures establish Windows rendering/layout; they seed reproducible states and do not claim manual native pointer validation or capture animation motion. Motion is verified using real egui frames with advancing time. No macOS/Linux/Windows 10 launch or high-DPI matrix was tested here. No new dependencies were added. All changes remain uncommitted under AGENTS.md.
