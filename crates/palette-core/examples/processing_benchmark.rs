//! CPU processing timings for a transparent, full-sized layer, not UI latency.
use palette_core::{
    composite::Renderer,
    document::{Document, EffectInstance, Filter, FilterInstance},
    processing::{Kind, Operation, gaussian_blur},
    raster::Raster,
};
use std::time::Instant;

fn main() -> Result<(), String> {
    for (w, h) in [(1280, 720), (3840, 2160)] {
        let mut source = Raster::new(w, h, [0; 4])?;
        for y in h / 4..h * 3 / 4 {
            for x in w / 4..w * 3 / 4 {
                source.set(x, y, [225, 70, 55, 255]);
            }
        }
        let started = Instant::now();
        let blurred = gaussian_blur(&source, 8.)?;
        let blur = started.elapsed();
        assert!(blurred.get(w / 4 - 1, h / 2)[3] > 0);
        for (label, radius, filter) in [
            (
                "Box Blur",
                4,
                palette_core::processing::box_blur_cancel
                    as fn(&Raster, u8, &dyn Fn() -> bool) -> Result<Raster, String>,
            ),
            ("Median", 1, palette_core::processing::median_cancel),
        ] {
            let started = Instant::now();
            let output = filter(&source, radius, &|| false)?;
            assert_eq!(output.get(w / 2, h / 2), [225, 70, 55, 255]);
            println!(
                "{w} x {h}: {label}, radius {radius} px {:.2?}",
                started.elapsed()
            );
        }
        let mut doc = Document::from_raster(source);
        let id = doc.allocate_id();
        doc.active_mut().filters.push(FilterInstance {
            id,
            enabled: true,
            filter: Filter::GaussianBlur { radius: 8. },
            mask: None,
        });
        for kind in [Kind::DropShadow, Kind::OuterGlow] {
            let Operation::Effect(effect) = kind.default_operation() else {
                unreachable!()
            };
            let id = doc.allocate_id();
            doc.active_mut().effects.push(EffectInstance {
                id,
                enabled: true,
                effect,
            });
        }
        doc.validate()?;
        let mut renderer = Renderer::default();
        let started = Instant::now();
        let cold = renderer.render(&doc)?;
        let cold_time = started.elapsed();
        let started = Instant::now();
        let cached = renderer.render(&doc)?;
        let cached_time = started.elapsed();
        assert!(cold.rgba() == cached.rgba());
        // A large prepared stack may deliberately exceed the bounded LRU.
        assert_eq!(renderer.evaluations + renderer.cache_hits, 2);
        println!(
            "{w} x {h}: Gaussian 8 px {blur:.2?}; blur + shadow + glow cold {cold_time:.2?}; repeat {cached_time:.2?}; {} cache hits / {} evaluations",
            renderer.cache_hits, renderer.evaluations,
        );
    }
    Ok(())
}
