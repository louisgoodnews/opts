//! The names of the pad axes and buttons in the settings: Bevy's variant names, `Axis<n>` and
//! `Button<n>` for the ones Bevy has no name for.

use bevy::input::gamepad::{GamepadAxis, GamepadButton};

/// The axis a settings text names, with `true` when a `-` in front inverts it.
#[must_use]
pub fn parse_axis(text: &str) -> Option<(GamepadAxis, bool)> {
    let (invert, name) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let axis = match name {
        "LeftStickX" => GamepadAxis::LeftStickX,
        "LeftStickY" => GamepadAxis::LeftStickY,
        "LeftZ" => GamepadAxis::LeftZ,
        "RightStickX" => GamepadAxis::RightStickX,
        "RightStickY" => GamepadAxis::RightStickY,
        "RightZ" => GamepadAxis::RightZ,
        other => GamepadAxis::Other(other.strip_prefix("Axis")?.parse().ok()?),
    };
    Some((axis, invert))
}

/// The settings text of an axis, with the `-` of an inverted binding.
#[must_use]
pub fn axis_text(axis: GamepadAxis, invert: bool) -> String {
    let name = match axis {
        GamepadAxis::LeftStickX => "LeftStickX".to_owned(),
        GamepadAxis::LeftStickY => "LeftStickY".to_owned(),
        GamepadAxis::LeftZ => "LeftZ".to_owned(),
        GamepadAxis::RightStickX => "RightStickX".to_owned(),
        GamepadAxis::RightStickY => "RightStickY".to_owned(),
        GamepadAxis::RightZ => "RightZ".to_owned(),
        GamepadAxis::Other(n) => format!("Axis{n}"),
    };
    if invert {
        format!("-{name}")
    } else {
        name
    }
}

/// The button a settings text names.
#[must_use]
pub fn parse_button(text: &str) -> Option<GamepadButton> {
    Some(match text {
        "South" => GamepadButton::South,
        "East" => GamepadButton::East,
        "North" => GamepadButton::North,
        "West" => GamepadButton::West,
        "C" => GamepadButton::C,
        "Z" => GamepadButton::Z,
        "LeftTrigger" => GamepadButton::LeftTrigger,
        "LeftTrigger2" => GamepadButton::LeftTrigger2,
        "RightTrigger" => GamepadButton::RightTrigger,
        "RightTrigger2" => GamepadButton::RightTrigger2,
        "Select" => GamepadButton::Select,
        "Start" => GamepadButton::Start,
        "Mode" => GamepadButton::Mode,
        "LeftThumb" => GamepadButton::LeftThumb,
        "RightThumb" => GamepadButton::RightThumb,
        "DPadUp" => GamepadButton::DPadUp,
        "DPadDown" => GamepadButton::DPadDown,
        "DPadLeft" => GamepadButton::DPadLeft,
        "DPadRight" => GamepadButton::DPadRight,
        other => GamepadButton::Other(other.strip_prefix("Button")?.parse().ok()?),
    })
}

/// The settings text of a button.
#[must_use]
pub fn button_text(button: GamepadButton) -> String {
    let name = match button {
        GamepadButton::South => "South",
        GamepadButton::East => "East",
        GamepadButton::North => "North",
        GamepadButton::West => "West",
        GamepadButton::C => "C",
        GamepadButton::Z => "Z",
        GamepadButton::LeftTrigger => "LeftTrigger",
        GamepadButton::LeftTrigger2 => "LeftTrigger2",
        GamepadButton::RightTrigger => "RightTrigger",
        GamepadButton::RightTrigger2 => "RightTrigger2",
        GamepadButton::Select => "Select",
        GamepadButton::Start => "Start",
        GamepadButton::Mode => "Mode",
        GamepadButton::LeftThumb => "LeftThumb",
        GamepadButton::RightThumb => "RightThumb",
        GamepadButton::DPadUp => "DPadUp",
        GamepadButton::DPadDown => "DPadDown",
        GamepadButton::DPadLeft => "DPadLeft",
        GamepadButton::DPadRight => "DPadRight",
        GamepadButton::Other(n) => return format!("Button{n}"),
    };
    name.to_owned()
}
