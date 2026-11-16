//! The vehicle viewer camera: the view presets, the keys that switch them and the camera follow.

use bevy::prelude::*;
use opts_bevy::{camera_preset_to_bevy_transform, omsi_to_bevy_pos, BusVehicle};
use opts_core::formats::vehicles::bus::BusManifest;

/// Marker component for the active vehicle viewer camera.
#[derive(Component)]
pub(super) struct BusViewerCamera {
    current_mode: CameraMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    DriverSeat,
    ScheduleView,
    TicketSellingView,
    PassengerView,
    ExteriorOrbit,
}

/// Spawns the camera at the first driver camera preset.
pub(super) fn spawn_viewer_camera(
    commands: &mut Commands,
    manifest: &BusManifest,
    bus_spawn_transform: Transform,
) {
    // 7. Spawn Camera configured to initial driver view
    let initial_driver_cam = &manifest.driver_cameras[0];
    let initial_transform = camera_preset_to_bevy_transform(
        Vec3::new(
            initial_driver_cam.x,
            initial_driver_cam.y,
            initial_driver_cam.z,
        ),
        initial_driver_cam.dist,
        initial_driver_cam.yaw_deg,
        initial_driver_cam.pitch_deg,
    );

    // Rebase relative to spawned bus root position
    let world_cam_transform = Transform {
        translation: bus_spawn_transform.translation + initial_transform.translation,
        rotation: initial_transform.rotation,
        scale: Vec3::ONE,
    };

    commands.spawn((
        (
            Camera3d::default(),
            opts_bevy::MainCamera,
            world_cam_transform,
            Projection::Perspective(PerspectiveProjection {
                fov: initial_driver_cam.fov_deg.to_radians(),
                ..default()
            }),
        ),
        BusViewerCamera {
            current_mode: CameraMode::DriverSeat,
        },
    ));
}

pub(super) fn handle_camera_inputs(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut BusViewerCamera>,
) {
    let Ok(mut cam) = camera_query.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Digit1) {
        cam.current_mode = CameraMode::DriverSeat;
        info!("Switched to Driver Seat View");
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        cam.current_mode = CameraMode::ScheduleView;
        info!("Switched to Timetable / Schedule View");
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        cam.current_mode = CameraMode::TicketSellingView;
        info!("Switched to Ticket Selling View");
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        cam.current_mode = CameraMode::PassengerView;
        info!("Switched to Passenger Compartment View");
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        cam.current_mode = CameraMode::ExteriorOrbit;
        info!("Switched to Exterior Orbit View");
    } else if keyboard.just_pressed(KeyCode::Space) {
        cam.current_mode = match cam.current_mode {
            CameraMode::DriverSeat => CameraMode::ScheduleView,
            CameraMode::ScheduleView => CameraMode::TicketSellingView,
            CameraMode::TicketSellingView => CameraMode::PassengerView,
            CameraMode::PassengerView => CameraMode::ExteriorOrbit,
            CameraMode::ExteriorOrbit => CameraMode::DriverSeat,
        };
        info!("Cycled Camera View to: {:?}", cam.current_mode);
    }
}

pub(super) fn update_camera_transform(
    time: Res<Time>,
    vehicle_query: Query<(&BusVehicle, &Transform), Without<BusViewerCamera>>,
    mut camera_query: Query<
        (&BusViewerCamera, &mut Transform, &mut Projection),
        With<BusViewerCamera>,
    >,
) {
    let Ok((vehicle, vehicle_tx)) = vehicle_query.single() else {
        return;
    };
    let Ok((viewer, mut cam_tx, mut projection)) = camera_query.single_mut() else {
        return;
    };

    let target_preset = match viewer.current_mode {
        CameraMode::DriverSeat => vehicle.driver_cameras.first(),
        CameraMode::ScheduleView => vehicle.driver_cameras.get(1),
        CameraMode::TicketSellingView => vehicle.driver_cameras.get(2),
        CameraMode::PassengerView => vehicle.pax_cameras.first(),
        CameraMode::ExteriorOrbit => None,
    };

    if let Some(preset) = target_preset {
        let local_tx = camera_preset_to_bevy_transform(
            Vec3::new(preset.x, preset.y, preset.z),
            preset.dist,
            preset.yaw_deg,
            preset.pitch_deg,
        );

        let target_pos = vehicle_tx.translation + local_tx.translation;
        cam_tx.translation = cam_tx
            .translation
            .lerp(target_pos, time.delta_secs() * 10.0);
        cam_tx.rotation = cam_tx
            .rotation
            .slerp(local_tx.rotation, time.delta_secs() * 10.0);

        if let Projection::Perspective(ref mut persp) = *projection {
            let target_fov = preset.fov_deg.to_radians();
            persp.fov = persp.fov + (target_fov - persp.fov) * time.delta_secs() * 8.0;
        }
    } else if viewer.current_mode == CameraMode::ExteriorOrbit {
        // Orbit around outside_camera_center
        let omsi_center = vehicle
            .outside_camera_center
            .unwrap_or(Vec3::new(0.0, 0.5, 1.4));
        let center_bevy = vehicle_tx.translation + omsi_to_bevy_pos(omsi_center);

        let orbit_dist = 14.0;
        let orbit_height = 4.0;
        let orbit_angle = time.elapsed_secs() * 0.2;

        let target_pos = center_bevy
            + Vec3::new(
                orbit_angle.sin() * orbit_dist,
                orbit_height,
                orbit_angle.cos() * orbit_dist,
            );

        cam_tx.translation = cam_tx.translation.lerp(target_pos, time.delta_secs() * 5.0);
        let look_rot = Transform::from_translation(cam_tx.translation)
            .looking_at(center_bevy, Vec3::Y)
            .rotation;
        cam_tx.rotation = cam_tx.rotation.slerp(look_rot, time.delta_secs() * 5.0);

        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = 60.0_f32.to_radians();
        }
    }
}
