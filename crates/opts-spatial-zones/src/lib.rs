//! Spatial zones: the game knows when the player vehicle or the camera is inside a tunnel
//! (M-0115; `docs/src/architecture/lighting-system.md`, section 13).
//!
//! * [`ceiling`]: the covered-section rule (spline ceiling profiles), a pure function.
//! * [`model`]: the zone component, its kinds and the [`ZoneEntered`] / [`ZoneLeft`] messages.
//! * [`tracker`]: the hysteresis state machine that turns per-sample coverage into transitions,
//!   and the 10 Hz sampling gate. Pure logic, no world access.
//! * [`registry`]: the zone entities per tile, built at tile load and dropped at unload.
//! * [`query`]: the JSON payload of the `zones.query` control method.
//! * [`overlay`]: the gizmo boxes of the resident zones and the membership markers (debug).
//! * [`sampler`]: the 10 Hz system that tests each [`sampler::ZoneSubject`] against the zones and
//!   emits the messages.
//!
//! The detector is the spline ceiling rule in [`ceiling`]. The game's tile loader feeds it the placed
//! splines of each resident tile (ISS-012). The thresholds are the constants in [`ceiling`].

use bevy::prelude::*;

pub mod ceiling;
pub mod model;
pub mod overlay;
pub mod query;
pub mod registry;
pub mod sampler;

/// The Bevy-free tracker and sample gate, re-exported from `opts-spatial-zones-core` (ISS-092).
pub use opts_spatial_zones_core::tracker;

pub use model::{SpatialZone, ZoneEntered, ZoneId, ZoneKind, ZoneLeft};
pub use overlay::ZoneOverlay;
pub use registry::{TileKey, ZoneRegistry};
pub use sampler::{ZoneMembership, ZoneSubject};
pub use tracker::{SampleGate, Transition, ZoneTracker};

/// Registers the zone messages, the tile registry and the 10 Hz membership sampler.
pub struct SpatialZonesPlugin;

impl Plugin for SpatialZonesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ZoneEntered>()
            .add_message::<ZoneLeft>()
            .init_resource::<ZoneRegistry>()
            .init_resource::<ZoneOverlay>()
            .add_systems(Update, sampler::sample_zone_subjects);
    }
}

#[cfg(test)]
mod tests;
