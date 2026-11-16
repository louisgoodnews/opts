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
use opts_bevy::{spawn_vehicle_model, OptsAssetPlugin};

#[path = "load_bus/sample.rs"]
mod sample;
#[path = "load_bus/viewer.rs"]
mod viewer;
use viewer::{handle_camera_inputs, spawn_viewer_camera, update_camera_transform};

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
