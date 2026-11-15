//! Right-of-way between crossing flows: right before left and priority roads.

use super::*;

#[test]
fn right_before_left_the_vehicle_from_the_right_goes_first() {
    let mut w = crossing(LaneRules::default(), LaneRules::default());
    // Both reach the crossing together; the north-bound one has the other on its right.
    w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let (slowest, closest) = run_crossing(&mut w, 150);
    assert!(
        slowest[1] > 10.0,
        "the vehicle from the right slowed to {}",
        slowest[1]
    );
    assert!(
        slowest[0] < 6.0,
        "the vehicle from the left did not give way ({})",
        slowest[0]
    );
    assert!(closest > 1.4, "bodies came within {closest} m");
}

#[test]
fn a_priority_road_goes_first_even_when_the_side_road_arrives_earlier() {
    let high = LaneRules {
        priority: Some(192),
        ..LaneRules::default()
    };
    let low = LaneRules {
        priority: Some(64),
        ..LaneRules::default()
    };
    // The west-bound vehicle is on the left-hand priority... flip: lane 0 is the priority road
    // although lane 1 comes from its right, and lane 1 is 10 m closer to the crossing.
    let mut w = crossing(high, low);
    w.spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let (slowest, closest) = run_crossing(&mut w, 150);
    assert!(
        slowest[0] > 10.0,
        "the priority road was held up (slowest {})",
        slowest[0]
    );
    assert!(
        slowest[1] < 6.0,
        "the side road did not give way ({})",
        slowest[1]
    );
    assert!(closest > 1.4, "bodies came within {closest} m");
}

#[test]
fn vehicles_without_right_of_way_do_not_wait_for_far_away_traffic() {
    let mut w = crossing(LaneRules::default(), LaneRules::default());
    // The vehicle from the right is 150 m away (12 s): the north-bound one just drives on.
    w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    w.spawn(LaneId(1), 0.0, 12.0, IdmParams::default()).unwrap();
    let (slowest, _) = run_crossing(&mut w, 80);
    assert!(slowest[0] > 10.0, "slowed to {}", slowest[0]);
}
