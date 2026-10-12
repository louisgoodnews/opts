//! Key 1 ego perspective: the camera sits at the player human's eyes and looks with the mouse.

use bevy::input::mouse::MouseMotion;
use bevy::input::ButtonInput;
use bevy::math::{EulerRot, Quat};
use bevy::prelude::*;

use crate::camera::state::{CameraMode, CameraState};
use crate::human::PlayerHead;

/// Mouse look relative to the head (yaw ±120°, pitch ±80°). Hold Left Alt to free the cursor.
pub fn update_ego_look(
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: MessageReader<MouseMotion>,
    mut state: ResMut<CameraState>,
) {
    if state.mode != CameraMode::Ego {
        return;
    }
    if keys.pressed(KeyCode::AltLeft) {
        motion.clear();
        return;
    }
    let delta: bevy::math::Vec2 = motion.read().map(|e| e.delta).sum();
    let sens = state.mouse_sensitivity;
    state.ego_yaw =
        (state.ego_yaw - delta.x * sens).clamp(-120.0_f32.to_radians(), 120.0_f32.to_radians());
    state.ego_pitch =
        (state.ego_pitch - delta.y * sens).clamp(-80.0_f32.to_radians(), 80.0_f32.to_radians());
}

/// Places the camera on the head anchor, rotated by the ego look angles.
pub fn update_ego_camera(
    state: Res<CameraState>,
    head: Query<&GlobalTransform, With<PlayerHead>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    if state.mode != CameraMode::Ego {
        return;
    }
    let (Ok(head), Ok(mut cam)) = (head.single(), camera.single_mut()) else {
        return;
    };
    let (_, rot, pos) = head.to_scale_rotation_translation();
    cam.translation = pos;
    cam.rotation = rot * Quat::from_euler(EulerRot::YXZ, state.ego_yaw, state.ego_pitch, 0.0);
}
