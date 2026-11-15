//! Material Design 3 Checkbox widget.

mod builder;
mod child;
mod spawn;
mod systems;

pub use systems::{checkbox_sync_system, checkbox_toggle_system};

use bevy::prelude::*;

use crate::interaction::{events::CheckboxToggled, state::CheckState};

/// Marker component for an M3 Checkbox root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Checkbox {
    pub state: CheckState,
}

/// Marker component for the internal indicator box of a checkbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxIndicator(pub Entity);

/// Marker component for the glyph text inside the indicator.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxGlyph(pub Entity);

/// Builder for constructing Material Design 3 checkboxes with optional labels.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_checkbox(mut commands: Commands) {
///     CheckboxBuilder::new("Enable timetable sync")
///         .checked(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CheckboxBuilder {
    pub label: Option<String>,
    pub state: CheckState,
    pub disabled: bool,
}

impl Default for CheckboxBuilder {
    fn default() -> Self {
        Self {
            label: None,
            state: CheckState::Unchecked,
            disabled: false,
        }
    }
}

/// Plugin registering Checkbox widget systems and events.
#[derive(Default)]
pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<CheckboxToggled>>() {
            app.add_message::<CheckboxToggled>();
        }
        app.add_systems(Update, (checkbox_toggle_system, checkbox_sync_system));
    }
}

#[cfg(test)]
mod tests;
