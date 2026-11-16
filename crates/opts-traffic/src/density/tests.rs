//! Unit tests for density.

use super::*;

#[test]
fn curve_interpolates_and_wraps() {
    let c = DensityCurve::new(vec![(24.0, 0.1), (0.0, 0.1), (6.0, 1.0)]);
    assert!((c.at(3.0) - 0.55).abs() < 1e-6);
    assert!((c.at(12.0) - 0.7).abs() < 0.31); // between 1.0 at 6 and 0.1 at 24
    assert!((c.at(24.0) - 0.1).abs() < 1e-6 && (c.at(48.0) - 0.1).abs() < 1e-6);
    assert_eq!(DensityCurve::default().at(5.0), 1.0);
}

#[test]
fn group_density_follows_rule_then_default_chain() {
    // Berlin-Spandau: NormalCars 1, Trucks 0, Commercials 1, Ambulance 1, GDRCars 0.
    let defaults = [1, 0, 1, 1, 0];
    let mut lane = LaneDensity::default();
    assert_eq!(lane.for_group(0, &defaults), MEDIUM_DENSITY);
    assert_eq!(lane.for_group(1, &defaults), 0.0, "Trucks: default 0");
    assert_eq!(
        lane.for_group(2, &defaults),
        MEDIUM_DENSITY,
        "via NormalCars"
    );
    assert_eq!(lane.for_group(4, &defaults), 0.0, "GDRCars: default 0");
    assert_eq!(lane.for_group(7, &defaults), 0.0, "no such group");
    lane.set(0, 0.5);
    lane.set(4, 3.0);
    assert_eq!(
        lane.for_group(2, &defaults),
        0.5,
        "Commercials follow NormalCars' rule"
    );
    assert_eq!(lane.for_group(4, &defaults), 3.0, "unclamped");
    assert!(!lane.closed_for(4) && !lane.closed_for(1));
    lane.set(0, 0.0);
    assert!(lane.closed_for(0) && !lane.closed_for(1));
    // Only the rule of the named group counts; a later rule for it wins.
    lane.set(4, 1.0);
    assert_eq!(lane.iter().collect::<Vec<_>>(), vec![(0, 0.0), (4, 1.0)]);
    assert!(!lane.set(MAX_DENSITY_GROUPS, 1.0));
    // A cycle of defaults ends in Medium.
    assert_eq!(LaneDensity::default().for_group(0, &[2, 1]), MEDIUM_DENSITY);
}

#[test]
fn targets_scale_proportionally_at_the_cap() {
    let (t, capped) = group_targets(&[30.0, 10.0, 0.0], 100);
    assert_eq!((t, capped), (vec![30.0, 10.0, 0.0], false));
    let (t, capped) = group_targets(&[150.0, 50.0], 100);
    assert!(capped);
    assert!((t[0] - 75.0).abs() < 1e-9 && (t[1] - 25.0).abs() < 1e-9);
    assert!((t[0] / t[1] - 3.0).abs() < 1e-9, "shares kept");
}

#[test]
fn picks_follow_the_weights() {
    let mut rng = Rng::new(42);
    let w = [Some(7.0), Some(3.0), Some(0.0)];
    let mut counts = [0usize; 3];
    for _ in 0..10_000 {
        counts[weighted_pick(&w, &mut rng).unwrap()] += 1;
    }
    assert_eq!(counts[2], 0);
    let share = counts[0] as f64 / 10_000.0;
    assert!((share - 0.7).abs() < 0.03, "share {share}");
}

#[test]
fn missing_weights_count_as_one_and_empty_lists_pick_nothing() {
    let mut rng = Rng::new(1);
    assert!(weighted_pick(&[None, None], &mut rng).is_some());
    assert_eq!(weighted_pick(&[], &mut rng), None);
    assert_eq!(weighted_pick(&[Some(0.0)], &mut rng), None);
}
