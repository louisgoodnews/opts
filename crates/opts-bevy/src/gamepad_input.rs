//! Gamepad and steering wheel input for the player's vehicle (ISS-049, M-0035).
//!
//! A wheel is a gamepad to Bevy, so one path serves both. The bindings are the settings
//! `controls.pad.<id>` (see [`PAD_SETTINGS`]), read through the settings registry like the key
//! bindings of `crate::action_keys`: an axis is named as Bevy names it (`RightZ`, `LeftStickX`,
//! `Axis7` for an other axis), a leading `-` inverts it, an empty text unbinds it. The pedals and
//! the steering are written after the keyboard and the mouse; a pad axis past its dead zone
//! overrides them, and at rest they stay as the keyboard leaves them. Rail vehicles have no
//! pedals and take no pad axes.

mod bindings;
mod parse;
mod systems;

pub use bindings::{
    AxisBinding, ButtonBinding, PadBindings, PadTarget, PAD_SETTINGS, PARKING_BRAKE,
};
pub use parse::{axis_text, button_text, parse_axis, parse_button};
pub use systems::{pad_button_system, pad_driving_system, AXIS_DEAD_ZONE};
