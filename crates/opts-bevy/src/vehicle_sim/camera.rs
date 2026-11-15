//! Camera rig for the vehicle: exterior chase and interior cockpit modes.

use bevy::prelude::*;

use super::controller::{PlayerVehicle, VehicleController};

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

/// System tracking player vehicle position and adjusting camera view mode.
pub fn vehicle_camera_system(
    time: Res<Time>,
    player_query: Query<(&Transform, &VehicleController), With<PlayerVehicle>>,
    mut camera_query: Query<(&mut Transform, &mut VehicleCameraRig), Without<PlayerVehicle>>,
) {
    let dt = time.delta_secs();

    let Ok((vehicle_tf, _controller)) = player_query.single() else {
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
