//! Spawning, walking and configuring pedestrians around the camera: the spawner is in [`spawn`],
//! the walkers in [`walker`]; the session's configuration stays here.

use super::*;

mod spawn;
mod walker;

pub use spawn::passenger_factor;
pub use spawn::spawn_pedestrians;
pub use walker::move_pedestrians;

/// Reads `--pedestrians` and resets the per-session state when a session starts.
pub fn configure_pedestrians(
    args: Option<Res<crate::AppArgs>>,
    user: Option<Res<crate::settings::GameSettings>>,
    mut settings: ResMut<PedestrianSettings>,
    mut net: ResMut<PedestrianNetwork>,
    mut pool: ResMut<PedestrianPool>,
    mut ground: ResMut<super::super::GroundIndex>,
) {
    *ground = super::super::GroundIndex::default();
    // The command line wins for this run; then the setting *Pedestrians*; then the default.
    settings.baseline = args
        .and_then(|a| a.0.pedestrians)
        .or_else(|| user.and_then(|u| u.raw("gameplay.pedestrians")?.parse().ok()))
        .unwrap_or(DEFAULT_PEDESTRIAN_BASELINE);
    net.0 = PedestrianNet::default();
    *pool = PedestrianPool::default();
}
