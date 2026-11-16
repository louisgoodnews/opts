//! Tooltips: shown after 600 ms of hover or immediately on keyboard focus.
//!
//! The text comes from a [`Tooltip`] component or, for a disabled control, from its
//! [`DisabledReason`]; icon-only buttons always carry a [`Tooltip`]. At most one tooltip exists.

use crate::interaction::state::DisabledReason;
use bevy::prelude::*;
use std::borrow::Cow;

mod system;

pub use system::{tooltip_system, TooltipParams};

/// Hover time before a tooltip appears.
pub const TOOLTIP_DELAY_SECS: f32 = 0.6;
/// Widest tooltip in logical pixels.
pub const TOOLTIP_MAX_WIDTH: f32 = 320.0;

/// Tooltip text of a control (repeat the shortcut here).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Tooltip(pub Cow<'static, str>);

impl Tooltip {
    /// Creates a tooltip from any string.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self(text.into())
    }
}

/// Marker of the single tooltip node.
#[derive(Component, Debug, Clone, Copy)]
pub struct TooltipNode {
    /// The control the tooltip describes.
    pub target: Entity,
}

/// What the tooltip system is currently tracking.
#[derive(Resource, Debug, Default)]
pub struct TooltipState {
    /// Control the pointer rests on or that holds keyboard focus.
    pub target: Option<Entity>,
    /// Seconds the target has been hovered.
    pub hover_secs: f32,
    /// Whether the target was chosen by keyboard focus (no delay).
    pub by_focus: bool,
}

/// Text a control shows as tooltip, if any.
pub fn tooltip_text(
    tooltip: Option<&Tooltip>,
    reason: Option<&DisabledReason>,
    disabled: bool,
) -> Option<String> {
    match (disabled, reason, tooltip) {
        (true, Some(r), _) => Some(r.0.to_string()),
        (_, _, Some(t)) => Some(t.0.to_string()),
        _ => None,
    }
}

/// Registers the tooltip resource and system.
pub fn build(app: &mut App) {
    app.init_resource::<TooltipState>()
        .add_systems(Update, tooltip_system);
}

#[cfg(test)]
mod tests;
