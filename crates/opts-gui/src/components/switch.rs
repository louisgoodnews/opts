//! Material Design 3 Switch toggle widget.

use bevy::prelude::*;

mod parts;
mod spawn;
mod systems;

pub use systems::{SwitchPlugin, switch_sync_system, switch_toggle_system};

/// Marker component for an M3 Switch root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Switch {
    pub is_on: bool,
}

/// Marker component linking an internal switch track to its root switch entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchTrack(pub Entity);

/// Marker component linking an internal switch thumb to its root switch entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchThumb(pub Entity);

/// Builder for constructing Material Design 3 switch toggles.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_switch(mut commands: Commands) {
///     SwitchBuilder::new("Auto-dispatch")
///         .with_value(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SwitchBuilder {
    pub label: Option<String>,
    pub is_on: bool,
    pub disabled: bool,
}

impl SwitchBuilder {
    /// Creates a new switch builder with an associated label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            is_on: false,
            disabled: false,
        }
    }

    /// Creates a standalone switch builder without a label.
    pub fn standalone() -> Self {
        Self::default()
    }

    /// Sets the initial switch value.
    pub fn with_value(mut self, is_on: bool) -> Self {
        self.is_on = is_on;
        self
    }

    /// Sets whether the switch is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[cfg(test)]
mod tests;
