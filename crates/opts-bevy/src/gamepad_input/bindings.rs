//! The pad bindings: the settings texts read into axes and buttons.

use bevy::input::gamepad::{GamepadAxis, GamepadButton};
use bevy::prelude::*;

use super::parse::{parse_axis, parse_button};

/// The action id of the parking brake, as `crate::action_keys` names it.
pub const PARKING_BRAKE: &str = "parking_brake";

/// The pad settings: (id, setting, default text). `throttle`, `brake` and `steering` are axes; the
/// `parking_brake` row is a button of that action.
// TODO(owner): the defaults are guesses; no pad or wheel was read to choose them. A wheel's
// pedals and steering axis differ between makes.
pub const PAD_SETTINGS: [(&str, &str, &str); 4] = [
    ("throttle", "controls.pad.throttle", "RightZ"),
    ("brake", "controls.pad.brake", "LeftZ"),
    ("steering", "controls.pad.steering", "LeftStickX"),
    (PARKING_BRAKE, "controls.pad.parking_brake", "West"),
];

/// What an axis drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadTarget {
    /// The throttle pedal, 0 to 1.
    Throttle,
    /// The service brake pedal, 0 to 1.
    Brake,
    /// The steering wheel, -1 (left) to 1 (right).
    Steering,
}

/// One axis binding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBinding {
    /// What the axis drives.
    pub target: PadTarget,
    /// The axis of the pad or wheel.
    pub axis: GamepadAxis,
    /// Whether the axis runs backwards (a `-` in the setting).
    pub invert: bool,
}

/// One button binding: the button presses the vehicle action `action` (its id).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonBinding {
    /// The action id.
    pub action: &'static str,
    /// The button of the pad or wheel.
    pub button: GamepadButton,
}

/// The pad bindings of the vehicle. Filled from the settings (`controls.pad.<id>`) by the game.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct PadBindings {
    /// The axes that drive the pedals and the steering.
    pub axes: Vec<AxisBinding>,
    /// The buttons that do vehicle actions.
    pub buttons: Vec<ButtonBinding>,
}

impl Default for PadBindings {
    fn default() -> Self {
        Self::from_lookup(|_| None)
    }
}

impl PadBindings {
    /// The bindings of the settings text that `lookup` gives (`None`: the default). An empty text
    /// unbinds; a text that names no axis or button is logged and the default stands.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let mut out = Self {
            axes: Vec::new(),
            buttons: Vec::new(),
        };
        for (id, setting, default) in PAD_SETTINGS {
            let text = lookup(setting).unwrap_or_else(|| default.to_owned());
            let text = text.trim();
            if id == PARKING_BRAKE {
                if let Some(button) = read(setting, text, default, parse_button) {
                    out.buttons.push(ButtonBinding { action: id, button });
                }
                continue;
            }
            let target = match id {
                "throttle" => PadTarget::Throttle,
                "brake" => PadTarget::Brake,
                _ => PadTarget::Steering,
            };
            if let Some((axis, invert)) = read(setting, text, default, parse_axis) {
                out.axes.push(AxisBinding {
                    target,
                    axis,
                    invert,
                });
            }
        }
        out
    }
}

/// The parsed setting, `None` when it is empty (unbound) or the default when it is not understood.
fn read<T>(
    setting: &str,
    text: &str,
    default: &str,
    parse: impl Fn(&str) -> Option<T>,
) -> Option<T> {
    if text.is_empty() {
        return None;
    }
    parse(text).or_else(|| {
        warn!(
            setting = setting,
            text = text,
            "pad binding not understood: the default stands"
        );
        parse(default)
    })
}
