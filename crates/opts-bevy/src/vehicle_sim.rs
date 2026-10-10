//! Vehicle physics simulation bridge and ECS integration for Bevy.
//!
//! Connects the authoritative [`VehicleSimulation`] domain model from `opts-sim`
//! to Bevy's ECS architecture. Coordinates deterministic fixed-step physics execution
//! in [`FixedUpdate`], smooth player input ingestion, chassis coordinate projection
//! into 3D transforms, dynamic wheel steering and rolling mesh animations, and
//! chase/cockpit camera tracking.

use bevy::prelude::*;
use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::vehicle::{
    axle::AxleState,
    inputs::{GearPosition, VehicleInputs},
    VehicleSimulation,
};

use crate::coords::omsi_to_bevy_pos;

/// Component wrapping an authoritative [`VehicleSimulation`] state for an entity.
#[derive(Component, Debug, Clone)]
pub struct VehicleController {
    /// Authoritative simulation instance from `opts-sim`.
    pub sim: VehicleSimulation,
}

impl VehicleController {
    /// Constructs a new controller wrapping an existing [`VehicleSimulation`].
    #[must_use]
    pub fn new(sim: VehicleSimulation) -> Self {
        Self { sim }
    }

    /// Constructs a controller with a standard 12-meter transit bus configuration.
    #[must_use]
    pub fn standard_bus(mass_kg: f32, wheelbase_m: f32) -> Self {
        Self {
            sim: VehicleSimulation::new_standard_bus(mass_kg, wheelbase_m),
        }
    }

    /// Constructs a controller from a parsed OMSI [`BusManifest`].
    #[must_use]
    pub fn from_manifest(manifest: &BusManifest) -> Self {
        Self {
            sim: VehicleSimulation::from_manifest(manifest),
        }
    }

    /// Provides immutable access to active driver inputs.
    #[must_use]
    pub fn inputs(&self) -> &VehicleInputs {
        &self.sim.inputs
    }

    /// Provides mutable access to active driver inputs.
    pub fn inputs_mut(&mut self) -> &mut VehicleInputs {
        &mut self.sim.inputs
    }

    /// Returns the forward road speed in kilometers per hour.
    #[must_use]
    pub fn forward_speed_km_h(&self) -> f32 {
        self.sim.chassis.forward_speed_m_s() * 3.6
    }

    /// Returns the engine rotational speed in revolutions per minute.
    #[must_use]
    pub fn engine_rpm(&self) -> f32 {
        self.sim.drivetrain.current_rpm
    }

    /// Returns the current transmission gear selector position.
    #[must_use]
    pub fn gear(&self) -> GearPosition {
        self.sim.inputs.gear()
    }

    /// Returns slice of axle physical states.
    #[must_use]
    pub fn axles(&self) -> &[AxleState] {
        &self.sim.axles
    }
}

/// Marker component denoting the vehicle currently piloted by the local user.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerVehicle;

/// Tuning parameters for smoothing keyboard driver inputs.
#[derive(Component, Clone, Copy, Debug)]
pub struct VehicleInputConfig {
    /// Acceleration rate of throttle input application in units/sec.
    pub throttle_rate: f32,
    /// Acceleration rate of service brake pedal application in units/sec.
    pub brake_rate: f32,
    /// Angular rate of steering lock application in units/sec.
    pub steering_rate: f32,
    /// Rate at which steering self-centers when no steering key is pressed.
    pub steering_return_rate: f32,
}

impl Default for VehicleInputConfig {
    fn default() -> Self {
        Self {
            throttle_rate: 2.5,
            brake_rate: 3.5,
            steering_rate: 2.0,
            steering_return_rate: 3.0,
        }
    }
}

/// Lateral side position of an individual road wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelSide {
    /// Left road wheel (+lateral in vehicle coordinates).
    Left,
    /// Right road wheel (-lateral in vehicle coordinates).
    Right,
}

/// Component attached to a child entity representing a 3D road wheel mesh.
#[derive(Component, Clone, Copy, Debug)]
pub struct WheelVisual {
    /// Zero-based index of the parent vehicle axle.
    pub axle_index: usize,
    /// Lateral axle side (Left or Right).
    pub side: WheelSide,
    /// Local rest translation vector relative to the vehicle root entity.
    pub rest_translation: Vec3,
}

impl WheelVisual {
    /// Creates a new wheel visual component.
    #[must_use]
    pub fn new(axle_index: usize, side: WheelSide, rest_translation: Vec3) -> Self {
        Self {
            axle_index,
            side,
            rest_translation,
        }
    }
}

/// Operating view mode for the vehicle camera rig.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VehicleCameraMode {
    /// Exterior orbit chase camera tracking behind the chassis.
    #[default]
    ExteriorChase,
    /// Interior cockpit camera positioned at the driver's eye level.
    InteriorCockpit,
}

/// Camera tracking rig component that binds a camera entity to follow a [`PlayerVehicle`].
#[derive(Component, Clone, Debug)]
pub struct VehicleCameraRig {
    /// Active camera perspective mode.
    pub mode: VehicleCameraMode,
    /// Chase camera distance offset behind the vehicle in meters.
    pub chase_distance: f32,
    /// Chase camera height offset above the chassis in meters.
    pub chase_height: f32,
    /// Cockpit eye viewpoint offset in local vehicle space.
    pub cockpit_offset: Vec3,
    /// Exponential smoothing factor for chase following (higher = snappier).
    pub damping: f32,
}

impl Default for VehicleCameraRig {
    fn default() -> Self {
        Self {
            mode: VehicleCameraMode::ExteriorChase,
            chase_distance: 12.0,
            chase_height: 4.2,
            cockpit_offset: Vec3::new(-0.7, 1.8, -4.2),
            damping: 8.0,
        }
    }
}

/// Systems that poll desktop keyboard inputs and translate them to [`VehicleInputs`].
pub fn vehicle_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut VehicleController, Option<&VehicleInputConfig>), With<PlayerVehicle>>,
) {
    let dt = time.delta_seconds();
    if dt <= 0.0 {
        return;
    }

    let default_config = VehicleInputConfig::default();

    for (mut controller, config_opt) in &mut query {
        let config = config_opt.unwrap_or(&default_config);
        let inputs = controller.inputs_mut();

        // 1. Throttle pedal (W or ArrowUp)
        let throttle_pressed =
            keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp);
        let current_throttle = inputs.throttle();
        if throttle_pressed {
            inputs.set_throttle((current_throttle + config.throttle_rate * dt).min(1.0));
        } else {
            inputs.set_throttle((current_throttle - config.throttle_rate * 2.5 * dt).max(0.0));
        }

        // 2. Service brake pedal (S or ArrowDown)
        let brake_pressed = keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown);
        let current_brake = inputs.service_brake();
        if brake_pressed {
            inputs.set_service_brake((current_brake + config.brake_rate * dt).min(1.0));
        } else {
            inputs.set_service_brake((current_brake - config.brake_rate * 2.5 * dt).max(0.0));
        }

        // 3. Steering wheel (A/D or ArrowLeft/ArrowRight)
        let steer_left = keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft);
        let steer_right = keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight);
        let current_steer = inputs.steering();

        if steer_left && !steer_right {
            inputs.set_steering((current_steer - config.steering_rate * dt).max(-1.0));
        } else if steer_right && !steer_left {
            inputs.set_steering((current_steer + config.steering_rate * dt).min(1.0));
        } else if current_steer > 0.0 {
            inputs.set_steering((current_steer - config.steering_return_rate * dt).max(0.0));
        } else if current_steer < 0.0 {
            inputs.set_steering((current_steer + config.steering_return_rate * dt).min(0.0));
        }

        // 4. Parking brake toggle (Space)
        // Shift+Space is the camera reset chord, not the parking brake.
        let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
        if keyboard.just_pressed(KeyCode::Space) && !shift {
            let current_park = inputs.is_parking_brake_engaged();
            inputs.set_parking_brake(!current_park);
        }

        // 5. Gear selector (E = Drive, N = Neutral, R = Reverse; digits belong to the camera modes)
        if keyboard.just_pressed(KeyCode::KeyE) {
            inputs.set_gear(GearPosition::Drive);
        } else if keyboard.just_pressed(KeyCode::KeyN) {
            inputs.set_gear(GearPosition::Neutral);
        } else if keyboard.just_pressed(KeyCode::KeyR) {
            inputs.set_gear(GearPosition::Reverse);
        }
    }
}

/// Fixed-update system executing deterministic vehicle physics simulation ticks.
pub fn vehicle_physics_step_system(time: Res<Time>, mut query: Query<&mut VehicleController>) {
    let dt = time.delta_seconds();
    if dt <= 0.0 {
        return;
    }

    for mut controller in &mut query {
        controller.sim.step(dt);
    }
}

/// System synchronizing chassis simulation coordinates to Bevy [`Transform`] components.
pub fn vehicle_transform_sync_system(mut query: Query<(&VehicleController, &mut Transform)>) {
    for (controller, mut transform) in &mut query {
        let sim_pos = controller.sim.chassis.position;
        let sim_elevation = controller.sim.chassis.elevation_z;

        // Convert OMSI simulation position (Z-up) to Bevy space (Y-up)
        let bevy_pos = omsi_to_bevy_pos(Vec3::new(sim_pos.x, sim_pos.y, sim_elevation));
        transform.translation = bevy_pos;

        // Yaw heading: In OMSI, +yaw turns left around Z; in Bevy, -rotation turns right around +Y
        let heading_rad = controller.sim.chassis.heading_rad;
        transform.rotation = Quat::from_rotation_y(-heading_rad);
    }
}

/// System animating child wheel entity transforms according to axle steering and spin rotation.
pub fn wheel_transform_sync_system(
    vehicles: Query<(&VehicleController, &Children)>,
    mut wheels: Query<(&WheelVisual, &mut Transform)>,
) {
    for (controller, children) in &vehicles {
        for &child_entity in children {
            if let Ok((visual, mut transform)) = wheels.get_mut(child_entity) {
                if let Some(axle) = controller.sim.axles.get(visual.axle_index) {
                    let wheel = match visual.side {
                        WheelSide::Left => &axle.left_wheel,
                        WheelSide::Right => &axle.right_wheel,
                    };

                    let steer_rot = Quat::from_rotation_y(-wheel.steer_angle_rad);
                    let roll_rot = Quat::from_rotation_x(-wheel.rotation_rad);

                    transform.rotation = steer_rot * roll_rot;
                    transform.translation = visual.rest_translation;
                }
            }
        }
    }
}

/// System tracking player vehicle position and adjusting camera view mode.
pub fn vehicle_camera_system(
    time: Res<Time>,
    player_query: Query<(&Transform, &VehicleController), With<PlayerVehicle>>,
    mut camera_query: Query<(&mut Transform, &mut VehicleCameraRig), Without<PlayerVehicle>>,
) {
    let dt = time.delta_seconds();

    let Ok((vehicle_tf, _controller)) = player_query.get_single() else {
        return;
    };

    for (mut cam_tf, rig) in &mut camera_query {
        match rig.mode {
            VehicleCameraMode::ExteriorChase => {
                let forward = vehicle_tf.forward();
                let target_pos = vehicle_tf.translation - forward * rig.chase_distance
                    + Vec3::new(0.0, rig.chase_height, 0.0);

                let smooth_factor = (rig.damping * dt).clamp(0.0, 1.0);
                cam_tf.translation = cam_tf.translation.lerp(target_pos, smooth_factor);

                let look_target = vehicle_tf.translation + Vec3::new(0.0, 1.5, 0.0);
                cam_tf.look_at(look_target, Vec3::Y);
            }
            VehicleCameraMode::InteriorCockpit => {
                let cockpit_world_pos = vehicle_tf.transform_point(rig.cockpit_offset);
                cam_tf.translation = cockpit_world_pos;
                cam_tf.rotation = vehicle_tf.rotation;
            }
        }
    }
}

/// Bevy plugin bundling vehicle physics simulation, input systems, and transform sync.
#[derive(Default)]
pub struct OptsVehiclePlugin;

impl Plugin for OptsVehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, vehicle_physics_step_system)
            .add_systems(
                Update,
                (
                    vehicle_input_system
                        .run_if(crate::camera::vehicle_accepts_input)
                        .run_if(crate::reporting::not_modal_active),
                    vehicle_transform_sync_system,
                    wheel_transform_sync_system,
                ),
            )
            .add_systems(
                PostUpdate,
                vehicle_camera_system.run_if(crate::camera::rig_drives_camera),
            );
    }
}
