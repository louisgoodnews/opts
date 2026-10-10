//! Number-key mode switching, availability tracking and Shift+Space reset.

use bevy::input::ButtonInput;
use bevy::math::{EulerRot, Vec3};
use bevy::prelude::*;

use crate::camera::state::{CameraAvailability, CameraMode, CameraState, FREE_ROAM_MIN_SPEED};
use crate::human::HumanHead;
use crate::vehicle::BusVehicle;
use crate::vehicle_sim::{PlayerVehicle, VehicleCameraMode, VehicleCameraRig};

const MODE_KEYS: [(KeyCode, CameraMode); 5] = [
    (KeyCode::Digit1, CameraMode::Ego),
    (KeyCode::Digit2, CameraMode::Cockpit),
    (KeyCode::Digit3, CameraMode::External),
    (KeyCode::Digit4, CameraMode::Passenger),
    (KeyCode::Digit5, CameraMode::FreeRoam),
];

/// Refreshes which modes can be entered. Modes without a backing implementation stay unavailable.
pub fn update_camera_availability(
    vehicles: Query<&BusVehicle, With<PlayerVehicle>>,
    humans: Query<(), With<HumanHead>>,
    mut avail: ResMut<CameraAvailability>,
) {
    let bus = vehicles.get_single().ok();
    let new = CameraAvailability {
        ego: !humans.is_empty(),
        cockpit: bus.is_some_and(|b| !b.driver_cameras.is_empty()),
        external: bus.is_some(),
        passenger: bus.is_some_and(|b| b.pax_camera_count() > 0),
    };
    if *avail != new {
        *avail = new;
    }
}

/// Records the camera pose as the free-roam spawn and seeds yaw/pitch from it.
fn enter_free_roam(state: &mut CameraState, camera: &Transform) {
    let (yaw, pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
    state.free_roam_spawn = camera.translation;
    state.free_roam_spawn_yaw = yaw;
    state.free_roam_spawn_pitch = pitch;
    state.free_roam_yaw = yaw;
    state.free_roam_pitch = pitch;
    tracing::debug!(position = ?camera.translation, "entered free roam");
}

/// Handles 0 (previous mode) and 1–5 (select mode), honouring availability.
pub fn handle_camera_key_input(
    keys: Res<ButtonInput<KeyCode>>,
    avail: Res<CameraAvailability>,
    mut state: ResMut<CameraState>,
    camera: Query<&Transform, With<Camera3d>>,
    vehicle: Query<&BusVehicle, With<PlayerVehicle>>,
    mut warned: Local<[bool; 2]>,
) {
    let mut requested = None;
    let mut trigger = KeyCode::Digit0;
    if keys.just_pressed(KeyCode::Digit0) {
        requested = Some(state.previous_mode);
    }
    for (key, mode) in MODE_KEYS {
        if keys.just_pressed(key) {
            requested = Some(mode);
            trigger = key;
        }
    }
    let Some(mode) = requested else { return };
    let std_index = vehicle.get_single().ok().map(|b| b.std_camera_index);

    if !avail.allows(mode) {
        match mode {
            CameraMode::Ego if !warned[0] => {
                warned[0] = true;
                tracing::warn!("key 1 ego perspective is unavailable: no player human is loaded");
            }
            CameraMode::Passenger if !warned[1] => {
                warned[1] = true;
                tracing::warn!("key 4 passenger views are unavailable: not implemented yet");
            }
            _ => tracing::debug!(
                ?mode,
                "camera mode unavailable; staying in {:?}",
                state.mode
            ),
        }
        return;
    }

    let from = state.mode;
    state.switch_to(mode);
    if mode == CameraMode::Cockpit {
        // Default view: `[set_camera_std]` of the vehicle, else the first driver camera.
        state.sub_camera_index = std_index.unwrap_or(0);
    }
    if mode == CameraMode::FreeRoam && from != mode {
        if let Ok(tf) = camera.get_single() {
            enter_free_roam(&mut state, tf);
        }
    }
    tracing::debug!(from = ?from, to = ?state.mode, previous = ?state.previous_mode, key = ?trigger, "camera mode changed");
}

/// Mirrors Cockpit/External onto the vehicle camera rig that currently implements them.
pub fn sync_vehicle_rig_mode(state: Res<CameraState>, mut rigs: Query<&mut VehicleCameraRig>) {
    if !state.is_changed() {
        return;
    }
    let target = match state.mode {
        CameraMode::Cockpit => VehicleCameraMode::InteriorCockpit,
        CameraMode::External => VehicleCameraMode::ExteriorChase,
        _ => return,
    };
    for mut rig in &mut rigs {
        if rig.mode != target {
            rig.mode = target;
        }
    }
}

/// Shift+Space: resets the current view to its default without changing mode or `previous_mode`.
pub fn handle_camera_reset_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CameraState>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    vehicle: Query<&BusVehicle, With<PlayerVehicle>>,
) {
    let std_index = vehicle.get_single().ok().map(|b| b.std_camera_index);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if !(shift && keys.just_pressed(KeyCode::Space)) {
        return;
    }
    match state.mode {
        CameraMode::FreeRoam => {
            let before = (
                state.free_roam_speed,
                state.free_roam_yaw,
                state.free_roam_pitch,
            );
            state.free_roam_yaw = 0.0;
            state.free_roam_pitch = 0.0;
            state.free_roam_speed = FREE_ROAM_MIN_SPEED;
            if let Ok(mut tf) = camera.get_single_mut() {
                tf.translation = state.free_roam_spawn;
                tf.rotation = Quat::IDENTITY;
            }
            tracing::debug!(?before, spawn = ?state.free_roam_spawn, "free-roam view reset");
        }
        CameraMode::External => {
            state.external_yaw = 0.0;
            state.external_pitch = -15.0_f32.to_radians();
            state.external_zoom = 8.0;
            state.pan_offset = Vec3::ZERO;
            tracing::debug!("external view reset");
        }
        CameraMode::Ego => {
            state.ego_yaw = 0.0;
            state.ego_pitch = 0.0;
            tracing::debug!("ego view reset to the head's forward direction");
        }
        mode => {
            state.sub_camera_index = if mode == CameraMode::Cockpit {
                std_index.unwrap_or(0)
            } else {
                0
            };
            state.view_yaw = 0.0;
            state.view_pitch = 0.0;
            tracing::debug!(?mode, "view reset");
        }
    }
}
