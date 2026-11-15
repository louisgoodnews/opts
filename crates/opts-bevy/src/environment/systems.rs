//! The systems that advance the environment and apply its changes.

use super::*;

/// Moves the clock with `Time<Virtual>` × rate (stands while the game is paused).
pub fn tick_environment(time: Res<Time<Virtual>>, mut env: ResMut<SessionEnvironment>) {
    let dt = time.delta_secs_f64() * env.rate();
    if dt > 0.0 {
        env.advance(dt);
    }
}

/// Applies the [`EnvironmentChange`] messages in order and announces map date changes.
pub fn apply_environment_changes(
    mut changes: MessageReader<EnvironmentChange>,
    mut env: ResMut<SessionEnvironment>,
    catalogue: Res<WeatherCatalogue>,
    mut map_dates: MessageWriter<MapDateChanged>,
) {
    for change in changes.read() {
        if let Some(changed) = env.apply_with(change, &catalogue) {
            tracing::info!(old = %changed.old, new = %changed.new, "map date changed");
            map_dates.write(changed);
        }
    }
}

/// Resolves the weather again when the preset catalogue changes (an install found after start).
pub fn refresh_weather_on_catalogue_change(
    catalogue: Res<WeatherCatalogue>,
    mut env: ResMut<SessionEnvironment>,
) {
    if catalogue.is_changed() && !catalogue.is_added() {
        env.refresh_weather(&catalogue);
    }
}
