//! The descriptor parts of a widget: how its value is written, its settings switch and the
//! condition under which it is visible.

use opts_bevy::CameraMode;

use super::VehicleKind;

/// How a bound value is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A number with `precision` decimals, tabular, in the unit chosen in the settings.
    Number {
        /// Digits after the decimal point.
        precision: u8,
    },
    /// A short word or text.
    Text,
    /// A lamp: a word in the state colour (`ok`, `alarm`).
    Lamp,
    /// A drawing (the mini map).
    Graphic,
    /// A horizontal bar gauge of a value in percent (0 to 100).
    Bar,
}

/// The switch that turns a widget on and off in *Settings → Interface*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetSwitch {
    /// Settings key (`hud.speed`).
    pub key: &'static str,
    /// Message key of the label.
    pub label_key: &'static str,
    /// Message key of the description.
    pub description_key: &'static str,
    /// Value when the user never chose.
    pub default_on: bool,
}

/// When a widget is visible besides its switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condition {
    /// Vehicle kinds it applies to; empty means all.
    pub kinds: &'static [VehicleKind],
    /// Only when the simulation provides an engine speed.
    pub needs_rpm: bool,
    /// Camera modes in which it is hidden (hidden, not removed: nothing moves).
    pub hidden_in: &'static [CameraMode],
}

impl Condition {
    /// No condition.
    pub const ALWAYS: Self = Self {
        kinds: &[],
        needs_rpm: false,
        hidden_in: &[],
    };
}
