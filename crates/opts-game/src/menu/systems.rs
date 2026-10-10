//! Top-level lifecycle systems for the main menu screen.

use bevy::app::AppExit;
use bevy::prelude::*;
use opts_gui::interaction::focus::Focusable;
use opts_gui::interaction::state::Focused;
use opts_gui::theme::theme::MaterialTheme;

use crate::menu::layout::{cleanup_main_menu, spawn_main_menu};
use crate::settings::GameSettings;

/// Setup system triggered on entering [`crate::app_state::AppState::MainMenu`].
pub fn enter_main_menu_system(
    mut commands: Commands,
    settings: Res<GameSettings>,
    mut images: ResMut<Assets<Image>>,
    theme: Res<MaterialTheme>,
) {
    tracing::info!("Initializing MainMenu UI");
    spawn_main_menu(
        &mut commands,
        settings.recent_map_path.as_deref(),
        &mut images,
        &theme,
    );
}

/// Cleanup system triggered on exiting [`crate::app_state::AppState::MainMenu`].
pub fn exit_main_menu_system(
    commands: Commands,
    query: Query<Entity, With<crate::menu::layout::MainMenuRoot>>,
) {
    cleanup_main_menu(commands, query);
}

/// System that ensures the "Play" button (tab_index: 0) acquires initial focus on entry.
pub fn initial_focus_system(
    mut commands: Commands,
    query: Query<(Entity, &Focusable), Added<Focusable>>,
) {
    for (entity, focusable) in &query {
        if focusable.tab_index != 0 {
            continue;
        }
        commands.entity(entity).insert(Focused);
        break;
    }
}

/// System that handles pressing `Escape` in [`crate::app_state::AppState::MainMenu`] to exit the application.
pub fn menu_escape_exit_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit_events: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        tracing::info!("Escape pressed on Main Menu; requesting clean exit");
        exit_events.write(AppExit::Success);
    }
}
