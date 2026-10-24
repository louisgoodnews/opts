//! Shift+Space and `camera.reset`: every view has a default, and the reset returns the *current*
//! view to it (R-33).
//!
//! | View | Default |
//! | :--- | :--- |
//! | Free roam (5) | the pose recorded when the view was entered, speed 5 m/s |
//! | External (3) | yaw 0, pitch −15°, zoom 14 m, no pan |
//! | Ego (1) | looking along the head's forward direction |
//! | Cockpit (2) | the sub camera in use, look angles 0, the **stored viewpoint** of the vehicle (or the vehicle's own view); an open editor is cancelled |
//! | Passenger (4) | the sub camera in use, look angles 0 |
//!
//! The reset never changes the mode, `previous_mode` or the sub camera.

use bevy::math::{Quat, Vec3};
use bevy::prelude::Transform;

use crate::camera::state::{CameraMode, CameraState, FREE_ROAM_MIN_SPEED};
use crate::camera::viewpoint::ViewpointState;

/// What was reset (the protocol answers with it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewReset {
    /// The view that was reset.
    pub mode: CameraMode,
    /// The sub camera it was reset in (cockpit and passenger), else 0.
    pub sub_camera: usize,
}

/// Resets the current view of `state` to its default. `camera` is the scene camera's transform
/// (free roam moves it back to the recorded pose).
pub fn reset_view(
    state: &mut CameraState,
    camera: Option<&mut Transform>,
    viewpoint: &mut ViewpointState,
) -> ViewReset {
    match state.mode {
        CameraMode::FreeRoam => {
            state.free_roam_yaw = state.free_roam_spawn_yaw;
            state.free_roam_pitch = state.free_roam_spawn_pitch;
            state.free_roam_speed = FREE_ROAM_MIN_SPEED;
            if let Some(tf) = camera {
                tf.translation = state.free_roam_spawn;
                tf.rotation = Quat::from_euler(
                    bevy::math::EulerRot::YXZ,
                    state.free_roam_yaw,
                    state.free_roam_pitch,
                    0.0,
                );
            }
        }
        CameraMode::External => {
            state.external_yaw = 0.0;
            state.external_pitch = -15.0_f32.to_radians();
            state.external_zoom = super::mode::vehicle_views::EXTERNAL_DEFAULT_ZOOM_M;
            state.pan_offset = Vec3::ZERO;
        }
        CameraMode::Ego => {
            state.ego_yaw = 0.0;
            state.ego_pitch = 0.0;
        }
        CameraMode::Cockpit => {
            state.view_yaw = 0.0;
            state.view_pitch = 0.0;
            // Back to the stored viewpoint (or the vehicle's own): a draft is thrown away.
            viewpoint.cancel();
            viewpoint.active = viewpoint.committed;
        }
        CameraMode::Passenger => {
            state.view_yaw = 0.0;
            state.view_pitch = 0.0;
        }
    }
    tracing::debug!(mode = ?state.mode, sub = state.sub_camera_index, "view reset to its default");
    ViewReset {
        mode: state.mode,
        sub_camera: state.sub_camera_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::viewpoint::Viewpoint;

    fn state(mode: CameraMode) -> CameraState {
        CameraState {
            mode,
            sub_camera_index: 3,
            ..CameraState::default()
        }
    }

    #[test]
    fn free_roam_returns_to_the_recorded_pose() {
        let mut s = state(CameraMode::FreeRoam);
        s.free_roam_spawn = Vec3::new(1.0, 2.0, 3.0);
        s.free_roam_spawn_yaw = 0.5;
        s.free_roam_spawn_pitch = -0.1;
        s.free_roam_yaw = 2.0;
        s.free_roam_speed = 90.0;
        let mut tf = Transform::from_xyz(50.0, 50.0, 50.0);
        let r = reset_view(&mut s, Some(&mut tf), &mut ViewpointState::default());
        assert_eq!(tf.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!((s.free_roam_yaw, s.free_roam_pitch), (0.5, -0.1));
        assert_eq!(s.free_roam_speed, FREE_ROAM_MIN_SPEED);
        assert_eq!(r.mode, CameraMode::FreeRoam);
    }

    #[test]
    fn external_returns_to_the_orbit_defaults() {
        let mut s = state(CameraMode::External);
        s.external_yaw = 1.0;
        s.external_pitch = 0.9;
        s.external_zoom = 30.0;
        s.pan_offset = Vec3::ONE;
        reset_view(&mut s, None, &mut ViewpointState::default());
        assert_eq!(s.external_yaw, 0.0);
        assert!((s.external_pitch + 15f32.to_radians()).abs() < 1e-6);
        assert_eq!((s.external_zoom, s.pan_offset), (14.0, Vec3::ZERO));
    }

    #[test]
    fn ego_looks_forward_again() {
        let mut s = state(CameraMode::Ego);
        s.ego_yaw = 1.0;
        s.ego_pitch = -0.5;
        reset_view(&mut s, None, &mut ViewpointState::default());
        assert_eq!((s.ego_yaw, s.ego_pitch), (0.0, 0.0));
    }

    #[test]
    fn cockpit_keeps_its_sub_camera_and_returns_to_the_stored_viewpoint() {
        let mut s = state(CameraMode::Cockpit);
        s.view_yaw = 1.0;
        s.view_pitch = 0.5;
        let stored = Viewpoint {
            y: 0.05,
            ..Viewpoint::default()
        };
        let mut vp = ViewpointState {
            committed: stored,
            active: stored,
            ..ViewpointState::default()
        };
        vp.open();
        vp.adjust(crate::camera::viewpoint::ViewpointRow::X, 9.0);
        let r = reset_view(&mut s, None, &mut vp);
        assert_eq!((s.view_yaw, s.view_pitch), (0.0, 0.0));
        assert_eq!(s.sub_camera_index, 3, "the sub camera stays");
        assert_eq!(r.sub_camera, 3);
        assert!(!vp.editing, "an open editor is cancelled");
        assert_eq!(
            vp.active, stored,
            "the draft is gone, the stored viewpoint stays"
        );
    }

    #[test]
    fn passenger_keeps_its_sub_camera() {
        let mut s = state(CameraMode::Passenger);
        s.view_yaw = -1.0;
        reset_view(&mut s, None, &mut ViewpointState::default());
        assert_eq!((s.view_yaw, s.sub_camera_index), (0.0, 3));
    }
}
