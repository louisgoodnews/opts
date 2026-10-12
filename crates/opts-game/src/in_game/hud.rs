//! The HUD entry points of the drive view. The HUD itself (widgets, profiles, slots, alerts) is the
//! [`crate::hud`] module; this file only starts it and handles the pause button.

use bevy::prelude::*;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::in_game::types::{InGameSessionInfo, PauseMenuBtn};

/// Builds the HUD of the player's vehicle (its profile is chosen from the vehicle kind).
pub fn spawn_in_game_hud(
    commands: &mut Commands,
    theme: &MaterialTheme,
    _session: &InGameSessionInfo,
) {
    crate::hud::build::queue_spawn(commands, theme);
}

/// Handles interactions with HUD buttons (the pause button opens the pause overlay).
pub fn handle_hud_interactions(
    mut events: MessageReader<ButtonClicked>,
    pause_query: Query<&PauseMenuBtn>,
    mut pause: MessageWriter<crate::pause::PauseRequest>,
) {
    for event in events.read() {
        if pause_query.contains(event.0) {
            tracing::info!("HUD pause button: opening the pause overlay");
            pause.write(crate::pause::PauseRequest);
        }
    }
}
