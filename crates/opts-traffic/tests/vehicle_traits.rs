//! Runtime comparison (M-0095 acceptance criterion 1): two AI vehicles of different mass and
//! brake force, under identical conditions, behave differently when their parameters come from
//! their traits, and identically under the old two-set choice (the negative control).
//!
//! The numbers are the declared values of real vehicles of the installation: a Waldhofen car
//! (`ai_hh109kipkw1`: 1.3 t, `AI_M_gas_max` 1000, `AI_F_brems_max` 18000, wheel diameter
//! 0.65 m) and the 14 t three-axle city bus `Citybus_L_AI` of Lucerne Line 24 (8000, 80000).

use glam::DVec2;
use opts_traffic::{
    AgentId, Calibration, ConnectParams, IdmParams, LaneGraph, LaneKind, LaneSegment, TrafficWorld,
    VehicleKind, VehicleTraits,
};

const STEP: f64 = 0.05;

fn car() -> VehicleTraits {
    VehicleTraits {
        kind: Some(VehicleKind::Car),
        length: 4.1,
        mass_kg: Some(1300.0),
        top_speed: Some(160.0 / 3.6),
        drive_torque: Some(1000.0),
        wheel_radius: Some(0.325),
        brake_force: Some(18_000.0),
        ..VehicleTraits::default()
    }
}

fn bus() -> VehicleTraits {
    VehicleTraits {
        kind: Some(VehicleKind::Truck),
        length: 11.0,
        mass_kg: Some(14_000.0),
        top_speed: Some(90.0 / 3.6),
        drive_torque: Some(8000.0),
        wheel_radius: Some(0.5),
        brake_force: Some(80_000.0),
        ..VehicleTraits::default()
    }
}

/// Two long straight lanes far apart, so the vehicles cannot influence each other.
fn world() -> (TrafficWorld, opts_traffic::LaneId, opts_traffic::LaneId) {
    let lane = |x: f64| LaneSegment::straight(DVec2::new(x, 0.0), 0.0, 4000.0, LaneKind::Road);
    let graph = LaneGraph::build(&[lane(0.0), lane(1000.0)], &ConnectParams::default());
    let ids: Vec<_> = graph.lane_ids().collect();
    (TrafficWorld::new(graph, 1), ids[0], ids[1])
}

fn params(t: &VehicleTraits) -> IdmParams {
    IdmParams::from_traits(t, &Calibration::default(), 0.5, None)
}

/// Seconds from rest to 40 km/h on an empty lane.
fn time_to_40_kmh(w: &mut TrafficWorld, lane: opts_traffic::LaneId, p: IdmParams) -> f64 {
    let id = w.spawn(lane, 10.0, 0.0, p).expect("spawn");
    for k in 1..4000 {
        w.step(STEP);
        if w.agent(id).is_some_and(|a| a.speed >= 40.0 / 3.6) {
            return f64::from(k) * STEP;
        }
    }
    panic!("never reached 40 km/h");
}

/// Metres travelled while braking for a parked vehicle, from 50 km/h: from the first step that
/// decelerates by more than 0.5 m/s² until standstill.
fn braking_distance(w: &mut TrafficWorld, lane: opts_traffic::LaneId, p: IdmParams) -> f64 {
    let parked = IdmParams {
        desired_speed: 0.1,
        ..IdmParams::default()
    };
    w.spawn(lane, 400.0, 0.0, parked).expect("parked");
    let id: AgentId = w.spawn(lane, 10.0, 50.0 / 3.6, p).expect("follower");
    let mut start = None;
    for _ in 0..4000 {
        w.step(STEP);
        let a = w.agent(id).expect("alive");
        if start.is_none() && a.accel < -0.5 {
            start = Some(a.s);
        }
        if start.is_some() && a.speed < 0.2 {
            return a.s - start.unwrap_or(a.s);
        }
    }
    panic!("never stopped");
}

#[test]
fn a_light_car_and_a_heavy_bus_accelerate_and_brake_differently() {
    let (mut w, a, b) = world();
    let (t_car, t_bus) = (
        time_to_40_kmh(&mut w, a, params(&car())),
        time_to_40_kmh(&mut w, b, params(&bus())),
    );
    assert!(
        t_car <= t_bus * 0.7,
        "the car must reach 40 km/h at least 30 % sooner: car {t_car:.1} s, bus {t_bus:.1} s"
    );
    let (mut w, a, b) = world();
    let (d_car, d_bus) = (
        braking_distance(&mut w, a, params(&car())),
        braking_distance(&mut w, b, params(&bus())),
    );
    // The bus carries 4.4 N of brake force per kg against the car's 13.8 and brakes earlier and
    // more gently, so its braking takes longer.
    eprintln!("0-40 km/h: car {t_car:.1} s, bus {t_bus:.1} s; braking from 50 km/h: car {d_car:.1} m, bus {d_bus:.1} m");
    assert!(
        d_bus > d_car * 1.1,
        "braking distance from 50 km/h: car {d_car:.1} m, bus {d_bus:.1} m"
    );
}

#[test]
fn negative_control_the_old_two_set_choice_cannot_tell_them_apart() {
    // The previous spawner gave every vehicle of at most 7.5 m `IdmParams::default()`; two cars
    // of different mass and brake force therefore behaved identically.
    let (mut w, a, b) = world();
    let old = IdmParams::default();
    let t1 = time_to_40_kmh(&mut w, a, old);
    let t2 = time_to_40_kmh(&mut w, b, old);
    assert_eq!(t1, t2, "identical parameters give identical behaviour");
    let heavy_car = VehicleTraits {
        mass_kg: Some(2600.0),
        brake_force: Some(10_000.0),
        ..car()
    };
    let (mut w, a, b) = world();
    let light = time_to_40_kmh(&mut w, a, params(&car()));
    let heavy = time_to_40_kmh(&mut w, b, params(&heavy_car));
    assert!(
        heavy > light * 1.2,
        "twice the mass takes longer: {light:.1} s vs {heavy:.1} s"
    );
}

#[test]
fn a_slow_vehicle_keeps_to_its_declared_top_speed() {
    let scooter = VehicleTraits {
        top_speed: Some(15.0 / 3.6),
        mass_kg: Some(400.0),
        drive_torque: Some(400.0),
        brake_force: Some(10_000.0),
        ..car()
    };
    let (mut w, a, _) = world();
    let id = w.spawn(a, 10.0, 0.0, params(&scooter)).expect("spawn");
    for _ in 0..2000 {
        w.step(STEP);
    }
    let v = w.agent(id).expect("alive").speed;
    assert!(
        v <= 15.0 / 3.6 + 0.05 && v > 3.5,
        "cruises at its top speed, not 50 km/h: {v}"
    );
}
