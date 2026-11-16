//! A controller spawns and accelerates on its fixed step.

use super::*;

#[test]
fn test_vehicle_controller_spawn_and_fixed_step() {
    let mut app = create_test_app();

    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(1.0);

    let vehicle_entity = app
        .world_mut()
        .spawn((
            controller,
            Transform::from_xyz(0.0, 0.0, 0.0),
            PlayerVehicle,
        ))
        .id();

    // Hold KeyW to simulate player pressing forward accelerator
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);

    // Advance simulation by 60 ticks (1.0 second of simulation time)
    for _ in 0..60 {
        app.update();
    }

    let controller_ref = app
        .world()
        .entity(vehicle_entity)
        .get::<VehicleController>()
        .expect("vehicle entity must have VehicleController");

    // Vehicle should have accelerated forward from rest
    let speed_km_h = controller_ref.forward_speed_km_h();
    assert!(
        speed_km_h > 5.0,
        "Vehicle should accelerate under 100% throttle, speed={speed_km_h} km/h"
    );

    let forward_dist = controller_ref.sim.chassis.position.y;
    assert!(
        forward_dist > 0.5,
        "Vehicle should travel forward, dist={forward_dist} m"
    );
}
