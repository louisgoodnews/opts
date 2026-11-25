//! The pad's axes and buttons written into the player's vehicle.

use std::collections::HashSet;

use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;

use super::bindings::{AxisBinding, PadBindings, PadTarget, PARKING_BRAKE};
use super::parse::{axis_text, button_text};
use crate::rail_controls::RailControls;
use crate::vehicle_sim::{PlayerVehicle, VehicleController};

/// The reading under which a pad axis counts as at rest (stick drift, trigger noise).
// TODO(owner): the dead zone is a guess; a wheel may need a wider one.
pub const AXIS_DEAD_ZONE: f32 = 0.05;

/// The pedal value of a raw reading: 0 at rest, 1 fully pressed.
fn pedal_of(raw: f32) -> f32 {
    ((raw - AXIS_DEAD_ZONE) / (1.0 - AXIS_DEAD_ZONE)).clamp(0.0, 1.0)
}

/// The steering value of a raw reading: -1 (full left) to 1 (full right), straight in the dead zone.
fn steer_of(raw: f32) -> f32 {
    let m = raw.abs();
    if m <= AXIS_DEAD_ZONE {
        0.0
    } else {
        raw.signum() * ((m - AXIS_DEAD_ZONE) / (1.0 - AXIS_DEAD_ZONE)).min(1.0)
    }
}

/// The pedals and the steering from the pad axes. An axis past its dead zone overrides the
/// keyboard and the mouse for its target; at rest it leaves them alone. A device that does not
/// report a bound axis is logged once.
pub fn pad_driving_system(
    pads: Query<(Entity, &Gamepad)>,
    bindings: Res<PadBindings>,
    mut unreadable: Local<HashSet<Entity>>,
    mut query: Query<&mut VehicleController, (With<PlayerVehicle>, Without<RailControls>)>,
) {
    let mut writes: Vec<(PadTarget, f32)> = Vec::new();
    for binding in &bindings.axes {
        for (device, pad) in &pads {
            let Some(raw) = pad.get(binding.axis) else {
                if unreadable.insert(device) {
                    warn!(device = ?device, axis = %axis_text(binding.axis, false), "pad axis cannot be read");
                }
                continue;
            };
            let raw = if binding.invert { -raw } else { raw };
            if raw.abs() <= AXIS_DEAD_ZONE {
                continue;
            }
            let value = pad_value(binding, raw);
            debug!(device = ?device, axis = %axis_text(binding.axis, binding.invert), raw = raw, target = ?binding.target, value = value, "pad axis mapped");
            writes.push((binding.target, value));
            break;
        }
    }
    if writes.is_empty() {
        return;
    }
    for mut controller in &mut query {
        let inputs = controller.inputs_mut();
        for &(target, value) in &writes {
            match target {
                PadTarget::Throttle => inputs.set_throttle(value),
                PadTarget::Brake => inputs.set_service_brake(value),
                PadTarget::Steering => inputs.set_steering(value),
            }
        }
    }
}

/// The value an axis drives: a pedal runs 0 to 1, the steering -1 to 1.
fn pad_value(binding: &AxisBinding, raw: f32) -> f32 {
    match binding.target {
        PadTarget::Steering => steer_of(raw),
        PadTarget::Throttle | PadTarget::Brake => pedal_of(raw),
    }
}

/// The pressed buttons of the pad bindings. The parking brake toggles as its key does.
pub fn pad_button_system(
    pads: Query<(Entity, &Gamepad)>,
    bindings: Res<PadBindings>,
    mut query: Query<&mut VehicleController, (With<PlayerVehicle>, Without<RailControls>)>,
) {
    for binding in &bindings.buttons {
        let Some(device) = pads
            .iter()
            .find(|(_, pad)| pad.just_pressed(binding.button))
            .map(|(device, _)| device)
        else {
            continue;
        };
        debug!(device = ?device, button = %button_text(binding.button), action = binding.action, "pad button mapped");
        if binding.action == PARKING_BRAKE {
            for mut controller in &mut query {
                let inputs = controller.inputs_mut();
                let engaged = inputs.is_parking_brake_engaged();
                inputs.set_parking_brake(!engaged);
            }
        }
    }
}
