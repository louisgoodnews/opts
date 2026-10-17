//! Application shutdown hooks and state persistence.

use crate::settings::GameSettings;
use bevy::prelude::*;

/// Plugin that captures [`AppExit`] events to ensure persistent configuration is saved.
#[derive(Default)]
pub struct ShutdownPlugin;

impl Plugin for ShutdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, handle_application_shutdown);
    }
}

/// System that intercepts [`AppExit`] and writes settings to disk before window destruction.
fn handle_application_shutdown(
    mut exit_events: MessageReader<AppExit>,
    settings: Option<Res<GameSettings>>,
) {
    if exit_events.read().next().is_some() {
        tracing::info!("Received AppExit event; performing clean shutdown tasks");
        if let Some(settings) = settings {
            GameSettings::flush_background_saves();
            if let Err(err) = settings.save() {
                tracing::warn!(err = ?err, "Failed to persist GameSettings during shutdown");
            } else {
                tracing::info!("GameSettings successfully persisted to disk");
            }
        }
    }
}
