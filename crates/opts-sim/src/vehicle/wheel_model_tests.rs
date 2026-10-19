//! Wheel model tests.

use super::*;

/// M-0097: under script drive the speed after 10 s at constant torque is the same, within
/// 5 %, at 30, 60 and 120 steps per second.
#[test]
fn script_drive_does_not_depend_on_the_step_size() {
    let speed_after_10_s = |hz: f32| {
        let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
        sim.inputs.set_parking_brake(false);
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: 6000.0,
            brake_force_n: 0.0,
        });
        for _ in 0..(10.0 * hz) as usize {
            sim.step(1.0 / hz);
        }
        sim.chassis.forward_speed_m_s()
    };
    let (a, b, c) = (
        speed_after_10_s(30.0),
        speed_after_10_s(60.0),
        speed_after_10_s(120.0),
    );
    assert!(
        (a - b).abs() < 0.05 * b && (c - b).abs() < 0.05 * b,
        "{a} {b} {c}"
    );
    assert!(b > 8.0, "it drives: {b}");
}

/// The wheels of a vehicle under script drive turn with the road when nothing brakes.
#[test]
fn the_wheels_follow_the_road_speed_without_brakes() {
    let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
    sim.inputs.set_parking_brake(false);
    sim.chassis.local_velocity.y = 10.0;
    sim.script_drive = Some(ScriptDrive::default());
    for _ in 0..120 {
        sim.step(1.0 / 60.0);
    }
    let v = sim.chassis.forward_speed_m_s();
    let w = &sim.axles[0].left_wheel;
    assert!(
        (w.angular_velocity_rad_s * w.rolling_radius_m - v).abs() < 0.2,
        "{v}"
    );
}

/// The scripts' spring factor stiffens the body: under the same braking the pitch is smaller
/// with a factor of 2, and a non-finite factor changes nothing.
#[test]
fn the_spring_factor_stiffens_the_suspension() {
    let pitch = |factor: f32| {
        let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
        sim.inputs.set_parking_brake(false);
        sim.chassis.local_velocity.y = 12.0;
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: 0.0,
            brake_force_n: 30_000.0,
        });
        sim.step(1.0 / 60.0); // builds the body
        for w in 0..sim.axles.len() * 2 {
            sim.set_spring_factor(w, factor);
            sim.set_spring_factor(w, f32::NAN);
        }
        for _ in 0..60 {
            sim.step(1.0 / 60.0);
        }
        sim.body.as_ref().expect("body").pitch_rad.abs()
    };
    let (soft, stiff) = (pitch(1.0), pitch(2.0));
    assert!(soft > 0.0 && stiff < 0.8 * soft, "{soft} vs {stiff}");
}

/// A vehicle the scripts' brakes hold on a slope does not roll backwards when the pedal
/// is untouched (M-0097): the guard counts the script brake.
#[test]
fn a_script_brake_stops_the_roll_through_zero() {
    let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
    sim.inputs.set_parking_brake(false);
    sim.chassis.local_velocity.y = 1.0;
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 0.0,
        brake_force_n: 60_000.0,
    });
    for _ in 0..600 {
        sim.step(1.0 / 60.0);
        assert!(
            sim.chassis.forward_speed_m_s() >= -0.001,
            "rolled backwards"
        );
    }
    assert!(sim.chassis.forward_speed_m_s().abs() < 0.01);
}
