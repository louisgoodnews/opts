//! Key 5 free-roam camera: WASD + mouse look, wheel-with-Shift speed control.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::ButtonInput;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::camera::state::{scale_free_roam_speed, CameraMode, CameraState};

/// Unit movement direction for the pressed keys, relative to the camera orientation.
///
/// W/S move along the look direction, A/D strafe, E/Q move up/down along world Y.
#[must_use]
pub fn movement_direction(rotation: Quat, forward: f32, right: f32, up: f32) -> Vec3 {
    let dir = rotation * (Vec3::NEG_Z * forward + Vec3::X * right) + Vec3::Y * up;
    dir.normalize_or_zero()
}

/// Orientation for a free-roam yaw/pitch pair (radians).
#[must_use]
pub fn free_roam_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

/// Moves and orients the camera while in free-roam mode.
#[allow(clippy::too_many_arguments)]
pub fn update_free_roam_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut state: ResMut<CameraState>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    if state.mode != CameraMode::FreeRoam {
        motion.clear();
        wheel.clear();
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let look_enabled = !keys.pressed(KeyCode::AltLeft);

    let notches: f32 = wheel.read().map(|e| e.y).sum();
    if shift && notches != 0.0 {
        let new = scale_free_roam_speed(state.free_roam_speed, notches);
        if (new - state.free_roam_speed).abs() > f32::EPSILON {
            tracing::debug!(
                from = state.free_roam_speed,
                to = new,
                "free-roam speed changed"
            );
        }
        state.free_roam_speed = new;
    }

    let delta: bevy::math::Vec2 = if look_enabled {
        motion.read().map(|e| e.delta).sum()
    } else {
        motion.clear();
        bevy::math::Vec2::ZERO
    };
    let sens = state.mouse_sensitivity;
    state.free_roam_yaw -= delta.x * sens;
    state.free_roam_pitch = (state.free_roam_pitch - delta.y * sens)
        .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());

    let Ok(mut tf) = camera.single_mut() else {
        return;
    };
    tf.rotation = free_roam_rotation(state.free_roam_yaw, state.free_roam_pitch);

    let axis = |pos: KeyCode, neg: KeyCode| {
        f32::from(u8::from(keys.pressed(pos))) - f32::from(u8::from(keys.pressed(neg)))
    };
    let dir = movement_direction(
        tf.rotation,
        axis(KeyCode::KeyW, KeyCode::KeyS),
        axis(KeyCode::KeyD, KeyCode::KeyA),
        axis(KeyCode::KeyE, KeyCode::KeyQ),
    );
    tf.translation += dir * state.free_roam_speed * time.delta_secs();
}

/// Locks and hides the cursor while flying the free camera or looking from the ego view.
///
/// The lock needs a 3D camera to exist (so never in the menus, where free roam is merely the
/// default mode) and is released while Left Alt is held, so HUD buttons stay reachable.
pub fn manage_free_roam_cursor(
    state: Res<CameraState>,
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<(), With<Camera3d>>,
    modal: Option<Res<crate::reporting::ModalActive>>,
    mut windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut locked: Local<bool>,
) {
    // The cursor is released while the report modal is open so its buttons are reachable.
    let want_lock = matches!(state.mode, CameraMode::FreeRoam | CameraMode::Ego)
        && !cameras.is_empty()
        && !modal.is_some_and(|m| m.0)
        && !keys.pressed(KeyCode::AltLeft);
    if want_lock == *locked {
        return;
    }
    *locked = want_lock;
    for mut window in &mut windows {
        window.grab_mode = if want_lock {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        window.visible = !want_lock;
    }
    tracing::debug!(locked = want_lock, "free-roam cursor capture changed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_is_camera_relative() {
        let facing_east = free_roam_rotation(-std::f32::consts::FRAC_PI_2, 0.0);
        let d = movement_direction(facing_east, 1.0, 0.0, 0.0);
        assert!(
            (d - Vec3::X).length() < 1e-5,
            "W moves along the look direction, got {d:?}"
        );
        let up = movement_direction(Quat::IDENTITY, 0.0, 0.0, 1.0);
        assert_eq!(up, Vec3::Y);
        assert_eq!(
            movement_direction(Quat::IDENTITY, 0.0, 0.0, 0.0),
            Vec3::ZERO
        );
    }
}
