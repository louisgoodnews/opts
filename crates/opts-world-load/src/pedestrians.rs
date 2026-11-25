//! Pedestrians walking on the footpaths of the loaded tiles.
//!
//! Lanes come from the type-1 `[path]` records of spline profiles and scenery objects
//! ([`opts_sim::pedestrian`]); characters are the `.hum` files of the map's `humans.txt`, built
//! with the same rig as the player ([`opts_bevy::spawn_human`]). See
//! `docs/src/simulation/pedestrians.md`.

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::TileTerrain;
use opts_bevy::{pose_rotations, rest_pivots, spawn_human, Human, HumanPose, ModelPart};
use opts_core::formats::humans::HumanProfile;
use opts_core::formats::map::{MapObjectInstance, MapTile};
use opts_sim::pedestrian::{
    gait, lanes_from_object, lanes_from_spline, Advance, FootLane, PedestrianNet, Walker,
};

use crate::in_game::types::InGameEntity;

use super::vehicle::load_human_parts;
use super::WorldLoadJob;

/// Default baseline of pedestrians around the player (setting `gameplay.pedestrians`): how many
/// there are when the map's passenger factor (`[trafficdensity_passenger]`) is 1. The footpaths
/// hold at most one per [`MIN_PATH_SPACING_M`].
pub const DEFAULT_PEDESTRIAN_BASELINE: usize = 120;
/// Closest spacing (m of footpath in the loaded tiles) the paths are filled to.
pub(super) const MIN_PATH_SPACING_M: f32 = 8.0;
/// Pedestrians appear at least this far from the camera (m)…
pub(super) const SPAWN_MIN_M: f32 = 25.0;
/// …and at most this far.
pub(super) const SPAWN_MAX_M: f32 = 140.0;
/// Pedestrians farther than this are removed (m).
pub(super) const DESPAWN_M: f32 = 175.0;
/// Seconds between spawn attempts.
pub(super) const SPAWN_INTERVAL_S: f32 = 0.3;

/// The footpath network of the resident tiles.
#[derive(Resource, Default)]
pub struct PedestrianNetwork(pub PedestrianNet);

/// Pedestrian settings.
#[derive(Resource, Clone, Copy)]
pub struct PedestrianSettings {
    /// Baseline of living pedestrians (`0` disables them).
    pub baseline: usize,
}

impl Default for PedestrianSettings {
    fn default() -> Self {
        Self {
            baseline: DEFAULT_PEDESTRIAN_BASELINE,
        }
    }
}

/// A loaded character model.
pub(super) struct Variant {
    pub(super) name: String,
    pub(super) profile: HumanProfile,
    pub(super) parts: Vec<ModelPart>,
}

/// Character models of the map's `humans.txt`, loaded on demand.
#[derive(Resource, Default)]
pub struct PedestrianPool {
    /// Entries of `humans.txt` (duplicates weight the draw).
    pub(super) entries: Vec<String>,
    /// Distinct entries and their loaded model, load attempts so far.
    pub(super) slots: Vec<(String, Option<Variant>, u32)>,
    pub(super) started: bool,
    /// The passenger system needs the characters even when the walkers are off.
    pub(super) wanted: bool,
    pub(super) rng: u32,
    pub(super) frames: u32,
}

/// Component of a living pedestrian.
#[derive(Component)]
pub struct Pedestrian {
    pub(super) walker: Walker,
    pub(super) leg_m: f32,
}

mod lanes;
pub(super) use lanes::{register_tile, tile_lanes, unregister_tile};
mod load;
pub use load::load_variants;

mod systems;
pub use systems::passenger_factor;
pub use systems::{configure_pedestrians, move_pedestrians, spawn_pedestrians};

mod actor;
pub use actor::{advance_phase, apply_gait, apply_rest_pose, apply_seated_pose};

#[cfg(test)]
mod tests;
