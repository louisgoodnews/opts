//! Unit tests of `spec`.

use super::*;
use glam::Vec3;
use opts_core::formats::vehicles::bus::{BusManifest, VehicleExtras};

fn manifest(text: &str) -> BusManifest {
    BusManifest::parse(text).expect("parses")
}

#[test]
fn a_file_that_states_nothing_gets_the_defaults_and_says_so() {
    let spec = VehicleDynamicsSpec::from_manifest(&manifest("[friendlyname]\nA\nB\nC\n"));
    assert_eq!(spec, VehicleDynamicsSpec::default());
    assert_eq!(spec.mass_kg.source, Source::Default);
    assert_eq!(spec.cg_height_m.source, Source::Default);
    assert_eq!(spec.rolling_coefficient.source, Source::Default);
}

#[test]
fn the_stated_values_are_used_and_marked_as_file() {
    let spec = VehicleDynamicsSpec::from_manifest(&manifest(
        "[mass]\n1.2\n0\n0\n0\n\n[momentofintertia]\n0.4\n2.1\n2.2\n\n[schwerpunkt]\n0.55\n\n[rollwiderstand]\n100\n",
    ));
    assert_eq!(spec.mass_kg, Sourced::file(1_200.0));
    assert_eq!(spec.cg_height_m, Sourced::file(0.55));
    assert_eq!(spec.yaw_inertia_kg_m2, Sourced::file(2_200.0));
    // 100 N over 1.2 t: about 0.0085.
    let c = spec.rolling_coefficient;
    assert_eq!(c.source, Source::File);
    assert!(
        (c.value - 100.0 / (1_200.0 * 9.81)).abs() < 1e-6,
        "{}",
        c.value
    );
}

#[test]
fn the_legacy_inertia_of_the_mass_block_counts_when_the_modern_one_is_missing() {
    let legacy = VehicleDynamicsSpec::from_manifest(&manifest("[mass]\n8\n10\n250\n300\n"));
    assert_eq!(legacy.yaw_inertia_kg_m2, Sourced::file(300_000.0));
    // The modern block wins whatever its place in the file.
    let both = VehicleDynamicsSpec::from_manifest(&manifest(
        "[momentofintertia]\n1\n2\n3\n\n[mass]\n8\n10\n250\n300\n",
    ));
    assert_eq!(both.yaw_inertia_kg_m2.value, 3_000.0);
    // All zero lines are not a statement.
    let none = VehicleDynamicsSpec::from_manifest(&manifest("[mass]\n8\n0\n0\n0\n"));
    assert_eq!(none.yaw_inertia_kg_m2.source, Source::Default);
}

#[test]
fn without_inertia_the_bounding_box_gives_an_estimate() {
    let mut m = manifest("[mass]\n1.2\n0\n0\n0\n");
    m.bounding_box = Some((Vec3::new(1.7, 4.0, 1.4), Vec3::ZERO));
    let spec = VehicleDynamicsSpec::from_manifest(&m);
    assert_eq!(spec.yaw_inertia_kg_m2.source, Source::Estimate);
    let want = 1_200.0 * (1.7 * 1.7 + 4.0 * 4.0) / 12.0;
    assert!((spec.yaw_inertia_kg_m2.value - want).abs() < 1.0);
}

#[test]
fn implausible_values_fall_back_and_the_rolling_coefficient_is_bounded() {
    let mut m = manifest("[mass]\n8\n0\n0\n0\n");
    m.extras = VehicleExtras {
        centre_of_gravity_height: Some(0.0),
        roll_resistance_n: Some(1.0e7),
        ..VehicleExtras::default()
    };
    let spec = VehicleDynamicsSpec::from_manifest(&m);
    assert_eq!(
        spec.cg_height_m.source,
        Source::Default,
        "a height of 0 is no statement"
    );
    assert_eq!(spec.rolling_coefficient.value, ROLLING_COEFF_RANGE.1);
    // A force without a stated mass cannot become a coefficient.
    let mut m = manifest("[friendlyname]\nA\nB\nC\n");
    m.extras.roll_resistance_n = Some(900.0);
    assert_eq!(
        VehicleDynamicsSpec::from_manifest(&m)
            .rolling_coefficient
            .source,
        Source::Default
    );
}
