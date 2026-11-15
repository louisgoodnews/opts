//! Keeps the water and the tile streaming in step with their settings.

use bevy::prelude::*;

use crate::in_game::tile_loading;
use crate::settings::GameSettings;

/// Whether tiles built from now on draw their water (`graphics.water`).
pub fn sync_water(
    settings: Res<GameSettings>,
    job: Option<ResMut<crate::in_game::omsi_world::WorldLoadJob>>,
) {
    let Some(mut job) = job else { return };
    let want = settings.raw("graphics.water").as_deref() != Some("false");
    if job.water_enabled != want {
        job.water_enabled = want;
    }
}

/// Applies `gameplay.load_all_tiles` and `gameplay.tile_radius` to the streamer (M-0080): the
/// mode follows the settings and, for *automatic*, the hardware profile; the streamer loads what
/// the new mode wants and, in radius mode, unloads what is too far.
pub fn sync_tile_loading(
    settings: Res<GameSettings>,
    hardware: Option<Res<crate::hardware::HardwareState>>,
    job: Option<ResMut<crate::in_game::omsi_world::WorldLoadJob>>,
) {
    let Some(mut job) = job else { return };
    let hardware_changed = hardware.as_ref().is_some_and(|h| h.is_changed());
    if !(settings.is_changed() || hardware_changed || job.is_added()) {
        return;
    }
    let profile = hardware.as_ref().and_then(|h| h.profile.as_ref());
    let mode = tile_loading::resolve(
        tile_loading::TileChoice::from_settings(&settings),
        profile,
        job.map_radius(),
    );
    if job.tile_mode() != mode {
        job.set_tile_mode(mode);
    }
    let budget = tile_loading::stream_budget_of(&settings);
    if job.stream_budget != budget {
        job.stream_budget = budget;
    }
}
