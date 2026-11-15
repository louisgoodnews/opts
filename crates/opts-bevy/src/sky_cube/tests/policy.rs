//! The regeneration policy and the exposure: when the cube is rebuilt, and how bright it is.

use super::*;

#[test]
fn the_first_cube_is_built_at_once() {
    assert!(should_regenerate(None, &snap(1, 10.0, [1.0; 3]), 0.0));
}

#[test]
fn nothing_changed_means_no_rebuild_ever() {
    let s = snap(1, 10.0, [1.0, 1.0, 0.0]);
    for t in [0.5, 2.0, 60.0, 3600.0] {
        assert!(!should_regenerate(Some((&s, 0.0)), &s, t));
    }
}

#[test]
fn a_change_waits_for_the_throttle() {
    let a = snap(1, 10.0, [1.0, 1.0, 0.0]);
    let b = snap(2, 10.0, [1.0, 1.0, 0.0]);
    assert!(!should_regenerate(Some((&a, 10.0)), &b, 11.9), "within 2 s");
    assert!(should_regenerate(Some((&a, 10.0)), &b, 12.0), "after 2 s");
}

#[test]
fn the_sun_must_move_more_than_two_degrees() {
    let a = snap(1, 10.0, [1.0; 3]);
    assert!(!should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 11.9, [1.0; 3]),
        5.0
    ));
    assert!(!should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 8.0, [1.0; 3]),
        5.0
    ));
    assert!(should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 12.1, [1.0; 3]),
        5.0
    ));
    assert!(should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 7.9, [1.0; 3]),
        5.0
    ));
}

#[test]
fn a_weather_change_rebuilds_each_component() {
    let a = snap(1, 10.0, [1.0, 1.0, 0.0]);
    assert!(!should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 10.0, [0.99, 1.0, 0.0]),
        5.0
    ));
    for w in [[0.9, 1.0, 0.0], [1.0, 0.9, 0.0], [1.0, 1.0, 0.1]] {
        assert!(should_regenerate(Some((&a, 0.0)), &snap(1, 10.0, w), 5.0));
    }
}

#[test]
fn exposure_is_dark_at_night_and_scales_with_the_weather() {
    assert!((exposure(-30.0, 1.0) - NIGHT_EXPOSURE).abs() < 1e-6);
    assert!(exposure(-30.0, 1.0) < 0.05, "night does not glow");
    assert!((exposure(45.0, 1.0) - 1.0).abs() < 1e-6);
    assert!((exposure(45.0, 0.4) - 0.4).abs() < 1e-6, "overcast");
    assert!(exposure(0.0, 1.0) > exposure(-3.0, 1.0), "monotone at dawn");
    assert!(exposure(5.0, 1.0) < 1.0);
}
