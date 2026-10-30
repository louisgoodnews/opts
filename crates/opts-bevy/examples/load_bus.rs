//! Example: Loading and Spawning a Full OMSI Bus Model with Cameras in Bevy.
//!
//! Demonstrates:
//! 1. Registering `OptsAssetPlugin` (asset loaders, coordinate bridge, and transform sync).
//! 2. Parsing a `.bus` vehicle manifest and `model.cfg` rendering definition via `opts-core`.
//! 3. Spawning the bus vehicle entity hierarchy with child meshes, materials, and light sources.
//! 4. Transforming and switching between vehicle camera presets (`[add_camera_driver]`,
//!    `[add_camera_pax]`, `[view_schedule]`, `[view_ticketselling]`, and outside orbit view)
//!    using [`camera_preset_to_bevy_transform`].
//!
//! Run with:
//! ```bash
//! cargo run -p opts-bevy --example load_bus
//! ```
//!
//! Controls:
//! - `1`: Driver seat view
//! - `2`: Timetable / schedule view
//! - `3`: Ticket selling view
//! - `4`: Passenger compartment view
//! - `5`: Exterior chase / orbit view
//! - `Space`: Cycle through camera presets

use bevy::prelude::*;
use opts_bevy::{
    camera_preset_to_bevy_transform, omsi_to_bevy_pos, spawn_vehicle_model, BusVehicle,
    OptsAssetPlugin,
};
use opts_core::formats::vehicles::bus::BusManifest;

#[path = "load_bus/sample.rs"]
mod sample;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS — OMSI Bus Vehicle Loader & Camera Preset Example".into(),
                resolution: bevy::window::WindowResolution::new(1920, 1080),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsAssetPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (handle_camera_inputs, update_camera_transform))
        .run();
}

/// Marker component for the active vehicle viewer camera.
#[derive(Component)]
struct BusViewerCamera {
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

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. Environment Lighting & Sky
    commands.spawn((
        DirectionalLight {
            illuminance: 14_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.65, 0.45, 0.0)),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.9, 0.92, 1.0),
        brightness: 450.0,
        ..default()
    });

    // 2. Asphalt Road Ground Plane
    let road_mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(100.0, 100.0)));
    let road_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(road_mesh),
        MeshMaterial3d(road_mat),
        Transform::IDENTITY,
    ));

    // 3.-5. Representative manifest, model config and demonstration meshes & materials
    let manifest = sample::sample_manifest();
    let model = sample::sample_model();
    let (mesh_handles, mat_handles) = sample::demo_handles(&mut meshes, &mut materials);

    // 6. Spawn the vehicle entity hierarchy
    let bus_spawn_transform = Transform::from_xyz(0.0, 1.6, 0.0);
    let bus_entity = spawn_vehicle_model(
        &mut commands,
        &manifest,
        &model,
        bus_spawn_transform,
        &mesh_handles,
        &mat_handles,
    );

    info!("Spawned OMSI Bus Vehicle Root Entity: {:?}", bus_entity);

    spawn_viewer_camera(&mut commands, &manifest, bus_spawn_transform);
    print_controls();
}

/// Spawns the camera at the first driver camera preset.
fn spawn_viewer_camera(
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

fn print_controls() {
    println!("============================================================");
    println!("OPTS Bus Loader Example Running!");
    println!("Controls to inspect vehicle camera presets:");
    println!("  [1] - Driver Seat View (Standard OMSI Driver Camera)");
    println!("  [2] - Timetable / Schedule View ([view_schedule])");
    println!("  [3] - Ticket Selling View ([view_ticketselling])");
    println!("  [4] - Passenger Seat View ([add_camera_pax])");
    println!("  [5] - Exterior Orbit / Chase View ([set_camera_outside_center])");
    println!("  [Space] - Cycle Through Presets");
    println!("============================================================");
}

fn handle_camera_inputs(
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

fn update_camera_transform(
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
