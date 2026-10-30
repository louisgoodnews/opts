//! Example: Interactive Bus Driving & Simulation Bridge in Bevy (`opts-bevy`).
//!
//! Demonstrates:
//! 1. Initializing the [`OptsVehiclePlugin`] and deterministic physics simulation bridge.
//! 2. Spawning a 12-meter commercial transit bus model with Ackermann-steered front wheels.
//! 3. Real-time driver keyboard controls (WASD/Arrows, Space handbrake, E/N/R gears).
//! 4. Exterior chase and interior cockpit camera tracking with F1 / F2 toggle.
//! 5. On-screen telemetry HUD showing road speed, engine RPM, gear, and controls.
//!
//! Run with:
//! ```bash
//! cargo run --example drive_bus
//! ```

use bevy::prelude::*;
use opts_bevy::{
    OptsAssetPlugin, PlayerVehicle, VehicleCameraMode, VehicleCameraRig, VehicleController,
    WheelSide, WheelVisual,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS — Interactive Bus Driving Simulation".into(),
                resolution: bevy::window::WindowResolution::new(1920, 1080),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsAssetPlugin)
        .add_systems(Startup, (setup_world, setup_vehicle, setup_hud))
        .add_systems(Update, (update_telemetry_hud, handle_exit))
        .run();
}

/// Sets up the driving environment: ground terrain, directional sunlight, and ambient lighting.
fn setup_world(
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
fn spawn_wheels(
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
        wheel(WheelVisual::new(0, WheelSide::Left, fl), fl, "Wheel_Axle0_Left"),
        wheel(WheelVisual::new(0, WheelSide::Right, fr), fr, "Wheel_Axle0_Right"),
        wheel(WheelVisual::new(1, WheelSide::Left, rl), rl, "Wheel_Axle1_Left"),
        wheel(WheelVisual::new(1, WheelSide::Right, rr), rr, "Wheel_Axle1_Right"),
    ]
}

/// Spawns the tracking camera with its `VehicleCameraRig`.
fn spawn_chase_camera(commands: &mut Commands) {
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

/// Spawns the playable bus entity hierarchy with chassis, body, windows, and wheels.
fn setup_vehicle(
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

/// Marker component for the on-screen telemetry dashboard text.
#[derive(Component)]
struct TelemetryHudText;

/// Spawns the telemetry overlay user interface.
fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                (
                    Text::new("OPTS Driving Telemetry\nSpeed: 0.0 km/h\nRPM: 600\nGear: D"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.95, 1.0)),
                ),
                TelemetryHudText,
            ));
        });
}

/// Refreshes telemetry text overlay each frame from active [`VehicleController`] state.
fn update_telemetry_hud(
    vehicle_query: Query<&VehicleController, With<PlayerVehicle>>,
    mut text_query: Query<&mut Text, With<TelemetryHudText>>,
) {
    let Ok(controller) = vehicle_query.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let speed = controller.forward_speed_km_h();
    let rpm = controller.engine_rpm();
    let gear_str = match controller.gear() {
        opts_sim::vehicle::inputs::GearPosition::Neutral => "N (Neutral)".to_string(),
        opts_sim::vehicle::inputs::GearPosition::Drive => "D (Drive)".to_string(),
        opts_sim::vehicle::inputs::GearPosition::Reverse => "R (Reverse)".to_string(),
        opts_sim::vehicle::inputs::GearPosition::Forward(g) => format!("{g} (Hold)"),
    };
    let park_str = if controller.inputs().is_parking_brake_engaged() {
        "PARK [ON]"
    } else {
        "PARK [OFF]"
    };
    let steer_pct = (controller.inputs().steering() * 100.0).round();

    text.0 = format!(
        "OPTS Driving Simulator\n\
         ─────────────────────────────\n\
         Speed:     {:>5.1} km/h\n\
         Engine:    {:>5.0} RPM\n\
         Gear:      {}\n\
         Brake:     {}\n\
         Steering:  {:>+4.0}%\n\
         ─────────────────────────────\n\
         W / S:     Throttle / Foot Brake\n\
         A / D:     Steer Left / Right\n\
         Space:     Handbrake Toggle\n\
         E / N / R: Gear Select (Drive/Neutral/Rev)\n\
         F1 / F2:   Chase / Cockpit Camera\n\
         Esc:       Exit",
        speed, rpm, gear_str, park_str, steer_pct
    );
}

/// Closes the application gracefully when Escape is pressed.
fn handle_exit(keyboard: Res<ButtonInput<KeyCode>>, mut exit_events: MessageWriter<AppExit>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(AppExit::Success);
    }
}
