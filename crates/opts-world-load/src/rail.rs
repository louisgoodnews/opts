//! Rail-bound vehicles in the game: the track network of the loaded tiles, the player's tram and
//! AI trains. The physics live in [`opts_sim::rail`]; see `docs/src/simulation/rail-vehicles.md`.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::TileTerrain;
use opts_bevy::VehicleController;
use opts_core::formats::map::{MapObjectInstance, MapTile};
use opts_core::formats::scenery::LaneKind;
use opts_core::formats::vehicles::model::{ModelConfig, ModelLod};
use opts_core::formats::vehicles::BusManifest;
use opts_sim::pedestrian::{
    lanes_from_object_of_type, lanes_of_kind, FootLane, PedestrianNet, Travel,
};
use opts_sim::rail::{Branch, Moved, Train};

use opts_game_state::in_game::types::InGameEntity;

use super::vehicle::load_segment_meshes;
use super::WorldLoadJob;

/// Below this speed (m/s) a rail vehicle may change its direction of travel.
const STANDSTILL_M_S: f32 = 0.2;
/// How far ahead (m) the key that toggles the points looks for them.
const SWITCH_REACH_M: f32 = 150.0;
/// A train formation is put on the nearest track within this distance of its entry point (m): the
/// entry points of a map are road-oriented, a station track can lie some way off.
pub const FORMATION_SNAP_M: f32 = 150.0;
/// Default number of AI trains.
pub const DEFAULT_MAX_TRAINS: usize = 3;
/// Cruise speed of cyclists (m/s).
const BIKE_CRUISE_MS: f32 = 4.5;
/// Cruise speed of AI trains (m/s).
const AI_CRUISE_MS: f32 = 9.0;
/// AI trains appear this far from the camera (m)…
const AI_SPAWN_MIN_M: f32 = 80.0;
/// …up to this far…
const AI_SPAWN_MAX_M: f32 = 400.0;
/// …and are removed beyond this (m).
const AI_DESPAWN_M: f32 = 600.0;

/// The track network of the resident tiles.
#[derive(Resource, Default)]
pub struct RailNetwork(pub PedestrianNet);

/// The cycle-path network of the resident tiles (`LaneKind::Bike` lanes of splines).
#[derive(Resource, Default)]
pub struct BikeNetwork(pub PedestrianNet);

/// Cap on riders (`--cyclists N`).
#[derive(Resource, Clone, Copy)]
pub struct BikeSettings {
    /// Cap on simultaneously riding cyclists.
    pub max_riders: usize,
}

impl Default for BikeSettings {
    fn default() -> Self {
        Self { max_riders: 6 }
    }
}

/// Number of AI trains (`--trains N`).
#[derive(Resource, Clone, Copy)]
pub struct RailSettings {
    /// Cap on AI trains.
    pub max_trains: usize,
}

impl Default for RailSettings {
    fn default() -> Self {
        Self {
            max_trains: DEFAULT_MAX_TRAINS,
        }
    }
}

// --- network ----------------------------------------------------------------------------------

mod network;
pub use network::{configure_rail, rail_player_pose};
pub(super) use network::{
    register_bike_tile, register_tile, tile_bike_lanes, tile_rail_lanes, unit_frame, unit_pose,
    unregister_bike_tile, unregister_tile,
};

// --- player ------------------------------------------------------------------------------------

mod player;
pub use player::{is_rail, RailPlayer};
mod player_step;
pub use player_step::rail_player_step;
mod player_track;

mod composition;
pub use composition::{Part, VehicleComposition};
mod body;
use body::RailBody;
mod points;
mod points_click;
pub use points::{points_visual_system, set_points_system};
pub use points_click::click_points_system;
mod switch;
pub use switch::{points_ahead, switch_hint_system, way_by_id, way_id};

// --- AI trains ---------------------------------------------------------------------------------

mod ai;
mod scheduled;
pub use scheduled::{spawn_scheduled_trains, step_rail_scripts, ScheduledTrain};
mod obstacles;
mod spec;
pub use ai::{
    load_rail_variants, move_ai_trains, spawn_ai_trains, AiTrain, BikeAiPool, RailAiPool,
};
