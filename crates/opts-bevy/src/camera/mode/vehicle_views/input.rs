//! Vehicle view input: the wheel zoom and the drag that turn the external and cockpit views.

use super::*;

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
                bus.driver_camera_count()
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
