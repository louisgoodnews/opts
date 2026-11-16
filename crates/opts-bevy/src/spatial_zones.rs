//! Spatial zones: the game knows when the player vehicle or the camera is inside a tunnel
//! (M-0115; `docs/src/architecture/lighting-system.md`, section 13).
//!
//! * [`model`]: the zone component, its kinds and the [`ZoneEntered`] / [`ZoneLeft`] messages.
//! * [`tracker`]: the hysteresis state machine that turns per-sample coverage into transitions,
//!   and the 10 Hz sampling gate. Pure logic, no world access.
//!
//! The detector that feeds the tracker (roof ray casts, tile data) is not wired yet; see the
//! milestone page for what is open.

use bevy::prelude::*;

pub mod model;
pub mod tracker;

pub use model::{SpatialZone, ZoneEntered, ZoneId, ZoneKind, ZoneLeft};
pub use tracker::{SampleGate, Transition, ZoneTracker};

/// Registers the zone messages. Systems that emit them are added by the detector.
pub struct SpatialZonesPlugin;

impl Plugin for SpatialZonesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ZoneEntered>().add_message::<ZoneLeft>();
    }
}

#[cfg(test)]
mod tests;
