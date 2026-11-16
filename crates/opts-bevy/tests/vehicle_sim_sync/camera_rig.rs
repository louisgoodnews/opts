//! The chase camera rig trails the vehicle.

use super::*;

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
