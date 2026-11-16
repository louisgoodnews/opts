//! Material Design 3 Button widget with variant styling and click actions.

mod builder;
mod spawn;
mod style;
mod systems;

pub use builder::ButtonBuilder;
pub use systems::button_click_system;

use bevy::prelude::*;
use std::sync::Arc;

use crate::interaction::{
    activation::Activated, events::ButtonClicked, focus::Focusable, layer::StateLayered,
    state::Hovered,
};

/// Marker for an interactive control: every widget root carries it. It requires the pointer
/// state Bevy's picking keeps current ([`Hovered`]) and `Node`; the state layer, focus ring and
/// disabled treatment of [`crate::interaction::layer`] attach to it, and pointer release inside
/// the control sends [`Activated`].
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[require(Node, bevy::ui::FocusPolicy::Block, Hovered, Focusable, StateLayered)]
pub struct Button;

/// Button variants supported by Material Design 3.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Filled,
    Elevated,
    FilledTonal,
    Outlined,
    Text,
    Icon,
}

/// Type alias for button click action callback closures.
pub type ButtonClickHandler = Arc<dyn Fn(&mut Commands, Entity) + Send + Sync>;

/// Optional user callback executed when an M3 button is clicked.
#[derive(Component, Clone)]
pub struct OnClickCallback(pub ButtonClickHandler);

/// Marker component for a Material Design 3 button root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Button {
    pub variant: ButtonVariant,
}

/// Plugin registering Button widget systems.
#[derive(Default)]
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ButtonClicked>()
            .add_message::<Activated>();
        app.add_systems(Update, button_click_system);
    }
}

#[cfg(test)]
mod tests;
