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

/// Default external pitch (−15°) and zoom (8 m) used by the reset.
pub const EXTERNAL_DEFAULT_PITCH_DEG: f32 = -15.0;

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

fn window_aspect(windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    windows
        .single()
        .map_or(16.0 / 9.0, |w| w.width() / w.height().max(1.0))
}

fn set_fov(projection: &mut Projection, fov_y: f32) {
    if let Projection::Perspective(p) = projection {
        if (p.fov - fov_y).abs() > 1e-4 {
            p.fov = fov_y;
        }
    }
}

/// Wheel handling for the vehicle views: cycles sub-cameras (wrapping) in modes 2/4, zooms in mode 3.
pub fn handle_vehicle_view_wheel(
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    vehicle: Query<&BusVehicle, With<PlayerVehicle>>,
    mut state: ResMut<CameraState>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let notches: f32 = wheel.read().map(|e| e.y).sum();
    if notches == 0.0 || shift {
        return;
    }
    let steps = notches.round() as i32;
    match state.mode {
        CameraMode::Cockpit | CameraMode::Passenger => {
            let Ok(bus) = vehicle.single() else {
                return;
            };
            let len = if state.mode == CameraMode::Cockpit {
                bus.driver_cameras.len()
            } else {
                bus.pax_camera_count()
            };
            let next = cycle_index(state.sub_camera_index, steps, len);
            if next != state.sub_camera_index {
                tracing::debug!(mode = ?state.mode, from = state.sub_camera_index, to = next, total = len, "sub-camera changed");
                state.sub_camera_index = next;
                state.view_yaw = 0.0;
                state.view_pitch = 0.0;
            }
        }
        CameraMode::External => {
            let zoom = (state.external_zoom * 0.9_f32.powf(notches))
                .clamp(EXTERNAL_ZOOM_RANGE.0, EXTERNAL_ZOOM_RANGE.1);
            state.external_zoom = zoom;
        }
        _ => {}
    }
}

/// Mouse handling: right-drag looks around in modes 2/4; left-drag orbits and middle-drag pans in mode 3.
///
/// Panning translates the orbit centre in the camera's right/up plane at `zoom × 0.005` m/px, so
/// it feels the same at close and far range, and leaves yaw, pitch and zoom untouched.
pub fn handle_vehicle_view_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut state: ResMut<CameraState>,
    camera: Query<&Transform, With<crate::MainCamera>>,
) {
    let delta: bevy::math::Vec2 = motion.read().map(|e| e.delta).sum();
    if delta == bevy::math::Vec2::ZERO {
        return;
    }
    let sens = state.mouse_sensitivity;
    match state.mode {
        CameraMode::Cockpit | CameraMode::Passenger if mouse.pressed(MouseButton::Right) => {
            state.view_yaw = (state.view_yaw - delta.x * sens).clamp(-2.1, 2.1);
            state.view_pitch = (state.view_pitch - delta.y * sens).clamp(-1.4, 1.4);
        }
        CameraMode::External => {
            if mouse.pressed(MouseButton::Left) {
                state.external_yaw -= delta.x * sens;
                state.external_pitch = (state.external_pitch - delta.y * sens).clamp(-1.5, 1.5);
            } else if mouse.pressed(MouseButton::Middle) {
                let Ok(cam) = camera.single() else { return };
                let rate = state.external_zoom * 0.005;
                let (right, up) = (cam.rotation * Vec3::X, cam.rotation * Vec3::Y);
                state.pan_offset += right * (-delta.x * rate) + up * (delta.y * rate);
            }
        }
        _ => {}
    }
}

/// The player's vehicle, which is not the camera.
type PlayerNotCamera = (With<PlayerVehicle>, Without<crate::MainCamera>);

/// Key 2: the active `[add_camera_driver]` preset, composed with the vehicle's world transform.
pub fn update_cockpit_camera(
    state: Res<CameraState>,
    vehicle: Query<(&Transform, &BusVehicle), PlayerNotCamera>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut viewpoint: ResMut<ViewpointState>,
    mut camera: Query<(&mut Transform, &mut Projection), With<crate::MainCamera>>,
) {
    if state.mode != CameraMode::Cockpit {
        return;
    }
    let (Ok((vehicle_tf, bus)), Ok((mut tf, mut proj))) = (vehicle.single(), camera.single_mut())
    else {
        return;
    };
    let Some(cam) = bus.driver_cameras.get(state.sub_camera_index) else {
        return;
    };
    // The vehicle's own (root) transform of this frame, not its `GlobalTransform`: this system
    // runs before the propagation (see `OptsCameraPlugin`), when the global one still holds the
    // last frame's pose. The camera then moves with the vehicle in the same frame.
    let gt = GlobalTransform::from(*vehicle_tf);
    *tf = preset_world_transform(&gt, cam, state.view_yaw, state.view_pitch);
    // Eye position and direction are the vehicle's own `[add_camera_driver]` preset (the player
    // character is hidden in this view, see `crate::human::hide_driver_in_seat_views`).
    // The viewpoint editor's offset (right, up, forward in the driver's frame) and FOV.
    if (viewpoint.default_fov_deg - cam.fov_deg).abs() > 1e-4 {
        viewpoint.default_fov_deg = cam.fov_deg;
    }
    let (_, vrot, _) = gt.to_scale_rotation_translation();
    tf.translation += vrot * viewpoint.active.local_offset();
    set_fov(
        &mut proj,
        fov_y_from_horizontal(viewpoint.effective_fov_deg(), window_aspect(&windows)),
    );
}

/// Key 4: the active `[add_camera_pax]` preset, in the frame of the segment (tractor or trailer)
/// that defines it.
pub fn update_passenger_camera(
    state: Res<CameraState>,
    vehicle: Query<(&GlobalTransform, &BusVehicle), With<PlayerVehicle>>,
    segments: Query<&GlobalTransform, Without<PlayerVehicle>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<crate::MainCamera>>,
) {
    if state.mode != CameraMode::Passenger {
        return;
    }
    let (Ok((gt, bus)), Ok((mut tf, mut proj))) = (vehicle.single(), camera.single_mut()) else {
        return;
    };
    let Some((segment, cam)) = bus.pax_camera(state.sub_camera_index) else {
        return;
    };
    let frame = segment.and_then(|e| segments.get(e).ok()).unwrap_or(gt);
    *tf = preset_world_transform(frame, cam, state.view_yaw, state.view_pitch);
    set_fov(
        &mut proj,
        fov_y_from_horizontal(cam.fov_deg, window_aspect(&windows)),
    );
}

/// Key 3: orbit around the vehicle's `[set_camera_outside_center]` (or its origin), plus pan offset.
pub fn update_external_camera(
    state: Res<CameraState>,
    vehicle: Query<(&GlobalTransform, &BusVehicle), With<PlayerVehicle>>,
    segments: Query<&GlobalTransform, (With<ArticulatedSegment>, Without<crate::MainCamera>)>,
    mut camera: Query<(&mut Transform, &mut Projection), With<crate::MainCamera>>,
) {
    if state.mode != CameraMode::External {
        return;
    }
    let (Ok((gt, bus)), Ok((mut tf, mut proj))) = (vehicle.single(), camera.single_mut()) else {
        return;
    };
    let (_, vrot, vpos) = gt.to_scale_rotation_translation();
    let local_centre = bus
        .outside_camera_center
        .map_or(Vec3::ZERO, omsi_to_bevy_pos);
    let centre = vpos + vrot * local_centre + state.pan_offset;
    // Orbit angles are relative to the vehicle's heading so the view turns with it.
    let orbit =
        vrot * Quat::from_euler(EulerRot::YXZ, state.external_yaw, state.external_pitch, 0.0);
    // The camera sits behind the centre looking forward: +Z (back) rotated by the orbit.
    // Articulated chains reach far behind the head unit: stay outside of every segment.
    let reach = segments
        .iter()
        .map(|g| g.translation().distance(centre))
        .filter(|d| *d < 25.0)
        .fold(0.0_f32, f32::max);
    let distance = state
        .external_zoom
        .max(if reach > 0.0 { reach + 6.0 } else { 0.0 });
    tf.translation = centre + orbit * Vec3::Z * distance;
    tf.look_at(centre, Vec3::Y);
    set_fov(&mut proj, DEFAULT_FOV_Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_to_vertical_fov() {
        let v = fov_y_from_horizontal(90.0, 16.0 / 9.0);
        assert!((v.to_degrees() - 58.7).abs() < 0.2, "{}", v.to_degrees());
        assert!((fov_y_from_horizontal(60.0, 1.0).to_degrees() - 60.0).abs() < 1e-3);
    }
}
