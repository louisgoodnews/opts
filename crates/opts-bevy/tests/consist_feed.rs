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
