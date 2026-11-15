//! Settings of the AI road traffic (the baseline density).

use bevy::prelude::*;

use super::DEFAULT_TRAFFIC_BASELINE;

/// Traffic settings.
#[derive(Resource, Clone, Copy)]
pub struct TrafficSettings {
    /// Baseline of driving AI vehicles around the player (`0` disables traffic).
    pub baseline: usize,
}

impl Default for TrafficSettings {
    fn default() -> Self {
        Self {
            baseline: DEFAULT_TRAFFIC_BASELINE,
        }
    }
}
