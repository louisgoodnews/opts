//! Tests of the audited OMSI-to-PBR mapping.

use super::*;
use crate::material::{specular_power_to_roughness, specular_to_reflectance};

#[test]
fn a_black_specular_colour_is_matte_whatever_the_power() {
    assert_eq!(audited_roughness(100.0, 0.0), 1.0);
    assert_eq!(audited_roughness(10_000.0, 0.0), 1.0);
}

#[test]
fn a_white_specular_colour_keeps_the_legacy_roughness() {
    let white = specular_luma([1.0, 1.0, 1.0]);
    assert!((white - 1.0).abs() < 1e-4);
    for power in [0.0, 16.0, 50.0, 200.0] {
        let r = audited_roughness(power, white);
        assert!(
            (r - specular_power_to_roughness(power)).abs() < 1e-6,
            "power {power}: {r}"
        );
    }
}

#[test]
fn roughness_falls_monotonically_as_specular_rises() {
    let mut last = 1.0_f32;
    for step in 0..=10 {
        let luma = FULL_SPECULAR_LUMA * step as f32 / 10.0;
        let r = audited_roughness(100.0, luma);
        assert!(r <= last, "roughness rose at luma {luma}: {last} -> {r}");
        last = r;
    }
}

#[test]
fn audited_roughness_stays_in_the_unit_interval() {
    for power in [0.0, 1.0, 50.0, 1e6] {
        for luma in [0.0, 0.1, 0.5, 1.0] {
            let r = audited_roughness(power, luma);
            assert!((0.0..=1.0).contains(&r), "power {power} luma {luma} -> {r}");
        }
    }
}

#[test]
fn reflectance_is_zero_for_black_and_capped_at_bevy_half() {
    assert_eq!(audited_reflectance(0.0), 0.0);
    assert_eq!(audited_reflectance(0.5), 0.5);
    assert_eq!(audited_reflectance(3.0), 0.5);
}

#[test]
fn white_specular_gives_the_same_reflectance_as_the_legacy_mapping() {
    let white = [1.0, 1.0, 1.0];
    let luma = specular_luma(white);
    assert!((audited_reflectance(luma) - specular_to_reflectance(white)).abs() < 1e-4);
}

#[test]
fn smoothstep_is_clamped_and_hits_both_ends() {
    assert_eq!(smoothstep(0.0, 1.0, -5.0), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 5.0), 1.0);
    assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
}
