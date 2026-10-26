//! The wheels of the following units reach the scripts (M-0097): the driven axle of an
//! articulated bus hangs in its second unit and its scripts read `Wheel_RotationSpeed_2_*`; the
//! gearbox never shifts up when that reads zero.

use bevy::math::DVec2;
use opts_bevy::vehicle_script::host_inputs;
use opts_bevy::VehicleController;
use opts_sim::vehicle::consist::{UnitAxle, UnitLoad};

fn articulated() -> VehicleController {
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.sim.attach_trailer(
        DVec2::new(0.0, -2.0),
        DVec2::new(0.0, 1.0),
        6.0,
        52.5,
        Some(UnitLoad {
            mass_kg: 8_000.0,
            axles: vec![UnitAxle {
                longitudinal_m: -4.0,
                is_driven: true,
                rated_n: 100_000.0,
            }],
        }),
    );
    controller
}

#[test]
fn the_axles_of_the_second_unit_continue_the_numbering_and_roll_with_the_road() {
    let mut c = articulated();
    c.sim.chassis.local_velocity.y = 10.0;
    c.sim.script_drive = None;
    for _ in 0..30 {
        c.sim.step(1.0 / 60.0);
    }
    let inputs = host_inputs(&c, None);
    let head_axles = c.sim.axles.len();
    assert_eq!(
        inputs.axles.len(),
        head_axles + 1,
        "one axle in the trail unit"
    );
    let head = inputs.axles[0].left.speed_rpm;
    let trail = inputs.axles[head_axles].left.speed_rpm;
    assert!(head > 50.0, "the head rolls: {head}");
    assert!((trail - head).abs() < 0.1 * head, "{trail} vs {head}");
}

#[test]
fn a_driven_axle_in_the_second_unit_takes_the_torque() {
    let mut c = articulated();
    c.inputs_mut().set_parking_brake(false);
    c.sim.script_drive = Some(opts_sim::vehicle::ScriptDrive {
        wheel_torque_nm: 6_000.0,
        brake_force_n: 0.0,
    });
    c.sim.chassis.local_velocity.y = 3.0;
    for _ in 0..120 {
        c.sim.step(1.0 / 60.0);
    }
    assert!(
        c.sim.chassis.forward_speed_m_s() > 3.5,
        "it accelerates: {}",
        c.sim.chassis.forward_speed_m_s()
    );
}

#[test]
fn a_right_turn_reaches_the_scripts_as_a_positive_steering_angle() {
    // The scripts' automatic indicator cancel arms the right indicator on a positive
    // `Axle_Steering_0_L` (`lights.osc` of the VW T3 and the TA cars, read as prose).
    let mut c = VehicleController::standard_bus(11_500.0, 5.85);
    c.inputs_mut().set_steering(0.5);
    c.sim.step(1.0 / 60.0);
    let inputs = host_inputs(&c, None);
    assert!(inputs.axles[0].left.steering_rad > 0.1);
    assert!(inputs.axles[0].right.steering_rad > 0.1);
}

#[test]
fn the_scripts_read_the_suspension_travel_from_the_unloaded_spring() {
    // A standing bus: about minus its static deflection (`bremse_level_setpoint` -0.105 is the
    // levelling's target), the trail unit's axle at the head's mean.
    let mut c = articulated();
    for _ in 0..120 {
        c.sim.step(1.0 / 60.0);
    }
    let inputs = host_inputs(&c, None);
    let head = inputs.axles[0].left.suspension_m;
    assert!((-0.25..-0.02).contains(&head), "{head}");
    let rest = c.sim.body.as_ref().unwrap().rest_travel_m();
    let trail = inputs.axles.last().unwrap().left.suspension_m;
    assert!((trail - rest).abs() < 1e-6, "{trail} vs {rest}");
}

#[test]
fn a_released_wheel_brake_does_not_cancel_another_wheels_force() {
    // The Valiant's brake script gives a released brake a negative force per wheel.
    let src = "{frame}\n 100 (S.L.M_Wheel)\n -700 (S.L.Axle_Brakeforce_0_L)\n -700 (S.L.Axle_Brakeforce_0_R)\n 2000 (S.L.Axle_Brakeforce_1_L)\n 2000 (S.L.Axle_Brakeforce_1_R)\n{end}";
    let (mut engine, _) = opts_sim::script::ScriptEngine::from_sources(&[("t.osc", src)]);
    engine.run_frame(0.02, false);
    let drive = opts_bevy::vehicle_script::script_outputs(&engine).expect("drive");
    assert!(
        (drive.brake_force_n - 4000.0).abs() < 1e-3,
        "{}",
        drive.brake_force_n
    );
}
