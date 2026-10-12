//! Systems of the settings screen: leaving it.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::settings_screen::types::{BackToMenuButton, SettingsScreenRoot};

/// System returning to the main menu when the back button is clicked.
pub fn handle_back_to_menu(
    mut events: MessageReader<ButtonClicked>,
    back_query: Query<&BackToMenuButton>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        if back_query.contains(event.0) {
            tracing::info!("Returning to Main Menu from Settings");
            next_state.set(AppState::MainMenu);
        }
    }
}

/// System that despawns all settings screen entities on exit.
pub fn cleanup_settings_screen(
    mut commands: Commands,
    query: Query<Entity, With<SettingsScreenRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Escape key listener returning to the Main Menu (unless a modal or the text field has it).
pub fn escape_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}
