//! The traffic road graph of the loaded tiles: the path sources the lanes are read from, the
//! registration of each tile's lanes and parked vehicles in [`register`], and the rebuild of the
//! lane graph in [`rebuild`].

use super::*;

mod rebuild;
mod register;
mod source;

pub use rebuild::update_traffic_graph;
pub(in super::super::super) use register::{
    register_parked_object, register_tile, reset_pool, unregister_tile,
};
pub(in super::super::super) use source::tile_segments;

/// Reads `--traffic` and resets the per-session state when a session starts.
pub fn configure_traffic(
    args: Option<Res<crate::AppArgs>>,
    user: Option<Res<crate::settings::GameSettings>>,
    mut settings: ResMut<TrafficSettings>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    mut sched: ResMut<ScheduledTraffic>,
    mut incidents: ResMut<super::IncidentQueue>,
) {
    // The command line wins for this run (`--traffic N` is the baseline); then the setting *Road traffic*; then the default.
    settings.baseline = args
        .and_then(|a| a.0.traffic)
        .or_else(|| user.and_then(|u| u.raw("gameplay.traffic")?.parse().ok()))
        .unwrap_or(DEFAULT_TRAFFIC_BASELINE);
    *state = TrafficState::default();
    *pool = TrafficPool::default();
    *sched = ScheduledTraffic::default();
    // Incident lanes belong to the graph that is replaced here.
    *incidents = super::IncidentQueue::default();
}
