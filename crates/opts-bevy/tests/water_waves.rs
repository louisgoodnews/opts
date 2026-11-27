//! Wave displacement of the water surface (M-0025 Phase B). Expected values by hand from the
//! trains of `water/waves.rs`: east 0.06 m over 9 m with a 4 s period, north 0.04 m over 6.5 m
//! with a 3 s period.

use opts_bevy::water::{displaced, displaced_height, wave_offset};

const EPS: f32 = 1e-5;

#[test]
fn the_surface_is_at_rest_at_the_origin_at_time_zero() {
    assert!(wave_offset(0.0, 0.0, 0.0).abs() < EPS);
}

#[test]
fn a_quarter_wavelength_east_is_at_the_crest() {
    // sin(2π · 2.25 / 9) = sin(π/2) = 1, so 0.06 m.
    assert!((wave_offset(2.25, 0.0, 0.0) - 0.06).abs() < EPS);
}

#[test]
fn a_quarter_wavelength_north_is_at_the_crest() {
    // sin(2π · 1.625 / 6.5) = 1, so 0.04 m.
    assert!((wave_offset(0.0, 1.625, 0.0) - 0.04).abs() < EPS);
}

#[test]
fn one_second_later_both_trains_have_moved() {
    // East: sin(2π · (0 - 1/4)) = -1, so -0.06 m.
    // North: sin(2π · (0 - 1/3)) = -√3/2, so -0.04 · 0.8660254 = -0.0346410 m.
    let want = -0.06 - 0.04 * 0.866_025_4;
    assert!((wave_offset(0.0, 0.0, 1.0) - want).abs() < EPS);
}

#[test]
fn the_offset_never_exceeds_the_sum_of_the_amplitudes() {
    for ix in 0..40 {
        for iz in 0..40 {
            for it in 0..8 {
                let o = wave_offset(ix as f32 * 0.7, iz as f32 * 0.4, it as f32 * 0.5);
                assert!(o.abs() <= 0.10 + EPS, "{o}");
            }
        }
    }
}

#[test]
fn the_wave_vanishes_at_the_shore_and_is_full_at_one_metre_of_depth() {
    // At the crest point of the east train: offset 0.06 m.
    assert!((displaced_height(-5.0, 0.0, 2.25, 0.0, 0.0) - -5.0).abs() < EPS);
    assert!((displaced_height(-5.0, 0.5, 2.25, 0.0, 0.0) - -4.97).abs() < EPS);
    assert!((displaced_height(-5.0, 1.0, 2.25, 0.0, 0.0) - -4.94).abs() < EPS);
    assert!((displaced_height(-5.0, 6.0, 2.25, 0.0, 0.0) - -4.94).abs() < EPS);
}

#[test]
fn displaced_moves_each_vertex_by_its_own_depth() {
    let rest = [[0.0, -5.0, 0.0], [2.25, -5.0, 0.0]];
    let moved = displaced(&rest, &[0.0, 2.0], 0.0);
    assert!((moved[0][1] - -5.0).abs() < EPS);
    assert!((moved[1][1] - -4.94).abs() < EPS);
    assert_eq!(moved[1][0], 2.25);
    assert_eq!(moved[1][2], 0.0);
}

#[test]
fn a_vertex_without_a_depth_stays_at_its_level() {
    let rest = [[2.25, -5.0, 0.0]];
    let moved = displaced(&rest, &[], 0.0);
    assert_eq!(moved, rest.to_vec());
}
