//! Tests: how a vehicle file's `[coupling_front_character]` reaches the articulated segment
//! (M-0053 Part 4): declared limits are taken over, absent ones give the documented defaults.

use super::*;

/// A manifest with the two coupling points and, when given, the limits block
/// (`max_alpha`, `min_beta`, `max_beta`, `kind`).
fn manifest(limits: Option<[&str; 4]>) -> BusManifest {
    let mut text = String::from("[coupling_front]\n0\n1\n0\n\n[coupling_back]\n0\n-5\n0\n\n");
    if let Some([a, lo, hi, kind]) = limits {
        text.push_str(&format!(
            "[coupling_front_character]\n{a}\n{lo}\n{hi}\n{kind}\n"
        ));
    }
    BusManifest::parse(&text).expect("manifest")
}

fn part(manifest: BusManifest) -> PlannedPart {
    PlannedPart {
        path: PathBuf::new(),
        dir: PathBuf::new(),
        manifest,
        turned: false,
        behind: 10.0,
        powered: false,
        joint: Some((Vec3::new(0.0, -5.0, 0.0), Vec3::new(0.0, 1.0, 0.0))),
    }
}

fn segment(manifest: BusManifest) -> opts_bevy::ArticulatedSegment {
    let load = opts_sim::UnitLoad::from_manifest_or(&manifest, false, 8_000.0);
    articulated_segment(&part(manifest.clone()), &manifest, &load, 2)
}

#[test]
fn declared_limits_are_enforced() {
    let s = segment(manifest(Some(["40", "-15", "25", "1"])));
    assert_eq!(s.max_alpha_deg, 40.0);
    assert_eq!(s.beta_limits_deg, Some((-15.0, 25.0)));
    assert_eq!(s.clamp_beta_deg(60.0), 25.0);
    assert_eq!(s.clamp_beta_deg(-60.0), -15.0);
    assert_eq!(s.clamp_beta_deg(3.0), 3.0);
}

#[test]
fn limits_listed_in_reverse_order_are_sorted() {
    // Three installed files state `20, -20` (max first) instead of `-20, 20`.
    let s = segment(manifest(Some(["52.5", "20", "-20", "1"])));
    assert_eq!(s.beta_limits_deg, Some((-20.0, 20.0)));
    assert_eq!(s.clamp_beta_deg(35.0), 20.0);
    assert_eq!(s.clamp_beta_deg(-35.0), -20.0);
}

#[test]
fn a_vehicle_without_limits_gets_the_defaults() {
    let s = segment(manifest(None));
    assert_eq!(s.max_alpha_deg, 52.5, "documented default for |alpha|");
    assert_eq!(s.beta_limits_deg, None, "beta is unlimited");
    assert_eq!(s.clamp_beta_deg(75.0), 75.0);
    assert_eq!(s.clamp_beta_deg(-75.0), -75.0);
}

#[test]
fn an_incomplete_limits_block_counts_as_absent() {
    // Only three of the four values: the parser rejects the block, the defaults apply.
    let text = "[coupling_front_character]\n40\n-15\n25\n";
    let s = segment(BusManifest::parse(text).expect("manifest"));
    assert_eq!(s.max_alpha_deg, 52.5);
    assert_eq!(s.beta_limits_deg, None);
}

#[test]
fn the_traffic_loader_uses_the_same_alpha_default() {
    // The AI chain reads the same block; both loaders must agree on the default.
    let declared = manifest(Some(["33", "-10", "10", "1"]));
    assert_eq!(
        declared.coupling_front_limits.map(|l| l.max_alpha_deg),
        Some(33.0)
    );
    assert!(manifest(None).coupling_front_limits.is_none());
}
