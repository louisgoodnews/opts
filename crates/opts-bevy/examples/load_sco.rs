//! Example: Loading and Spawning an OMSI Scenery Object (`.sco`) in Bevy.
//!
//! Demonstrates:
//! 1. Registering `OptsAssetPlugin` (includes `.sco`, `.sli`, `.o3d`, `.x`, textures, and coordinate bridges).
//! 2. Parsing a `.sco` scenery object definition via `opts-core`.
//! 3. Spawning the scenery entity with `OmsiTransform` and child 3D mesh attachments.
//! 4. Configuring a 3D camera and directional lighting.
//!
//! Run with:
//! ```bash
//! cargo run --example load_sco
//! ```

use bevy::prelude::*;
use opts_bevy::{spawn_scenery_object, OptsAssetPlugin};
use opts_core::formats::scenery::{SceneryObject, ScoMaterial, ScoMesh};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS — Scenery Object Loader Example".into(),
                resolution: (1280.0_f32, 720.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsAssetPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. Setup Camera and Lighting
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.5, 0.0)),
    ));

    // Ambient light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
    });

    // 2. Define or parse an OMSI Scenery Object
    let mut sco = SceneryObject {
        friendly_name: Some("Bus_Stop_Shelter".to_string()),
        ..Default::default()
    };

    let mut mesh_base = ScoMesh::new("shelter_base.o3d");
    mesh_base.materials.push(ScoMaterial::new("metal.bmp", 0));
    sco.meshes.push(mesh_base);

    let mut mesh_glass = ScoMesh::new("shelter_glass.o3d");
    let mut glass_mat = ScoMaterial::new("glass.dds", 0);
    glass_mat.alpha_mode = Some(2); // Alpha blend
    mesh_glass.materials.push(glass_mat);
    sco.meshes.push(mesh_glass);

    // 3. Create placeholder meshes & materials for the demonstration
    let base_mesh_handle = meshes.add(Cuboid::new(2.5, 2.2, 1.2));
    let glass_mesh_handle = meshes.add(Plane3d::new(Vec3::Z, Vec2::new(1.1, 0.9)));

    let base_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.25, 0.3),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    let glass_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.9, 1.0, 0.4),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let mesh_handles = vec![base_mesh_handle, glass_mesh_handle];
    let mat_handles = vec![base_mat_handle, glass_mat_handle];

    // 4. Spawn the scenery object entity hierarchy in Bevy world space
    let spawn_transform = Transform::from_xyz(0.0, 1.1, 0.0);
    let entity = spawn_scenery_object(
        &mut commands,
        &sco,
        spawn_transform,
        &mesh_handles,
        &mat_handles,
    );

    info!(
        "Successfully spawned OMSI Scenery Object entity: {:?}",
        entity
    );
}
