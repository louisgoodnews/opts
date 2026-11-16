//! Key 2 cockpit, key 4 passenger and key 3 external views of the player's vehicle.
//!
//! Cockpit and passenger cameras are defined per vehicle in its `.bus` file
//! (`[add_camera_driver]`, `[add_camera_pax]`, `[set_camera_std]`, `[set_camera_outside_center]`)
//! and arrive on the [`BusVehicle`] component; nothing here is hard-coded.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::ButtonInput;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_core::formats::vehicles::BusCamera;

use crate::articulation::ArticulatedSegment;
use crate::camera::state::{cycle_index, CameraMode, CameraState};
use crate::camera::viewpoint::ViewpointState;
use crate::coords::{camera_preset_to_bevy_transform, omsi_to_bevy_pos};
use crate::vehicle::BusVehicle;
use crate::vehicle_sim::PlayerVehicle;

/// Default vertical field of view when a view does not define one.
pub const DEFAULT_FOV_Y: f32 = std::f32::consts::FRAC_PI_4;

/// Minimum and maximum external orbit distance in metres.
pub const EXTERNAL_ZOOM_RANGE: (f32, f32) = (0.5, 50.0);

/// Default external pitch (−15°) used by the reset.
pub const EXTERNAL_DEFAULT_PITCH_DEG: f32 = -15.0;

/// Default external orbit distance (m) and the one the reset returns to: far enough to see the
/// whole rear of a 12 m bus.
pub const EXTERNAL_DEFAULT_ZOOM_M: f32 = 14.0;

/// Converts a horizontal field of view to a vertical one for `aspect` (width / height).
#[must_use]
pub fn fov_y_from_horizontal(fov_h_deg: f32, aspect: f32) -> f32 {
    2.0 * ((fov_h_deg.to_radians() * 0.5).tan() / aspect.max(0.1)).atan()
}

/// Camera pose of `cam` in world space for a vehicle at `vehicle`, with look offsets applied.
#[must_use]
pub fn preset_world_transform(
    vehicle: &GlobalTransform,
    cam: &BusCamera,
    yaw: f32,
    pitch: f32,
) -> Transform {
    let local = camera_preset_to_bevy_transform(
        Vec3::new(cam.x, cam.y, cam.z),
        cam.dist,
        cam.yaw_deg,
        cam.pitch_deg,
    );
    let (_, vrot, vpos) = vehicle.to_scale_rotation_translation();
    Transform {
        translation: vpos + vrot * local.translation,
        rotation: vrot * local.rotation * Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0),
        scale: Vec3::ONE,
    }
}

pub(super) fn window_aspect(windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    windows
        .single()
        .map_or(16.0 / 9.0, |w| w.width() / w.height().max(1.0))
}

pub(super) fn set_fov(projection: &mut Projection, fov_y: f32) {
    if let Projection::Perspective(p) = projection {
        if (p.fov - fov_y).abs() > 1e-4 {
            p.fov = fov_y;
        }
    }
}

mod input;
mod update;

pub use input::*;
pub use update::*;

#[cfg(test)]
mod tests;
