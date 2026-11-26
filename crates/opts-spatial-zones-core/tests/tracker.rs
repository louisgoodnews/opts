//! Unit tests of the hysteresis tracker and the sample gate (moved with the tracker, ISS-092).

use opts_spatial_zones_core::tracker::*;

/// Feeds `n` samples of `advance_m` each in state `covered`, collecting the transitions.
fn drive(t: &mut ZoneTracker, covered: bool, n: usize, advance_m: f32) -> Vec<Transition> {
    (0..n)
        .filter_map(|_| t.update(covered, advance_m))
        .collect()
}

#[test]
fn enters_only_after_the_covered_distance_threshold() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    assert!(drive(&mut t, true, 7, 2.0).is_empty(), "14 m is below 15 m");
    assert_eq!(t.update(true, 2.0), Some(Transition::Entered));
    assert!(t.inside());
}

#[test]
fn leaves_only_after_the_uncovered_distance_threshold() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    drive(&mut t, true, 8, 2.0);
    assert!(t.inside());
    assert!(drive(&mut t, false, 2, 2.0).is_empty(), "4 m is below 5 m");
    assert_eq!(t.update(false, 2.0), Some(Transition::Left));
    assert!(!t.inside());
}

#[test]
fn short_uncovered_gap_inside_does_not_flap() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    drive(&mut t, true, 8, 2.0);
    // A 4 m gap in the roof, then coverage again: no leave, no second enter.
    assert!(drive(&mut t, false, 2, 2.0).is_empty());
    assert!(drive(&mut t, true, 10, 2.0).is_empty());
    assert!(t.inside());
}

#[test]
fn short_covered_blip_outside_does_not_enter() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    assert!(drive(&mut t, true, 3, 2.0).is_empty(), "a 6 m canopy");
    assert!(drive(&mut t, false, 5, 2.0).is_empty());
    assert!(!t.inside());
}

#[test]
fn one_crossing_emits_exactly_one_enter_and_one_leave() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    let mut seen = drive(&mut t, true, 20, 1.0);
    seen.extend(drive(&mut t, false, 20, 1.0));
    assert_eq!(seen, vec![Transition::Entered, Transition::Left]);
}

#[test]
fn stationary_subject_never_changes_state() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    assert!(drive(&mut t, true, 100, 0.0).is_empty());
    assert!(!t.inside());
}

#[test]
fn negative_advance_counts_as_zero() {
    let mut t = ZoneTracker::new(15.0, 5.0);
    assert!(drive(&mut t, true, 100, -3.0).is_empty());
}

#[test]
fn gate_fires_at_ten_hertz_and_drops_backlog() {
    let mut g = SampleGate::new();
    assert!(g.tick(0.0), "first call is due");
    assert!(!g.tick(0.05));
    assert!(g.tick(0.05), "0.1 s after the last sample");
    // A 1 s stall yields one sample, not ten.
    assert!(g.tick(1.0));
    assert!(!g.tick(0.05));
}
