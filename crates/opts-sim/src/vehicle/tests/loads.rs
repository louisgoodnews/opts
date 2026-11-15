//! Unit tests of the vehicle simulation: torque and brake limits, hitch force, load serialisation.

use super::*;

/// Design V5 / acceptance 2: with the same wheel torque the acceleration scales with
/// `1 / Σm` within 2 %. The traction part is the difference to a run without torque from
/// the same state, so resistances cancel.
#[test]
fn script_torque_accelerates_the_whole_consist_mass() {
    let accel = |trailer: Option<f32>, torque: f32| {
        let mut sim = towing(trailer, false);
        sim.chassis.local_velocity.y = 5.0;
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: torque,
            brake_force_n: 0.0,
        });
        sim.step(1.0 / 64.0);
        sim.chassis.local_acceleration.y
    };
    let traction = |trailer| accel(trailer, 4_000.0) - accel(trailer, 0.0);
    let (solo, pair) = (traction(None), traction(Some(10_000.0)));
    let expected = solo * 11_500.0 / 21_500.0;
    assert!(
        (pair - expected).abs() < 0.02 * expected,
        "traction accel {pair} vs {expected}"
    );
    assert!((towing(Some(10_000.0), false).consist_mass_kg() - 21_500.0).abs() < 1e-3);
}

/// Plan step 4: stopping distance of head-only versus consist differs by the mass ratio
/// (here: not at all, since brake force is limited by friction, `μ·g` per kilogram).
#[test]
fn brake_limit_follows_the_summed_axle_loads() {
    let stop = |sim: &mut VehicleSimulation| {
        sim.chassis.local_velocity.y = 14.0; // ~50 km/h
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: 0.0,
            brake_force_n: 1.0e9, // more than the tyres can carry
        });
        sim.inputs.set_service_brake(1.0);
        let mut s = 0.0_f32;
        for _ in 0..2000 {
            let before = sim.chassis.position.y;
            sim.step(1.0 / 64.0);
            s += sim.chassis.position.y - before;
            if sim.chassis.forward_speed_m_s() <= 0.0 {
                break;
            }
        }
        s
    };
    let (solo, pair) = (
        stop(&mut towing(None, false)),
        stop(&mut towing(Some(10_000.0), false)),
    );
    // Same μ·g deceleration: the distance matches within 2 % (every axle brakes).
    assert!((solo - pair).abs() < 0.02 * solo, "{solo} vs {pair}");
    let ideal = 14.0_f32 * 14.0 / (2.0 * ROAD_FRICTION * GRAVITY);
    assert!(solo > ideal * 0.95, "{solo} vs ideal {ideal}");
}

/// Plan step 5: the drive cap follows the driven axles of all units.
#[test]
fn drive_limit_follows_the_driven_axles_of_all_units() {
    let head_only = towing(Some(10_000.0), false);
    let all_driven = towing(Some(10_000.0), true);
    let (d0, b0) = head_only.friction_loads_n();
    let (d1, b1) = all_driven.friction_loads_n();
    assert!((b0 - b1).abs() < 1.0, "braked load is the same");
    assert!((b0 - 21_500.0 * GRAVITY).abs() < 5.0);
    assert!((d1 - d0 - 10_000.0 * GRAVITY).abs() < 5.0, "{d0} -> {d1}");
    // The head's rear axle carries half its weight on this symmetric bus.
    assert!((d0 - 0.5 * 11_500.0 * GRAVITY).abs() < 5.0);
}

/// Plan step 6: on a straight run `F_h = m_last·a + resistances`.
#[test]
fn hitch_force_of_the_last_unit_is_mass_times_acceleration_plus_rolling() {
    let mut sim = towing(Some(10_000.0), false);
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 6_000.0,
        brake_force_n: 0.0,
    });
    for _ in 0..64 {
        sim.step(1.0 / 64.0);
    }
    let a = sim.chassis.local_acceleration.y;
    let f = sim.hitch_force_n(0).unwrap();
    let want = 10_000.0 * (a + drivetrain::ROLLING_RESISTANCE_COEFF * GRAVITY);
    assert!(a > 0.1 && (f - want).abs() < 1.0, "{f} vs {want}");
    assert_eq!(sim.hitch_force_n(1), None);
}

#[test]
fn loads_survive_serialisation() {
    let sim = towing(Some(9_000.0), true);
    let back: VehicleSimulation = bincode::deserialize(&bincode::serialize(&sim).unwrap()).unwrap();
    assert_eq!(sim, back);
}
