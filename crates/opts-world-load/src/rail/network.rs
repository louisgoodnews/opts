//! Track and cycle-path tile registration, configuration and unit pose interpolation on rails.

use super::*;

/// `[path]` type of rail.
const PATH_TYPE_RAIL: u32 = 2;

/// Smallest half span used for the chord of a unit without bogie data (m).
const MIN_HALF_SPAN: f32 = 1.0;

mod lanes;
pub(in crate) use lanes::{
    register_bike_tile, register_tile, tile_bike_lanes, tile_rail_lanes, unregister_bike_tile,
    unregister_tile,
};
mod pose;
pub use pose::{rail_player_pose, unit_frame, unit_pose};

/// Resets the per-session state and reads `--trains`.
pub fn configure_rail(
    args: Option<Res<crate::AppArgs>>,
    mut settings: ResMut<RailSettings>,
    mut net: ResMut<RailNetwork>,
    mut pool: ResMut<RailAiPool>,
    mut bike_settings: ResMut<BikeSettings>,
    mut bike_net: ResMut<BikeNetwork>,
    mut bike_pool: ResMut<BikeAiPool>,
) {
    bike_settings.max_riders = args
        .as_ref()
        .and_then(|a| a.0.cyclists)
        .unwrap_or(BikeSettings::default().max_riders);
    bike_net.0 = PedestrianNet::default();
    *bike_pool = BikeAiPool::default();
    settings.max_trains = args.and_then(|a| a.0.trains).unwrap_or(DEFAULT_MAX_TRAINS);
    net.0 = PedestrianNet::default();
    *pool = RailAiPool::default();
}
