//! Tests of the speed policy (M-0114): the blend, the look-ahead and the regime label.

use super::*;

#[test]
fn blend_is_zero_when_slow_and_one_when_fast() {
    assert_eq!(speed_blend(0.0), 0.0);
    assert_eq!(speed_blend(SLOW_MPS), 0.0);
    assert_eq!(speed_blend(FAST_MPS), 1.0);
    assert_eq!(speed_blend(40.0), 1.0);
}

#[test]
fn blend_is_monotonic_and_continuous() {
    let mut last = 0.0;
    for step in 0..=300 {
        let v = step as f32 * 0.1;
        let b = speed_blend(v);
        assert!(b >= last - 1e-6, "not monotonic at {v} m/s");
        last = b;
    }
    // no jump: 0.01 m/s apart differ by far less than the whole range
    assert!((speed_blend(6.5) - speed_blend(6.51)).abs() < 0.01);
}

#[test]
fn non_finite_speed_counts_as_stopped() {
    assert_eq!(speed_blend(f32::NAN), 0.0);
    assert_eq!(speed_blend(f32::INFINITY), 0.0);
}

#[test]
fn ninety_kmh_is_fully_tile_regime() {
    let v = 90.0 / 3.6;
    assert_eq!(speed_blend(v), 1.0);
    assert_eq!(lookahead(speed_blend(v)), MAX_LOOKAHEAD);
}

#[test]
fn forced_modes_ignore_the_speed() {
    assert_eq!(effective_blend(StreamMode::Tile, 0.0), 1.0);
    assert_eq!(effective_blend(StreamMode::Object, 30.0), 0.0);
    assert_eq!(effective_blend(StreamMode::Auto, 30.0), 1.0);
}

#[test]
fn mode_setting_parses_and_defaults_to_auto() {
    assert_eq!(StreamMode::from_setting(Some(" tile ")), StreamMode::Tile);
    assert_eq!(StreamMode::from_setting(Some("object")), StreamMode::Object);
    assert_eq!(StreamMode::from_setting(Some("auto")), StreamMode::Auto);
    assert_eq!(StreamMode::from_setting(Some("bogus")), StreamMode::Auto);
    assert_eq!(StreamMode::from_setting(None), StreamMode::Auto);
    assert_eq!(StreamMode::Tile.name(), "tile");
}

#[test]
fn lookahead_and_heading_grow_with_the_blend() {
    assert_eq!(lookahead(0.0), 0);
    assert_eq!(lookahead(0.5), 1);
    assert_eq!(lookahead(1.0), 2);
    assert_eq!(lookahead(7.0), 2, "blend is clamped");
    assert_eq!(heading_weight(0.0), 0.0);
    assert!((heading_weight(1.0) - MAX_HEADING_WEIGHT).abs() < 1e-6);
}

#[test]
fn hysteresis_covers_the_lookahead() {
    assert_eq!(hysteresis(0), MIN_HYSTERESIS);
    assert_eq!(hysteresis(2), 2);
    assert!(hysteresis(lookahead(1.0)) >= lookahead(1.0));
}

#[test]
fn label_does_not_flicker_between_the_thresholds() {
    let mid = (ENTER_TILE + LEAVE_TILE) / 2.0;
    assert_eq!(latch(Regime::Object, mid), Regime::Object);
    assert_eq!(latch(Regime::Tile, mid), Regime::Tile);
    assert_eq!(latch(Regime::Object, ENTER_TILE), Regime::Tile);
    assert_eq!(latch(Regime::Tile, LEAVE_TILE), Regime::Object);
    assert_eq!(Regime::Tile.name(), "tile");
}
