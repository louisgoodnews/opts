//! The vehicle transform follows the simulation.

use super::*;

#[test]
fn test_vehicle_transform_synchronization() {
    let mut app = create_test_app();

    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(0.8);
    controller.inputs_mut().set_steering(0.5); // Steer right

    let vehicle_entity = app
        .world_mut()
        .spawn((controller, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();

    // Run 30 steps
    for _ in 0..30 {
        app.update();
    }

    let entity_ref = app.world().entity(vehicle_entity);
    let controller_ref = entity_ref
        .get::<VehicleController>()
        .expect("vehicle must have VehicleController");
    let transform_ref = entity_ref
        .get::<Transform>()
        .expect("vehicle must have Transform");

    // In OMSI: +Y is forward, so in Bevy: -Z is forward
    assert!(
        transform_ref.translation.z < -0.1,
        "Bevy Z translation should be negative (forward), was {}",
        transform_ref.translation.z
    );

    // In OMSI: steering right turns clockwise, heading becomes positive.
    // In Bevy: rotation around +Y should be negative (-heading).
    let heading = controller_ref.sim.chassis.heading_rad;
    assert!(
        heading > 0.0,
        "Vehicle should yaw right under positive steering lock"
    );
}
