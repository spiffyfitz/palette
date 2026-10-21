# Master roadmap review

Current scope, 2026-10-20: a native Paint-style editor with nondestructive conventional filters and layer effects. At the user's request, all twelve Neural Filters and AVIF support have been removed, together with the model assets, inference runtime, codec dependencies and specialized development tools. This deliberately narrows the original master plan; those features are no longer planned acceptance requirements.

| Phase | Current evidence | Remaining gate / next work |
|---|---|---|
| 0 — inspection/design | UI-independent document model, native Rust UI, reference/video research and specimen inspection | Broader Paint format variants |
| 1 — runnable interface | Paint-style ribbon, tabs, layers, zoom, brush controls and ordinary raster I/O | Accessibility and cross-platform visual/behavior verification |
| 2 — core painting | Live strokes with nine brush families, independent sizes/size preview, oil/watercolor paint depletion, erasing, fill, shapes, text, clipboard, selections, transforms and undo | Tablet hardware/tilt validation, adjustable spacing, text shaping and mask painting |
| 3 — file compatibility | PNG/JPEG/BMP/GIF/TIFF/WebP, native v8 preservation and older-project migration, genuine layered .paint subset, recents/recovery/atomic saves | Broader Paint variants, Paint edit/save-back cycle, metadata-aware export and working color conversion |
| 4 — advanced layers/selections | 19 blend modes, Wand/freeform/edge-aware selection brush, feathering, masks, multiple layer selection and undoable merge | Polygon selection, morphology and fuller mask editing/view/apply/link workflows |
| 5 — filters/styles | Conventional filter registry, ten live style families, transactional canvas previews, masks and editable native persistence | Filter Gallery, presets, per-effect masks and broader quality validation |
| 6 — platform polish | Persistent GPU layer presentation, unified Move, CI configuration, leaner dependency graph and optimized release profile | GPU processing kernels, dense 4K/8K benchmarks, packaging/signing and actual platform matrix |

The user confirmed the generated .paint file opens and looks correct in Microsoft Paint. This supports the current experimental subset; it does not establish a complete Paint-save-back cycle or general HEIF support. No macOS/Linux execution is claimed from the Windows environment.

Older .palette documents containing any retired Neural Filter, even disabled, are rejected with an explicit diagnostic and remain untouched. Use an older Palette build to remove or rasterize the filters before saving a compatible copy. Other represented native state remains editable. See [file compatibility](docs/FILE_FORMATS.md).

Next recommended vertical slice: Filter Gallery, with category browsing, a shared sample image, live current-document preview and transactional Apply/Cancel into the existing nondestructive stack. It remains disabled until implemented. Keep the Paint-style main ribbon.

Performance follow-up: per-stage/tile caching and individual GPU filter/effect kernels, particularly for moving processed selections. Existing eligible selection/layer moves retain GPU acceleration and the CPU reference fallback. See [GPU validation](docs/GPU_COMPOSITOR_VALIDATION.md) and [unified Move scope](docs/UNIFIED_MOVE_VALIDATION.md).

The current generated [filter inventory](docs/FILTER_INVENTORY.md) and [dependency audit](docs/DEPENDENCY_AUDIT.md) describe the reduced build. Older milestone validation documents retain historical evidence and explicitly identify retired features. All changes remain uncommitted.
