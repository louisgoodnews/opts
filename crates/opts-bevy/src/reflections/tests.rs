//! Unit tests for `super` (`reflections.rs`).

use super::*;

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
