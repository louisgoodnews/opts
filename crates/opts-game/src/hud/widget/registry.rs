//! Built-in [`HudWidget`] registry table (`WIDGETS`) and lookup helpers.

use super::*;

mod entries_a;
mod entries_b;
use entries_a::*;
use entries_b::*;

/// Every built-in widget.
pub const WIDGETS: &[HudWidget] = &[
    SPEED,
    GEAR,
    RPM,
    PARKING_BRAKE,
    RAIL_DIRECTION,
    RAIL_LEVER,
    SESSION,
    CLOCK,
    CONTROLS_HINT,
    ALERTS,
    CAMERA_LABEL,
    MINIMAP,
];

/// The widget with `id`.
#[must_use]
pub fn widget(id: &str) -> Option<&'static HudWidget> {
    WIDGETS.iter().find(|w| w.id == id)
}

/// The stored value of a widget switch, from the raw setting text.
#[must_use]
pub fn switch_on(switch: &WidgetSwitch, raw: Option<&str>) -> bool {
    match raw {
        Some("true") => true,
        Some("false") => false,
        _ => switch.default_on,
    }
}
