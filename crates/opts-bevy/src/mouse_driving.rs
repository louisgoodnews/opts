//! Driving with the mouse (M-0143): the cursor is the pedals and the steering wheel.
//!
//! Pressing the mouse wheel switches the mode on and off. While it is on, the cursor's height
//! is the pedals (the upper half of the window is throttle, full at the top edge; the lower half
//! is the service brake, full at the bottom edge; the middle line is neither) and its width is the
//! steering (full lock at the left and right edges, straight at the middle). A right *click*
//! (press and release without moving) frees the mouse for the cockpit and the interface until the
//! next right click; the pedals are released meanwhile and the keyboard drives as usual. A right
//! *drag* still turns the camera.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::vehicle_sim::{PlayerVehicle, VehicleController};

/// Share of the half window around the middle in which the mouse does not move the control.
pub const DEAD_ZONE: f32 = 0.04;
/// The most a right click may move and still be a click (logical pixels).
const CLICK_SLOP_PX: f32 = 6.0;
/// The longest a right click may last (seconds).
const CLICK_MAX_S: f32 = 0.4;

/// The state of the mouse driving mode.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct MouseDriving {
    /// The setting `controls.mouse_driving`: whether the wheel press does anything.
    pub enabled: bool,
    /// The mode is on.
    pub on: bool,
    /// The mouse was freed with a right click: it does not drive until the next one.
    pub freed: bool,
    /// The pedals and the steering are being written by the mouse right now.
    pub steering: bool,
    /// Right press: where and when it began.
    right_down: Option<(Option<Vec2>, f32)>,
    /// Wheel press: where and when it began (a drag with the wheel pans the external camera).
    middle_down: Option<(Option<Vec2>, f32)>,
}

impl Default for MouseDriving {
    fn default() -> Self {
        Self {
            enabled: true,
            on: false,
            freed: false,
            steering: false,
            right_down: None,
            middle_down: None,
        }
    }
}

impl MouseDriving {
    /// Whether the mouse owns the pedals and the steering (mode on, not freed).
    #[must_use]
    pub fn drives(&self) -> bool {
        self.enabled && self.on && !self.freed
    }
}

/// Throttle, brake and steering of a cursor position in a window of `size`.
///
/// `(throttle, brake, steering)`: the upper half gives throttle, the lower half brake, both 0 to
/// 1 with a dead zone around the middle; steering is -1 (left) to 1 (right).
#[must_use]
pub fn axes_of(cursor: Vec2, size: Vec2) -> (f32, f32, f32) {
    if size.x <= 0.0 || size.y <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let shape = |offset: f32| {
        let m = offset.abs();
        if m <= DEAD_ZONE {
            0.0
        } else {
            offset.signum() * ((m - DEAD_ZONE) / (1.0 - DEAD_ZONE)).min(1.0)
        }
    };
    let y = shape((0.5 - cursor.y / size.y) * 2.0);
    let x = shape((cursor.x / size.x - 0.5) * 2.0);
    (y.max(0.0), (-y).max(0.0), x.clamp(-1.0, 1.0))
}

/// The wheel press switches the mode, a right click frees and captures the mouse.
pub fn mouse_driving_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<MouseDriving>,
) {
    if !state.enabled {
        if state.on {
            state.on = false;
        }
        return;
    }
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    let now = time.elapsed_secs();
    let is_click = |down: Option<(Option<Vec2>, f32)>| {
        let Some((from, at)) = down else { return false };
        let near = match (from, cursor) {
            (Some(from), Some(to)) => from.distance(to) <= CLICK_SLOP_PX,
            // The cursor is not readable (not over the window): the time alone decides.
            _ => true,
        };
        near && now - at <= CLICK_MAX_S
    };
    if mouse.just_pressed(MouseButton::Middle) {
        state.middle_down = Some((cursor, now));
    }
    if mouse.just_released(MouseButton::Middle) {
        let down = state.middle_down.take();
        if is_click(down) {
            state.on = !state.on;
            state.freed = false;
        }
    }
    if mouse.just_pressed(MouseButton::Right) {
        state.right_down = Some((cursor, now));
    }
    if mouse.just_released(MouseButton::Right) {
        let down = state.right_down.take();
        if is_click(down) && state.on {
            state.freed = !state.freed;
        }
    }
}

/// Writes the pedals and the steering of the player's vehicle from the cursor (after the keyboard
/// path, which it overrides while the mode drives).
pub fn mouse_driving_axes(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<MouseDriving>,
    mut query: Query<&mut VehicleController, With<PlayerVehicle>>,
) {
    let cursor = windows.single().ok().and_then(|w| w.cursor_position().map(|c| (c, w.size())));
    let drives = state.drives() && cursor.is_some();
    if state.steering != drives && !drives {
        release(&mut query);
    }
    if state.steering != drives {
        state.steering = drives;
    }
    let Some((cursor, size)) = cursor.filter(|_| drives) else {
        return;
    };
    let (throttle, brake, steer) = axes_of(cursor, size);
    for mut controller in &mut query {
        let inputs = controller.inputs_mut();
        inputs.set_throttle(throttle);
        inputs.set_service_brake(brake);
        inputs.set_steering(steer);
    }
}

/// Lets go of the pedals and the wheel (the mouse stopped driving).
fn release(query: &mut Query<&mut VehicleController, With<PlayerVehicle>>) {
    for mut controller in query {
        let inputs = controller.inputs_mut();
        inputs.set_throttle(0.0);
        inputs.set_service_brake(0.0);
        inputs.set_steering(0.0);
    }
}

/// While the vehicle takes no input (free camera, a dialog) the mouse does not drive: the pedals
/// that it held are released once.
pub fn mouse_driving_gate_closed(
    mut state: ResMut<MouseDriving>,
    mut query: Query<&mut VehicleController, With<PlayerVehicle>>,
) {
    if state.steering {
        release(&mut query);
        state.steering = false;
    }
}

#[cfg(test)]
mod tests;
