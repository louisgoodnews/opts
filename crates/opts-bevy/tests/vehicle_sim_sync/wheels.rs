//! Wheel visuals take steering and rolling rotation.

use super::*;

#[test]
fn test_wheel_visual_transform_updates() {
    let mut app = create_test_app();

    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(1.0);
    controller.inputs_mut().set_steering(0.6); // Steering lock

    let front_left_wheel = app
        .world_mut()
        .spawn((
            WheelVisual::new(0, WheelSide::Left, Vec3::new(-1.25, 0.49, -2.9)),
            Transform::IDENTITY,
        ))
        .id();

    let front_right_wheel = app
        .world_mut()
        .spawn((
            WheelVisual::new(0, WheelSide::Right, Vec3::new(1.25, 0.49, -2.9)),
            Transform::IDENTITY,
        ))
        .id();

    let rear_left_wheel = app
        .world_mut()
        .spawn((
            WheelVisual::new(1, WheelSide::Left, Vec3::new(-1.25, 0.49, 2.9)),
            Transform::IDENTITY,
        ))
        .id();

    let vehicle_entity = app
        .world_mut()
        .spawn((controller, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();

    app.world_mut().entity_mut(vehicle_entity).add_children(&[
        front_left_wheel,
        front_right_wheel,
        rear_left_wheel,
    ]);

    // Step physics & transforms
    for _ in 0..45 {
        app.update();
    }

    // Inspect front left wheel: should have both steering yaw and rolling spin rotation
    let fl_transform = app
        .world()
        .entity(front_left_wheel)
        .get::<Transform>()
        .expect("front left wheel must have Transform");

    assert_ne!(
        fl_transform.rotation,
        Quat::IDENTITY,
        "Steered and rolling wheel must have non-identity rotation"
    );

    // Inspect rear left wheel: non-steered axle, but rolling rotation should be active
    let rl_transform = app
        .world()
        .entity(rear_left_wheel)
        .get::<Transform>()
        .expect("rear left wheel must have Transform");

    assert_ne!(
        rl_transform.rotation,
        Quat::IDENTITY,
        "Driven rolling wheel must have non-identity rotation"
    );
}
