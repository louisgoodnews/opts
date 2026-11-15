//! Unit tests of the vehicle simulation: forward torque while rolling back, missing ground samples.

use super::*;

/// A forward torque while the vehicle rolls backwards slows the roll: the traction part of the
/// acceleration (the difference to a run without torque) is forward at any speed, also negative.
#[test]
fn forward_script_torque_pushes_forward_while_rolling_backwards() {
    for v in [-0.5_f32, -5.0, -12.0] {
        let accel = |torque: f32| {
            let mut sim = towing(None, false);
            sim.chassis.local_velocity.y = v;
            sim.script_drive = Some(ScriptDrive {
                wheel_torque_nm: torque,
                brake_force_n: 0.0,
            });
            for _ in 0..4 {
                sim.step(1.0 / 64.0);
            }
            sim.chassis.local_acceleration.y
        };
        let traction = accel(4_000.0) - accel(0.0);
        assert!(traction > 0.0, "at {v} m/s the torque pushed {traction}");
    }
}

/// Rolling backwards with a constant forward idle torque (a bus in drive on a flat road): the
/// speed moves towards zero, it never grows backwards.
#[test]
fn a_steady_forward_torque_slows_a_roll_back_on_the_flat() {
    let mut sim = towing(None, false);
    sim.chassis.local_velocity.y = -4.0;
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 4_000.0,
        brake_force_n: 0.0,
    });
    let mut last = sim.chassis.forward_speed_m_s();
    for i in 0..640 {
        sim.step(1.0 / 64.0);
        let v = sim.chassis.forward_speed_m_s();
        assert!(v >= last - 1e-3, "step {i}: {last} -> {v}");
        last = v;
    }
    assert!(last > -4.0, "ended at {last}");
}

/// The same roll back with an articulated consist (a trailer behind the head).
#[test]
fn a_steady_forward_torque_slows_a_roll_back_of_a_consist_too() {
    let mut sim = towing(Some(10_000.0), false);
    sim.chassis.local_velocity.y = -4.0;
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 4_000.0,
        brake_force_n: 0.0,
    });
    let mut last = sim.chassis.forward_speed_m_s();
    for i in 0..640 {
        sim.step(1.0 / 64.0);
        let v = sim.chassis.forward_speed_m_s();
        assert!(v >= last - 1e-3, "step {i}: {last} -> {v}");
        last = v;
    }
}

/// Over ground that is no longer there (the edge of the loaded map) the last tilt must not pull
/// the vehicle down a slope for ever: it levels out.
#[test]
fn a_missing_ground_sample_levels_the_body_instead_of_freezing_its_tilt() {
    let mut sim = towing(None, false);
    let probe = sim.ground_probes()[0];
    let rise = |m: f32| {
        let [rl, rr, fl, fr] = [0.0, 0.0, m, m];
        Some(GroundSample::from_heights(probe, [rl, rr, fl, fr]))
    };
    for _ in 0..64 {
        sim.apply_ground(&[rise(1.5)], 1.0 / 64.0);
    }
    assert!(sim.head_body.expect("body").pitch > 0.1);
    for _ in 0..256 {
        sim.apply_ground(&[None], 1.0 / 64.0);
    }
    let b = sim.head_body.expect("body");
    assert!(b.pitch.abs() < 0.01 && b.roll.abs() < 0.01, "{b:?}");
}
