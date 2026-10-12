//! Systems handling interactions and persistence in the settings screen.

use bevy::prelude::*;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::catalogue_service::{spawn_discovery, CatalogueService};
use crate::settings::GameSettings;
use crate::settings_screen::types::{
    BackToMenuButton, DiscoveryIntervalButton, RunDiscoveryNowButton, SettingsScreenRoot,
};

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

/// System handling discovery interval frequency selection and saving to SQLite.
pub fn handle_discovery_interval_clicks(
    mut events: MessageReader<ButtonClicked>,
    interval_query: Query<&DiscoveryIntervalButton>,
    mut card_query: Query<(
        &DiscoveryIntervalButton,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
    mut settings: ResMut<GameSettings>,
    theme: Res<MaterialTheme>,
) {
    let mut selected = None;
    for event in events.read() {
        if let Ok(btn) = interval_query.get(event.0) {
            selected = Some(btn.interval_days);
        }
    }

    if let Some(days) = selected {
        settings.discovery_interval_days = days;
        tracing::info!(days = days, "Updated vehicle discovery interval");

        for (btn, mut border, mut bg) in &mut card_query {
            let is_active = btn.interval_days == days;
            *border = BorderColor::all(if is_active {
                theme.scheme.get(ColorRole::Primary)
            } else {
                theme.scheme.get(ColorRole::OutlineVariant)
            });
            *bg = BackgroundColor(if is_active {
                theme.scheme.get(ColorRole::PrimaryContainer)
            } else {
                theme.scheme.get(ColorRole::SurfaceContainerHigh)
            });
        }

        if let Err(err) = settings.save() {
            tracing::warn!(err = ?err, "Failed to persist discovery interval to SQLite");
        }
    }
}

/// System handling manual "Run Discovery Now" button click.
pub fn handle_run_discovery_clicks(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    trigger_query: Query<&RunDiscoveryNowButton>,
    service: Option<Res<CatalogueService>>,
) {
    for event in events.read() {
        if trigger_query.contains(event.0) {
            tracing::info!("Starting immediate catalogue discovery from Settings menu");
            if let Some(service) = &service {
                spawn_discovery(service);
                ToastBuilder::new("OMSI catalogue discovery started in the background.")
                    .with_duration(3.0)
                    .spawn(&mut commands);
            } else {
                ToastBuilder::new("No OMSI installation found; cannot run discovery.")
                    .with_duration(4.0)
                    .spawn(&mut commands);
            }
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

/// Escape key listener returning to the Main Menu.
pub fn escape_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}
