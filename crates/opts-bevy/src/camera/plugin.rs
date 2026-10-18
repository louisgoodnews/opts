//! Plugin registering the camera perspective system.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;

use crate::camera::input::{
    handle_camera_key_input, handle_camera_reset_input, sync_vehicle_rig_mode,
    update_camera_availability,
};
use crate::camera::mode::ego::{update_ego_camera, update_ego_look};
use crate::camera::mode::free_roam::{manage_free_roam_cursor, update_free_roam_camera};
use crate::camera::mode::vehicle_views::{
    handle_vehicle_view_drag, handle_vehicle_view_wheel, update_cockpit_camera,
    update_external_camera, update_passenger_camera,
};
use crate::camera::state::{CameraAvailability, CameraMode, CameraState};
use crate::camera::viewpoint::{handle_viewpoint_keys, ViewpointState};
use crate::reporting::not_modal_active;

/// Whether the vehicle camera rig (cockpit / external) should drive the camera this frame.
#[must_use]
pub fn rig_drives_camera(state: Option<Res<CameraState>>) -> bool {
    // The key 2/3/4 views are driven by `camera::mode::vehicle_views`; the legacy rig only runs
    // when no camera state exists (headless tools and examples).
    state.is_none()
}

/// Whether driving keys should reach the vehicle (they must not while flying the free camera).
#[must_use]
pub fn vehicle_accepts_input(
    state: Option<Res<CameraState>>,
    viewpoint: Option<Res<ViewpointState>>,
) -> bool {
    state.is_none_or(|s| s.mode != CameraMode::FreeRoam) && !viewpoint.is_some_and(|v| v.editing)
}

/// Brings the [`GlobalTransform`] of the main camera (and whatever hangs on it) up to the pose the
/// camera systems have just written to its [`Transform`].
///
/// The camera systems run after `TransformSystems::Propagate` because they read the freshly
/// propagated vehicle and head poses. Without this step the camera would be rendered with the
/// global transform of the previous frame while the vehicle it looks out of is drawn at the
/// current one: the view lags `speed x frame time` behind the cab and the lag changes with every
/// frame time, which shows as jitter in the driver view.
pub fn sync_camera_global_transform(
    helper: TransformHelper,
    cameras: Query<Entity, With<crate::MainCamera>>,
    children: Query<&Children>,
    mut globals: Query<&mut GlobalTransform>,
) {
    for camera in &cameras {
        for entity in std::iter::once(camera).chain(children.iter_descendants(camera)) {
            let Ok(pose) = helper.compute_global_transform(entity) else {
                continue;
            };
            if let Ok(mut global) = globals.get_mut(entity) {
                if *global != pose {
                    *global = pose;
                }
            }
        }
    }
}

/// Registers [`CameraState`], mode switching, free roam and cursor handling.
#[derive(Default)]
pub struct OptsCameraPlugin;

impl Plugin for OptsCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MouseMotion>()
            .add_message::<MouseWheel>()
            .init_resource::<CameraState>()
            .init_resource::<CameraAvailability>()
            .init_resource::<ViewpointState>()
            .add_systems(
                Update,
                (
                    crate::human::align_driver_eyes,
                    crate::human::sync_player_head,
                    update_camera_availability,
                    handle_camera_key_input.run_if(not_modal_active),
                    handle_camera_reset_input.run_if(not_modal_active),
                    handle_viewpoint_keys.run_if(not_modal_active),
                    update_free_roam_camera.run_if(not_modal_active),
                    update_ego_look.run_if(not_modal_active),
                    handle_vehicle_view_wheel.run_if(not_modal_active),
                    handle_vehicle_view_drag.run_if(not_modal_active),
                    sync_vehicle_rig_mode,
                    manage_free_roam_cursor,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    update_ego_camera,
                    update_cockpit_camera,
                    update_passenger_camera,
                    update_external_camera,
                )
                    .after(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                sync_camera_global_transform
                    .after(update_ego_camera)
                    .after(update_cockpit_camera)
                    .after(update_passenger_camera)
                    .after(update_external_camera),
            );
    }
}
