//! Road grip in weather (M-0117): the tyres' friction limit scales the braking distance, and a
//! change of the grip while driving does not jump the vehicle's speed.

use super::*;
use crate::surface_grip::{approach, MU_SNOW, MU_WET};

const DT: f32 = 1.0 / 64.0;

/// Distance (m) of a full service brake from 50 km/h with the given friction coefficient.
fn stopping_distance_m(mu: f32) -> f32 {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.road_friction = mu;
    sim.chassis.local_velocity.y = 14.0;
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 0.0,
        brake_force_n: 1.0e9,
    });
    sim.inputs.set_service_brake(1.0);
    let mut distance = 0.0_f32;
    for _ in 0..2000 {
        let before = sim.chassis.position.y;
        sim.step(DT);
        distance += sim.chassis.position.y - before;
        if sim.chassis.forward_speed_m_s() <= 0.0 {
            break;
        }
    }
    distance
}

/// Acceptance 1 of M-0117: wet at least 20 % and snow at least 100 % longer than dry.
#[test]
fn wet_and_snow_lengthen_the_braking_distance() {
    let dry = stopping_distance_m(ROAD_FRICTION);
    let wet = stopping_distance_m(MU_WET);
    let snow = stopping_distance_m(MU_SNOW);
    assert!(wet >= 1.2 * dry, "wet {wet} vs dry {dry}");
    assert!(snow >= 2.0 * dry, "snow {snow} vs dry {dry}");
    // Braking distance scales with 1/μ: the ratio stays within 10 % of the coefficients' ratio.
    let ideal = ROAD_FRICTION / MU_SNOW;
    assert!(
        (snow / dry - ideal).abs() < 0.1 * ideal,
        "{} vs {ideal}",
        snow / dry
    );
}

/// Acceptance 2 of M-0117: a change to snow while braking is rate-limited, so the speed change
/// per step does not change abruptly.
#[test]
fn a_grip_change_while_braking_does_not_jump_the_speed() {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.chassis.local_velocity.y = 14.0;
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 0.0,
        brake_force_n: 1.0e9,
    });
    sim.inputs.set_service_brake(1.0);
    // A quarter second of braking on dry road lets the brake pressure build up first; the
    // vehicle stays above standstill over the window (the stop itself is a clamp, not grip).
    for _ in 0..16 {
        sim.step(DT);
    }
    let mut mu = ROAD_FRICTION;
    let mut previous_dv = None::<f32>;
    let mut worst_change = 0.0_f32;
    for step in 0..64 {
        let target = if step < 8 { ROAD_FRICTION } else { MU_SNOW };
        mu = approach(mu, target, DT);
        sim.road_friction = mu;
        let before = sim.chassis.forward_speed_m_s();
        sim.step(DT);
        let dv = sim.chassis.forward_speed_m_s() - before;
        if let Some(prev) = previous_dv {
            worst_change = worst_change.max((dv - prev).abs());
        }
        previous_dv = Some(dv);
    }
    assert!(
        worst_change < 0.1,
        "speed change jumped by {worst_change} m/s in one step"
    );
    assert!(sim.chassis.forward_speed_m_s() > 0.0);
}

/// A save from before the field loads with the dry road's friction.
#[test]
fn a_save_without_the_friction_field_loads_as_dry_road() {
    let sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    let mut value = serde_json::to_value(&sim).expect("serialise");
    value
        .as_object_mut()
        .expect("object")
        .remove("road_friction");
    let loaded: VehicleSimulation = serde_json::from_value(value).expect("deserialise");
    assert_eq!(loaded.road_friction, ROAD_FRICTION);
}
