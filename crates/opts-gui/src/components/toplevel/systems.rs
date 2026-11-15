//! Close and plugin systems of the toplevel widget.

use bevy::prelude::*;

use crate::interaction::{activation::Activated, events::ToplevelClosed, state::Disabled};

use super::ToplevelCloseButton;
/// System handling clicks on the toplevel close button and despawning the window.
pub fn toplevel_close_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    close_query: Query<&ToplevelCloseButton, Without<Disabled>>,
    mut events: MessageWriter<ToplevelClosed>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(btn) = close_query.get(*entity) {
            events.write(ToplevelClosed { entity: btn.0 });
            commands.entity(btn.0).despawn();
        }
    }
}

/// Plugin registering Toplevel widget systems and events.
#[derive(Default)]
pub struct ToplevelPlugin;

impl Plugin for ToplevelPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<ToplevelClosed>>() {
            app.add_message::<ToplevelClosed>();
        }
        app.add_systems(Update, toplevel_close_system);
    }
}
