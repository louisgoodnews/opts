//! Unit tests for `super` (`reflections.rs`).

use super::*;
use crate::WeatherEffects;

#[test]
fn default_switches_are_all_off() {
    let s = ReflectionSwitches::parse("");
    assert!(!s.any());
}

#[test]
fn switch_list_parses_and_ignores_the_promoted_ones() {
    let s = ReflectionSwitches::parse("env,glass=trans,mirror,mirror_size=128x64,wet");
    assert!(s.wet && !s.ssr);
    assert!(ReflectionSwitches::parse("ssr").ssr);
}

#[test]
fn wet_standard_is_identity_when_dry_and_darker_when_wet() {
    let (c, r) = wet_standard(Color::linear_rgb(0.5, 0.5, 0.5), 0.8, 0.0);
    assert!((c.to_linear().red - 0.5).abs() < 1e-6 && (r - 0.8).abs() < 1e-6);
    let (c, r) = wet_standard(Color::linear_rgb(0.5, 0.5, 0.5), 0.8, 1.0);
    assert!(c.to_linear().red < 0.5 && r < 0.8);
}

#[test]
fn f16_encodes_one() {
    assert_eq!(f16_bits(1.0), 0x3c00);
    assert_eq!(f16_bits(0.0), 0);
}

#[test]
fn only_moisture_materials_take_the_wetness() {
    assert_eq!(material_wetness(true, 0.7), 0.7);
    assert_eq!(material_wetness(true, 3.0), 1.0);
    // A texture without the flag stays dry at any wetness.
    assert_eq!(material_wetness(false, 1.0), 0.0);
}

#[test]
fn wet_objects_follow_the_switch_or_the_setting() {
    let off = ReflectionSwitches::default();
    let on = ReflectionSwitches::parse("wet");
    let ground = WeatherEffects::default();
    let objects = WeatherEffects {
        wet_objects: true,
        ..WeatherEffects::default()
    };
    assert!(!wet_objects_on(&off, None));
    assert!(!wet_objects_on(&off, Some(&ground)));
    assert!(wet_objects_on(&off, Some(&objects)));
    assert!(wet_objects_on(&on, None));
}
