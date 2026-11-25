//! Lateral pose: walkers keep to the right, the offset is capped, and spawn sides vary.

use super::*;

#[test]
fn walkers_keep_to_the_right_so_opposite_walkers_pass_on_different_sides() {
    let n = north_lane(2.0);
    let (north, h_north) = walker_on(0, 1, 0.5).pose(&n).unwrap();
    let (south, h_south) = walker_on(0, -1, 0.5).pose(&n).unwrap();
    // Walking north the right hand is east; walking south it is west; half the width is 1 m.
    assert!((north.x - 0.5).abs() < 1e-5, "{north}");
    assert!((south.x + 0.5).abs() < 1e-5, "{south}");
    assert!((north.y - 50.0).abs() < 1e-5 && (south.y - 50.0).abs() < 1e-5);
    assert!(
        (north - south).length() >= 0.99,
        "a metre apart at the same place"
    );
    assert!(h_north.abs() < 1e-5 && (h_south - std::f32::consts::PI).abs() < 1e-5);
}

#[test]
fn no_lateral_offset_walks_on_the_centre_line_as_before() {
    let n = north_lane(2.0);
    let (p, _) = walker_on(0, 1, 0.0).pose(&n).unwrap();
    assert!(p.x.abs() < 1e-6);
}

#[test]
fn the_offset_is_capped_on_a_wide_lane_and_ignores_a_broken_width() {
    let wide = north_lane(20.0);
    let (p, _) = walker_on(0, 1, 1.0).pose(&wide).unwrap();
    assert!((p.x - super::walker::MAX_OFFSET_M).abs() < 1e-5, "{p}");
    for width in [f32::NAN, f32::INFINITY, -3.0, 0.0] {
        let odd = north_lane(width);
        let (p, _) = walker_on(0, 1, 0.8).pose(&odd).unwrap();
        assert!(p.is_finite() && p.x.abs() < 1e-6, "width {width}: {p}");
    }
}

#[test]
fn new_walkers_draw_their_side_within_the_range_and_differ() {
    let n = north_lane(2.0);
    let mut rng = 12345;
    let sides: Vec<f32> = (0..200)
        .map(|_| Walker::new(&n, 0, 10.0, 1.4, &mut rng).unwrap().lateral)
        .collect();
    assert!(sides.iter().all(|l| (0.15..=0.85).contains(l)));
    let (lo, hi) = sides
        .iter()
        .fold((1.0f32, 0.0f32), |a, l| (a.0.min(*l), a.1.max(*l)));
    assert!(hi - lo > 0.5, "the sides vary: {lo}..{hi}");
    let mut again = 12345;
    let first = Walker::new(&n, 0, 10.0, 1.4, &mut again).unwrap().lateral;
    assert_eq!(first, sides[0], "the same seed gives the same side");
}
