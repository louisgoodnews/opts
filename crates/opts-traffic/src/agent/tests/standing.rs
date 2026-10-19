//! Standing vehicles, stuck despawn and spawn room.

use super::conflicts::crossing;
use super::lane_change::two_lanes;
use super::*;

/// A bus standing across lane 0 at `y`, heading north.
fn bus_at(x: f64, y: f64) -> Obstacle {
    Obstacle {
        centre: DVec2::new(x, y),
        heading: 0.0,
        length: 12.0,
        width: 2.55,
        velocity: DVec2::ZERO,
    }
}

#[test]
fn the_first_vehicle_stuck_is_despawned_with_an_event() {
    let mut w = world(&[straight(0.0, 800.0)]);
    w.set_stuck_after(STUCK_AFTER_S);
    w.set_obstacles(vec![bus_at(0.0, 200.0)]);
    let first = w
        .spawn(LaneId(0), 150.0, 0.0, IdmParams::default())
        .unwrap();
    let second = w
        .spawn(LaneId(0), 100.0, 0.0, IdmParams::default())
        .unwrap();
    let mut stuck = Vec::new();
    for _ in 0..((STUCK_AFTER_S + 60.0) * 10.0) as usize {
        w.step(0.1);
        for e in w.drain_events() {
            if let TrafficEvent::Stuck { id, .. } = e {
                stuck.push(id);
            }
        }
    }
    // Both drive up to the bus and stand; the one that reaches the limit first
    // goes, and the event names it.
    assert!(!stuck.is_empty());
    assert!(stuck.contains(&first) || stuck.contains(&second));
    assert!(stuck.iter().all(|id| w.agent(*id).is_none()));
}

#[test]
fn a_car_stops_behind_a_standing_vehicle_on_its_lane_and_phantoms_do_not_show() {
    let mut w = world(&[straight(0.0, 800.0)]);
    w.set_obstacles(vec![bus_at(0.0, 200.0)]); // rear end at y = 194
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..600 {
        w.step(0.1);
        assert!(w.agents().iter().all(|a| !a.foreign), "a phantom leaked");
    }
    let a = w.agent(id).unwrap();
    assert!(a.speed < 0.1, "it still drives at {} m/s", a.speed);
    // Its front bumper stays short of the bus' rear end by about the minimum gap.
    assert!(
        a.s < 194.0 - 0.5,
        "front bumper at {} m touches the bus",
        a.s
    );
    assert!(a.s > 194.0 - 6.0, "it stopped far too early at {} m", a.s);
}

#[test]
fn a_car_changes_to_a_free_lane_round_a_standing_vehicle() {
    let mut w = two_lanes();
    w.set_obstacles(vec![bus_at(0.0, 300.0)]);
    let id = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    let mut changed = false;
    let mut worst = 0.0f64;
    for _ in 0..500 {
        w.step(0.1);
        worst = worst.min(w.agent(id).map_or(0.0, |a| a.accel));
        changed |= w
            .drain_events()
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id: i, .. } if *i == id));
    }
    assert!(changed, "the car never changed lanes");
    let a = w.agent(id).unwrap();
    assert!(a.s > 330.0, "it did not get past the bus: s = {}", a.s);
    assert!(worst > -4.5, "it braked at {worst} m/s²");
}

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

#[test]
fn the_deadlock_breaker_does_not_drive_through_a_standing_vehicle_in_a_junction() {
    let mut w = crossing(LaneRules::default(), LaneRules::default());
    // The bus stands across the north-bound lane at the crossing.
    w.set_obstacles(vec![Obstacle {
        centre: DVec2::new(0.0, 100.0),
        heading: std::f64::consts::FRAC_PI_2, // lying across the road
        length: 12.0,
        width: 2.55,
        velocity: DVec2::ZERO,
    }]);
    let id = w
        .spawn(LaneId(0), 40.0, 10.0, IdmParams::default())
        .unwrap();
    // Far longer than the deadlock timeout.
    for _ in 0..1200 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert!(a.speed < 0.3, "still moving at {} m/s", a.speed);
    assert!(
        a.s < 100.0 - 1.0,
        "it drove into the bus: front at {} m",
        a.s
    );
}
