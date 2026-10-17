use palette_core::{raster::MAX_PIXELS, selection};

#[test]
fn lasso_closes_concave_paths_clips_and_is_independent_of_direction() {
    let points = [[-2., 1.], [6., 1.], [6., 4.], [3., 4.], [3., 7.], [-2., 7.]];
    let mask = selection::freeform(8, 8, &points).unwrap();
    for y in 0..8 {
        for x in 0..8 {
            let expected = (1..7).contains(&y) && x < 6 && (y < 4 || x < 3);
            assert_eq!(mask.at(x, y), if expected { 1. } else { 0. }, "{x},{y}");
        }
    }
    let reversed: Vec<_> = points.into_iter().rev().collect();
    assert_eq!(mask, selection::freeform(8, 8, &reversed).unwrap());
}

#[test]
fn lasso_antialiasing_retains_fractional_coverage_and_self_crossing_is_even_odd() {
    let square = [[1.25, 1.], [4.75, 1.], [4.75, 5.], [1.25, 5.]];
    let mask = selection::freeform(8, 8, &square).unwrap();
    assert_eq!(mask.values[(2 * 8 + 1) as usize], 191);
    assert_eq!(mask.at(2, 2), 1.);
    assert_eq!(mask.at(0, 2), 0.);
    assert!(
        (mask
            .values
            .iter()
            .map(|v| f32::from(*v) / 255.)
            .sum::<f32>()
            - 14.)
            .abs()
            < 0.02
    );
    let bow = [[1., 1.], [7., 7.], [1., 7.], [7., 1.]];
    let mask = selection::freeform(8, 8, &bow).unwrap();
    assert_eq!(mask.at(3, 1), 1.);
    assert_eq!(mask.at(1, 3), 0.);
    assert_eq!(mask.at(3, 6), 1.);
}

#[test]
fn lasso_degenerate_offcanvas_and_invalid_inputs_remain_bounded() {
    for points in [
        [[1., 1.], [2., 2.], [3., 3.]],
        [[-20., -20.], [-10., -20.], [-10., -10.]],
        [[20., 20.], [30., 20.], [30., 30.]],
    ] {
        assert!(
            selection::freeform(8, 8, &points)
                .unwrap()
                .values
                .iter()
                .all(|v| *v == 0)
        );
    }
    let valid = [[0., 0.], [1., 0.], [0., 1.]];
    assert!(selection::freeform(0, 8, &valid).is_err());
    assert!(selection::freeform(u32::MAX, u32::MAX, &valid).is_err());
    assert!(selection::freeform(MAX_PIXELS as u32 + 1, 1, &valid).is_err());
    assert!(selection::freeform(8, 8, &valid[..2]).is_err());
    assert!(selection::freeform(8, 8, &vec![[0.; 2]; 4097]).is_err());
    for bad in [f32::NAN, f32::INFINITY, -1_000_001.] {
        let mut points = valid;
        points[1][0] = bad;
        assert!(selection::freeform(8, 8, &points).is_err());
    }
}
