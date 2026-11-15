//! The bus: its entity hierarchy with chassis, body, windows and the controller.

use super::*;

/// Spawns the playable bus entity hierarchy with chassis, body, windows, and wheels.
pub(super) fn setup_vehicle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Controller: standard 11.5 tonne 12m transit bus with 5.85m wheelbase
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    // Disengage parking brake and engage forward automatic drive
    controller.inputs_mut().set_parking_brake(false);
    controller
        .inputs_mut()
        .set_gear(opts_sim::vehicle::inputs::GearPosition::Drive);

    let vehicle_root = commands
        .spawn((
            controller,
            PlayerVehicle,
            Transform::from_xyz(0.0, 0.0, 0.0),
            Name::new("Player_Bus"),
        ))
        .id();

    // Visual Bus Body: 11.8m length, 2.5m width, 2.8m height
    let body_mesh = meshes.add(Cuboid::new(2.5, 2.8, 11.8));
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.55, 0.65), // Transit teal livery
        perceptual_roughness: 0.35,
        metallic: 0.2,
        ..default()
    });

    let body_entity = commands
        .spawn((
            (
                Mesh3d(body_mesh),
                MeshMaterial3d(body_mat),
                Transform::from_xyz(0.0, 1.85, 0.0),
            ),
            Name::new("Bus_Chassis_Body"),
        ))
        .id();

    // Windshield & Side Windows
    let glass_mesh = meshes.add(Cuboid::new(2.45, 1.1, 11.0));
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.2, 0.25, 0.6),
        perceptual_roughness: 0.1,
        metallic: 0.8,
        ..default()
    });

    let glass_entity = commands
        .spawn((
            (
                Mesh3d(glass_mesh),
                MeshMaterial3d(glass_mat),
                Transform::from_xyz(0.0, 2.3, 0.0),
            ),
            Name::new("Bus_Glass_Strip"),
        ))
        .id();

    // Wheels (4 road wheels): 0.98m diameter (0.49m radius), 0.35m width
    let wheel_mesh = meshes.add(Cylinder::new(0.49, 0.35));
    let wheel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1),
        perceptual_roughness: 0.9,
        ..default()
    });

    let [front_left_wheel, front_right_wheel, rear_left_wheel, rear_right_wheel] =
        spawn_wheels(&mut commands, &wheel_mesh, &wheel_mat);

    commands.entity(vehicle_root).add_children(&[
        body_entity,
        glass_entity,
        front_left_wheel,
        front_right_wheel,
        rear_left_wheel,
        rear_right_wheel,
    ]);

    spawn_chase_camera(&mut commands);
}
