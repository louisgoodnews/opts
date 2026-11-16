//! Clearance: what blocks a lane and what may spawn on it.

use super::*;

#[test]
fn a_vehicle_next_to_the_lane_does_not_block_it() {
    let mut w = world(&[straight(0.0, 800.0)]);
    // 3.4 m beside the lane: the bus' side is 2.1 m from the lane's centre line, the car's
    // half width 0.95 m: it passes with room to spare.
    w.set_obstacles(vec![bus_at(3.4, 200.0)]);
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..400 {
        w.step(0.1);
    }
    assert!(w.agent(id).is_some_and(|a| a.s > 250.0 && a.speed > 5.0));
}

#[test]
fn a_vehicle_driving_ahead_is_followed_not_overtaken_through() {
    let mut w = world(&[straight(0.0, 2000.0)]);
    let id = w
        .spawn(LaneId(0), 20.0, 14.0, IdmParams::default())
        .unwrap();
    let mut y = 60.0;
    for _ in 0..300 {
        // The player drives at 5 m/s ahead.
        y += 5.0 * 0.1;
        w.set_obstacles(vec![Obstacle {
            velocity: DVec2::new(0.0, 5.0),
            ..bus_at(0.0, y)
        }]);
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert!(
        a.s < y - 6.0,
        "the car ran into the bus: s = {}, bus at {y}",
        a.s
    );
    assert!(
        (a.speed - 5.0).abs() < 1.5,
        "it should follow at the bus' speed, not {}",
        a.speed
    );
}

#[test]
fn nothing_is_spawned_inside_a_standing_vehicle() {
    let mut w = world(&[straight(0.0, 800.0)]);
    w.set_obstacles(vec![bus_at(0.0, 200.0)]);
    assert_eq!(
        w.spawn(LaneId(0), 200.0, 0.0, IdmParams::default())
            .unwrap_err(),
        SpawnError::Occupied,
        "inside the bus"
    );
    assert_eq!(
        w.spawn(LaneId(0), 197.0, 0.0, IdmParams::default())
            .unwrap_err(),
        SpawnError::Occupied,
        "with its nose in the bus' rear"
    );
    assert!(
        w.spawn(LaneId(0), 120.0, 0.0, IdmParams::default()).is_ok(),
        "far behind it"
    );
    assert!(
        w.spawn(LaneId(0), 260.0, 0.0, IdmParams::default()).is_ok(),
        "far in front"
    );
}
