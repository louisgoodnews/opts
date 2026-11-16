//! Unit tests for `super` (`moon.rs`).

use super::*;

mod harness;
mod scenarios;

#[test]
fn the_carrier_keeps_hysteresis_and_prefers_the_sun_on_ties() {
    assert_eq!(choose_carrier(None, 100.0, 100.0), Some(LightRole::Sun));
    assert_eq!(choose_carrier(None, 0.0, 0.0), None);
    assert_eq!(choose_carrier(None, 0.5, 0.2), None);
    assert_eq!(choose_carrier(None, 0.0, 5.0), Some(LightRole::Moon));
    // 14 % better is not enough, 16 % is.
    assert_eq!(
        choose_carrier(Some(LightRole::Sun), 100.0, 114.0),
        Some(LightRole::Sun)
    );
    assert_eq!(
        choose_carrier(Some(LightRole::Sun), 100.0, 116.0),
        Some(LightRole::Moon)
    );
    // A carrier that fell dark hands over to any light.
    assert_eq!(
        choose_carrier(Some(LightRole::Sun), 0.0, 5.0),
        Some(LightRole::Moon)
    );
    assert_eq!(choose_carrier(Some(LightRole::Sun), 0.0, 0.0), None);
}

#[test]
fn the_sun_fading_through_the_hand_over_flips_once() {
    // The sun's illuminance falls from 800 to 0 against a steady 900 lx moon: one switch.
    let mut cur = Some(LightRole::Sun);
    let mut switches = 0;
    for i in 0..=100 {
        let sun = 1500.0 * (1.0 - i as f32 / 100.0);
        let moon = 900.0 + if i % 2 == 0 { 40.0 } else { -40.0 };
        let next = choose_carrier(cur, sun, moon);
        if next != cur {
            switches += 1;
        }
        cur = next;
    }
    assert_eq!(switches, 1);
    assert_eq!(cur, Some(LightRole::Moon));
}

#[test]
fn the_disc_is_faint_by_day_and_dimmed_by_cloud() {
    assert_eq!(disc_brightness(-30.0, 1.0), 1.0);
    assert!((disc_brightness(40.0, 1.0) - MOON_DISC_DAY_BRIGHTNESS).abs() < 1e-6);
    assert!(disc_brightness(-30.0, 0.35) < 0.4);
}
