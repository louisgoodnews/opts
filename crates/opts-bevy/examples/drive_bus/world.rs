//! The world and the chase camera: the ground, the light, the road wheels and the tracking camera.

use super::*;

/// Sets up the driving environment: ground terrain, directional sunlight, and ambient lighting.
pub(super) fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. Large asphalt ground plane (400m x 400m)
    let ground_mesh = meshes.add(Plane3d::default().mesh().size(400.0, 400.0));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.16, 0.17),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..default()
    });

    commands.spawn((
        (
            Mesh3d(ground_mesh),
            MeshMaterial3d(ground_mat),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ),
        Name::new("Ground_Asphalt"),
    ));

    // 2. Road lane markings / reference grid pads
    let marker_mesh = meshes.add(Cuboid::new(0.3, 0.02, 3.0));
    let marker_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.5,
        ..default()
    });

    for z_idx in -30..=30 {
        let z_pos = (z_idx as f32) * 6.0;
        commands.spawn((
            (
                Mesh3d(marker_mesh.clone()),
                MeshMaterial3d(marker_mat.clone()),
                Transform::from_xyz(0.0, 0.01, z_pos),
            ),
            Name::new(format!("Road_Stripe_{z_idx}")),
        ));
    }

    // 3. Directional sun lighting
    commands.spawn((
        (
            DirectionalLight {
                illuminance: 18_000.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.75, 0.65, 0.0)),
        ),
        Name::new("Sun_Light"),
    ));

    // 4. Ambient light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.85, 0.9, 1.0),
        brightness: 450.0,
        ..default()
    });
}

/// Spawns the four road wheels: front steer axle (0), rear drive axle (1); returns them as
/// front left, front right, rear left, rear right.
pub(super) fn spawn_wheels(
    commands: &mut Commands,
    wheel_mesh: &Handle<Mesh>,
    wheel_mat: &Handle<StandardMaterial>,
) -> [Entity; 4] {
    let mut wheel = |visual: WheelVisual, pos: Vec3, name: &'static str| {
        commands
            .spawn((
                (
                    Mesh3d(wheel_mesh.clone()),
                    MeshMaterial3d(wheel_mat.clone()),
                    Transform::from_translation(pos),
                ),
                visual,
                Name::new(name),
            ))
            .id()
    };
    // Front Steer Axle (Axle 0): longitudinal +2.925m in OMSI => -2.925m in Bevy (-Z forward)
    let fl = Vec3::new(-1.25, 0.49, -2.925);
    let fr = Vec3::new(1.25, 0.49, -2.925);
    // Rear Drive Axle (Axle 1): longitudinal -2.925m in OMSI => +2.925m in Bevy
    let rl = Vec3::new(-1.25, 0.49, 2.925);
    let rr = Vec3::new(1.25, 0.49, 2.925);
    [
        wheel(
            WheelVisual::new(0, WheelSide::Left, fl),
            fl,
            "Wheel_Axle0_Left",
        ),
        wheel(
            WheelVisual::new(0, WheelSide::Right, fr),
            fr,
            "Wheel_Axle0_Right",
        ),
        wheel(
            WheelVisual::new(1, WheelSide::Left, rl),
            rl,
            "Wheel_Axle1_Left",
        ),
        wheel(
            WheelVisual::new(1, WheelSide::Right, rr),
            rr,
            "Wheel_Axle1_Right",
        ),
    ]
}

/// Spawns the tracking camera with its `VehicleCameraRig`.
pub(super) fn spawn_chase_camera(commands: &mut Commands) {
    commands.spawn((
        (
            Camera3d::default(),
            opts_bevy::MainCamera,
            Transform::from_xyz(0.0, 4.5, 14.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
        ),
        VehicleCameraRig {
            mode: VehicleCameraMode::ExteriorChase,
            chase_distance: 14.0,
            chase_height: 4.5,
            cockpit_offset: Vec3::new(-0.7, 2.0, -4.8),
            damping: 8.0,
        },
        Name::new("Chase_Camera"),
    ));
}
