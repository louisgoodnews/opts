//! The probe's scene: a ground plane, a sun, the two cubes behind the camera, the player vehicle
//! with its window and mirror panes, and the HDR main camera.

use bevy::camera::Hdr;
use bevy::prelude::*;
use opts_bevy::glass::{register_surface, SurfaceKind};
use opts_bevy::PlayerVehicle;

use super::{GLASS_AT, MIRROR_AT};

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.37),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(ground),
        Transform::default(),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 20_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(-Vec3::new(0.4, 0.7, 0.3), Vec3::Y),
    ));
    // The cubes the mirror must show: behind the main camera.
    for (x, c) in [
        (0.0, Color::srgb(0.9, 0.1, 0.1)),
        (2.0, Color::srgb(0.1, 0.2, 0.9)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.5, 1.5, 1.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: c,
                ..default()
            })),
            Transform::from_xyz(x, 0.75, 6.0),
        ));
    }
    // The player vehicle with a window pane and a mirror pane.
    let vehicle = commands
        .spawn((PlayerVehicle, Transform::default(), Visibility::Inherited))
        .id();
    let pane = meshes.add(Rectangle::new(0.7, 0.45));
    let glass_mat = StandardMaterial {
        base_color: Color::srgba(0.35, 0.45, 0.5, 0.25),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.8,
        ..default()
    };
    let glass = materials.add(glass_mat.clone());
    register_surface(&glass, SurfaceKind::Glass, &glass_mat);
    let mirror_mat = StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        unlit: true,
        ..default()
    };
    let mirror = materials.add(mirror_mat.clone());
    register_surface(&mirror, SurfaceKind::Mirror, &mirror_mat);
    for (at, mat) in [(GLASS_AT, glass), (MIRROR_AT, mirror)] {
        let e = commands
            .spawn((
                Mesh3d(pane.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(at),
            ))
            .id();
        commands.entity(vehicle).add_child(e);
    }
    commands.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Hdr,
        Transform::from_xyz(0.0, 1.5, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y),
    ));
}
