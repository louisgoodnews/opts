//! Integration tests for the Bevy vehicle physics simulation bridge.

use std::time::Duration;

use bevy::{input::InputPlugin, prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    OptsVehiclePlugin, PlayerVehicle, VehicleCameraMode, VehicleCameraRig, VehicleController,
    WheelSide, WheelVisual,
};
use opts_sim::vehicle::inputs::GearPosition;

fn create_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(OptsVehiclePlugin);
    app
}

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

    app.world_mut().entity_mut(vehicle_entity).push_children(&[
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

#[test]
fn test_camera_rig_tracking() {
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

    let camera_entity = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 5.0, 15.0),
            VehicleCameraRig {
                mode: VehicleCameraMode::ExteriorChase,
                chase_distance: 10.0,
                chase_height: 3.5,
                cockpit_offset: Vec3::new(-0.7, 1.8, -4.2),
                damping: 5.0,
            },
        ))
        .id();

    for _ in 0..30 {
        app.update();
    }

    let vehicle_pos = app
        .world()
        .entity(vehicle_entity)
        .get::<Transform>()
        .expect("vehicle must have Transform")
        .translation;

    let cam_pos = app
        .world()
        .entity(camera_entity)
        .get::<Transform>()
        .expect("camera must have Transform")
        .translation;

    // In chase mode, camera should trail behind the vehicle (greater Z than vehicle's negative Z)
    assert!(
        cam_pos.z > vehicle_pos.z,
        "Chase camera should remain behind moving vehicle: cam.z={}, vehicle.z={}",
        cam_pos.z,
        vehicle_pos.z
    );
}
