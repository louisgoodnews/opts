//! The crowds at the stops: how many people there are, how long they gather, how many are drawn.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_sim::passengers::Destination;

/// Most people drawn waiting at a stop; the rest of the crowd boards without a figure.
pub(super) const MAX_VISIBLE_WAITING: u32 = 24;
/// Most figures drawn inside one vehicle chain; people beyond it are counted but not drawn.
pub(super) const ACTOR_BUDGET: usize = 90;
/// Most people at a stop.
pub(super) const MAX_CROWD: u32 = 400;
/// Minutes of arrivals a stop has gathered when it is served for the first time.
pub(super) const FIRST_VISIT_WAIT_MIN: f64 = 8.0;
/// Most minutes of arrivals a stop gathers between two visits.
pub(super) const MAX_ACCUMULATE_MIN: f64 = 40.0;
/// Seconds between two people of the unseen crowd boarding through the same open door.
pub(super) const UNSEEN_BOARD_S: f32 = 0.35;
/// Passengers per hour at a stop when the setting is missing.
pub(super) const DEFAULT_RATE: f32 = 120.0;
/// Key of the setting *Passengers per hour*.
pub(super) const RATE_SETTING: &str = "gameplay.passenger_rate";

/// The people at the stops: when a stop was last served and who was left behind (a full vehicle
/// leaves some, each with the destination they hold). Between visits people gather at the rate of
/// the setting times the map's passenger factor for the hour.
#[derive(Resource, Default)]
pub struct StopCrowds(pub(super) HashMap<(i32, i32), Crowd>);

/// What a stop holds between two visits.
#[derive(Debug, Clone, Default)]
pub(super) struct Crowd {
    /// When the stop was last served (s).
    pub(super) since: f64,
    /// The destinations of those left behind, in order (`None`: no trip serves the stop).
    pub(super) left: Vec<Option<Destination>>,
}

/// A stop's key: its position to 2 m.
pub(super) fn stop_key(t: &Transform) -> (i32, i32) {
    (
        (t.translation.x / 2.0).round() as i32,
        (t.translation.z / 2.0).round() as i32,
    )
}
