//! Material Design 3 Radio button widget.

mod builder;
mod child;
mod spawn;
mod systems;

pub use systems::{radio_select_system, radio_sync_system};

use bevy::prelude::*;

use crate::interaction::events::RadioSelected;

/// Marker component for an M3 Radio root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Radio {
    pub is_selected: bool,
    pub group_id: Option<u32>,
}

/// Marker component linking an internal radio outer circle to its root entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioOuterRing(pub Entity);

/// Marker component linking an internal radio dot to its root entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioInnerDot(pub Entity);

/// Builder for constructing Material Design 3 radio buttons.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_radio(mut commands: Commands) {
///     RadioBuilder::new("Express Line")
///         .with_group(1)
///         .selected(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct RadioBuilder {
    pub label: Option<String>,
    pub is_selected: bool,
    pub group_id: Option<u32>,
    pub disabled: bool,
}

/// Plugin registering Radio widget systems and events.
#[derive(Default)]
pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<RadioSelected>>() {
            app.add_message::<RadioSelected>();
        }
        app.add_systems(Update, (radio_select_system, radio_sync_system));
    }
}

#[cfg(test)]
mod tests;
