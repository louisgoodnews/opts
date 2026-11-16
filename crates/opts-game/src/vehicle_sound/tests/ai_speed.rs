//! Engine speed, gear changes, and how a car differs from a bus.

use super::*;

#[test]
fn the_engine_speed_rises_drops_at_a_shift_and_rises_again() {
    assert_eq!(ai_engine_rpm(0.0, true), 650.0);
    assert!(ai_engine_rpm(6.0, true) > 650.0 && ai_engine_rpm(11.9, true) < 2100.1);
    let before = ai_engine_rpm(11.99, true);
    let after = ai_engine_rpm(12.1, true);
    assert!(after < before - 500.0, "{before} -> {after}");
    assert!(ai_engine_rpm(20.0, true) > after);
    assert!(ai_engine_rpm(300.0, true) <= 3200.0);
    let mut last = 0.0;
    for k in 0..=120 {
        let v = ai_engine_rpm(k as f32, true);
        assert!((600.0..=3200.0).contains(&v));
        last = v;
    }
    assert!(last > 650.0);
}

#[test]
fn a_car_idles_higher_and_revs_further_than_a_bus_and_the_gears_follow_the_speed() {
    assert_eq!(ai_engine_rpm(0.0, false), 800.0);
    assert!(ai_engine_rpm(15.0, false) > ai_engine_rpm(15.0, true));
    // Both shift: the speed drops at a gear change.
    for heavy in [true, false] {
        let before = ai_engine_rpm(if heavy { 11.99 } else { 17.99 }, heavy);
        let after = ai_engine_rpm(if heavy { 12.01 } else { 18.01 }, heavy);
        assert!(after < before - 400.0, "heavy {heavy}: {before} -> {after}");
    }
    assert_eq!(ai_gear(0.0, true), 0.0);
    assert_eq!(ai_gear(20.0, true), 2.0);
    assert_eq!(ai_gear(20.0, false), 2.0);
    assert_eq!(ai_gear(50.0, false), 3.0);
}
