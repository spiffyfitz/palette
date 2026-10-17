# Layer processing controls and on-canvas previews

Windows validation, 2026-10-17. This is a presentation/interaction follow-up to the first Phase 5 slice. It adds no filter algorithms, dependency or project format change.

## Working controls

The property card stays beneath the selected layer. Three equal-sized original geometric icon buttons now sit in one row directly below Opacity:

| Icon | Control | Behavior |
|---|---|---|
| Three adjustment sliders | Filters | Toggle the active layer's contextual filter toolbar |
| Sparkle | Effects | Toggle the active layer's contextual effect toolbar |
| Overlapping circles | Blending mode | Open the anchored, scrollable catalog of 19 implemented modes; the current mode is selected |

Hover text identifies each icon; Filters/Effects show an active outline, and a non-Normal blend highlights its button. The existing ribbon Effects control opens the same effects toolbar, and Filter-menu commands use the same filter editor. Selection/Text and processing bars are mutually exclusive.

Processing bars sit four points below the workspace top and use the same 28-point controls and eight-point frame as the Text/Selection bars: 46 points overall height. They fit their contents, center in the usable canvas viewport, narrow controls in compact windows and leave canvas position/zoom unchanged. They replace the earlier floating stack window and modal parameter dialog.

The stack bar selects an instance, adds an implemented operation, toggles it On, and opens Edit. The … menu offers Move up/down, Duplicate and Remove; filter entries also retain selection-derived mask creation/toggle/removal. Edit shows the operation name when space permits, a parameter selector, its slider/numeric field or color control, Preview, Apply, Cancel and an action menu. All six shadow settings remain accessible without increasing toolbar height. Remove is available both from the stack menu and while editing an existing instance. A pending new instance is discarded with Cancel.

## Live and nondestructive behavior

The canvas displays accepted worker preview textures directly, including layer blending, masks and existing filters/styles. Inputs remain source-image units regardless of zoom. Preview can be switched off to compare the committed image; Cancel/Escape immediately restores committed presentation. Source pixels, saved state, history revision and native assets do not change during parameter previews. Apply commits editable parameters once, and a ready preview remains visible until the normal canvas renderer supplies the committed full result. Source data is never replaced by the preview.

Removing a filter/effect changes only its stack entry and is undoable. Native `.palette` saving retains source assets, parameters, enabled states, ordering and masks. Existing `.paint` compatibility safeguards are unchanged.

The preview worker retains cancellation tickets and session/generation/layer checks. Tool/tab/layer changes discard unapplied settings; stale results cannot appear on another layer. Painting and editing shortcuts are blocked during a draft, while navigation and Apply/Cancel remain usable. Toolbar hit rectangles exclude underlying painting/selection input, including the first opening/closing frame. Paint-only progress status and stable action IDs keep a completed preview from changing widget identity during a click/menu operation.

## Verification

- `cargo fmt --all --check` passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- `cargo test --workspace --locked`: 169 tests pass (80 app, 89 core); ordinary runs leave the two desktop test functions ignored.
- `cargo build --workspace --locked` passes; native Windows wgpu executable launched for captures.
- Desktop regression passes: two test functions cover 41 states at 1386 × 963. Updated captures cover the new layer row, filter/effect canvas editors and stacks, blend popup and existing layer/mask/scrollbar scenes. The effect bar and property row were also inspected at 780 × 520.
- Three new app tests click the layer icons and toolbar controls, verify an actual canvas mesh uses the draft texture, compare committed presentation when Preview is off, remove filters/effects from both editing/stack menus, exercise undo/native preservation and test compact bars at 1×/1.5×/2× DPI. The prior parameter-input test still verifies real slider changes; existing tab-stale/cancellation and live-stroke tests pass.
- The test label helper now waits for stable Area geometry, scopes ambiguous Edit labels to the toolbar and clears epaint texture deltas before assertions can unwind.

## Limits

Previews are full resolution through a 2048-pixel maximum side; larger images use a reduced composite with scaled spatial parameters and linear presentation. Full-size previews use nearest sampling. Final rendering always uses full-resolution source/parameters. Expensive stacks can lag; background work/cancellation does not establish smooth dense 4K/8K performance. The previous [CPU measurements and cache limits](PHASE5_VALIDATION.md) still apply; the larger preview cap has not received a new latency benchmark.

Parameters are selected one at a time to preserve the compact bar. Unapplied drafts are intentionally excluded from autosave/native saving. Locked layers must be unlocked before editing/removing processing. Broader filters/styles, GPU processing and untested operating systems remain tracked. Changes are left uncommitted; no Git branch or remote operation was performed.
