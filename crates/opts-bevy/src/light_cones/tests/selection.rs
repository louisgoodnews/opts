//! Selection under the cap.

use super::*;

fn candidates(n: u32) -> Vec<(Entity, f32)> {
    // Coverage falls with the index: cone 1 is the biggest.
    (1..=n).map(|i| (e(i), 1000.0 / i as f32)).collect()
}

#[test]
fn the_cap_limits_the_selection() {
    let c = candidates(200);
    assert!(select_cones(&c, &[], 0, CONE_HYSTERESIS).is_empty());
    let sixteen = select_cones(&c, &[], 16, CONE_HYSTERESIS);
    assert_eq!(sixteen, (1..=16).map(e).collect::<Vec<_>>());
    let all = select_cones(&c, &[], 96, CONE_HYSTERESIS);
    assert_eq!(all.len(), 96);
    assert_eq!(all[0], e(1));
    assert_eq!(all[95], e(96));
    // Fewer candidates than the cap.
    assert_eq!(select_cones(&c[..5], &[], 96, CONE_HYSTERESIS).len(), 5);
}

#[test]
fn a_cone_that_holds_a_slot_keeps_it_against_a_slightly_bigger_one() {
    // Cone 3 is the fourth by coverage, but holds a slot; cone 4 is 10 % bigger.
    let c = [(e(1), 10.0), (e(2), 9.0), (e(3), 8.0), (e(4), 8.8)];
    let first = select_cones(&c, &[e(3)], 3, CONE_HYSTERESIS);
    assert!(first.contains(&e(3)) && !first.contains(&e(4)));
    // Without the slot it would lose.
    assert!(!select_cones(&c, &[], 3, CONE_HYSTERESIS).contains(&e(3)));
    // A much bigger newcomer still wins.
    let c = [(e(1), 10.0), (e(2), 9.0), (e(3), 8.0), (e(4), 20.0)];
    let next = select_cones(&c, &[e(3)], 3, CONE_HYSTERESIS);
    assert!(next.contains(&e(4)));
    // Repeating the choice gives the same set.
    let mut a = select_cones(&c, &next, 3, CONE_HYSTERESIS);
    a.sort();
    let mut b = next;
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn coverage_grows_with_size_and_falls_with_distance() {
    assert!(cone_coverage(8.0, 4.0, 10.0) > cone_coverage(8.0, 4.0, 20.0));
    assert!(cone_coverage(30.0, 12.0, 50.0) > cone_coverage(8.0, 4.0, 50.0));
    assert!(cone_coverage(8.0, 4.0, 0.0).is_finite());
}
