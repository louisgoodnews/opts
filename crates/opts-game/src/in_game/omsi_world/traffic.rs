//! AI road traffic on the lanes of the loaded tiles (kinematic mode, M-0049 phase C).
//!
//! Lanes come from the type-0 `[path]` records of splines and scenery objects
//! ([`opts_traffic::segments_from_tile`]) and join geometrically; vehicles are drawn from the map's
//! `ailists.cfg` (every group that `unsched_vehgroups.txt` lets take part) and driven by the
//! Intelligent Driver Model with parameters derived from each vehicle's own files
//! ([`opts_traffic::IdmParams::from_traits`]). The vehicle models are placed on the lane (no
//! scripts or physics); coupled vehicles pull their trailing units behind them
//! (`traffic_chain.rs`). Plan and limits: `docs/src/architecture/ai-traffic.md`,
//! `docs/src/architecture/ai-vehicle-config.md`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::TileTerrain;
use opts_core::formats::map::lists::day;
use opts_core::formats::map::{DensityProfile, MapObjectInstance, MapTile};
use opts_core::formats::scenery::sco::{ScoPath, ScoTrafficLightGroup};
use opts_core::formats::scenery::{SceneryObject, SplinePath, SplineProfile};
use opts_core::formats::vehicles::model::ModelLod;
use opts_traffic::{
    group_targets, segments_from_tile, signals_from_tile, AgentId, Calibration, ConnectParams,
    IdmParams, LaneGraph, LaneKind, LaneSegment, PathSource, Rng, SignalKey, SignalProgram,
    TrafficWorld, VehicleClass,
};

use crate::in_game::types::InGameEntity;

use super::traffic_chain::{despawn_chain, ChainUnit, TrafficChain};
use super::traffic_variant::{
    load_variant, read_groups, PoolGroup, PoolSetup, Slot, SlotState, Unit,
};
use super::vehicle::load_segment_meshes_for;
use super::WorldLoadJob;

/// Default baseline of driving AI vehicles around the player (setting `gameplay.traffic`): how
/// many there are, within the despawn disc, when the map's traffic factor is 1. The map's factors
/// (`[trafficdensity_road]` curve, group curves, lane rules) shape it; the roads hold at most one
/// vehicle per [`MIN_LANE_SPACING_M`], and when the baseline exceeds that all groups are scaled
/// down in proportion.
pub const DEFAULT_TRAFFIC_BASELINE: usize = 150;
/// Closest spacing (m of density-weighted lane) the roads are filled to however high the baseline.
pub const MIN_LANE_SPACING_M: f64 = 25.0;
/// `L0`: one vehicle per this much lane (m) at density `1` and time factor `1` (the density model,
/// `docs/src/architecture/ai-traffic.md`). Calibrated with the `density_census` example: the
/// median, over 56 entry points of 12 maps at 12:00 on a weekday, of the lane metres × density ×
/// time factor within 320 m divided by the former target (84 m; Berlin-Spandau's main entry 60 m).
pub const METRES_OF_LANE_PER_VEHICLE: f64 = 80.0;
/// Vehicles appear at least this far from the camera (m)…
const SPAWN_MIN_M: f64 = 80.0;
/// …and at most this far.
const SPAWN_MAX_M: f64 = 220.0;
/// Vehicles farther than this are removed (m).
const DESPAWN_M: f32 = 320.0;
/// Seconds between spawn attempts.
const SPAWN_INTERVAL_S: f32 = 0.5;
/// Load attempts per vehicle type before it is given up (scrambled meshes decode in the background).
const MAX_LOAD_TRIES: u32 = 8;

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

/// The lane network of the resident tiles and the agents on it.
#[derive(Resource)]
pub struct TrafficState {
    world: TrafficWorld,
    /// Road lane segments per resident tile.
    segments: HashMap<(i32, i32), Vec<LaneSegment>>,
    /// Light programs of the junction objects per resident tile.
    signals: HashMap<(i32, i32), Vec<(SignalKey, SignalProgram)>>,
    /// The segments changed; the graph is rebuilt before the next step.
    dirty: bool,
    road_length_m: f64,
}

impl TrafficState {
    /// Vehicles on the lanes (scheduled ones included).
    #[must_use]
    pub(crate) fn agent_count(&self) -> usize {
        self.world.agents().len()
    }

    /// Whether the lanes of tile `key` are in the graph (the tile is resident).
    #[must_use]
    pub(crate) fn has_tile(&self, key: (i32, i32)) -> bool {
        self.segments.contains_key(&key)
    }

    /// Sets the vehicles outside the traffic that the agents must stop for or drive round (the
    /// player's bus), as footprints in the session frame.
    pub(super) fn set_obstacles(&mut self, obstacles: Vec<opts_traffic::Obstacle>) {
        self.world.set_obstacles(obstacles);
    }
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            world: TrafficWorld::new(LaneGraph::default(), 0x7AF1_C001),
            segments: HashMap::new(),
            signals: HashMap::new(),
            dirty: false,
            road_length_m: 0.0,
        }
    }
}

/// The mini map reads the road paths of the resident tiles (read-only, no copy of the lane graph).
impl crate::hud::minimap::MapSource for TrafficState {
    fn roads_near(
        &self,
        centre: DVec2,
        radius_m: f64,
        out: &mut Vec<crate::hud::minimap::MapRoad>,
    ) {
        for segment in self.segments.values().flatten() {
            if crate::hud::minimap::may_touch(&segment.geometry, centre, radius_m) {
                out.push(crate::hud::minimap::MapRoad {
                    geometry: segment.geometry,
                    width: segment.width,
                });
            }
        }
    }
}

/// Groups and vehicle types of the map's AI lists, loaded on demand.
#[derive(Resource, Default)]
pub struct TrafficPool {
    started: bool,
    groups: Vec<PoolGroup>,
    slots: Vec<Slot>,
    /// Default density of each `unsched_vehgroups.txt` group.
    defaults: Vec<u32>,
    /// The vehicles of the depot groups by lower-case name (see [`PoolSetup::depots`]).
    depots: HashMap<String, Vec<(usize, u32)>>,
    /// `global.cfg` `[trafficdensity_road]`: the time curve of groups without their own.
    road_curve: DensityProfile,
    /// The pool group of every vehicle this pool spawned.
    agent_groups: HashMap<AgentId, usize>,
    /// The ceiling bound at the last spawn attempt (logged when that changes).
    capped: bool,
    rng: Option<Rng>,
    frames: u32,
    /// Vehicles built so far; with the slot it seeds each vehicle's repaint choice.
    built: u64,
}

/// Component of a driving AI vehicle.
#[derive(Component)]
pub struct TrafficVehicle {
    id: AgentId,
    delta_height: f32,
    /// Wheel radius (m) used for the wheel rotation (from the vehicle's file when it gives one).
    wheel_radius: f32,
    /// Longitudinal position of the rear axle in the model frame (m): the model origin sits
    /// `rear_axle_y` along the body from the agent's rear axle.
    rear_axle_y: f32,
    /// Accumulated wheel rotation (rad).
    wheel_angle: f32,
    /// Seat occupation (`ai_pax1…4`): bit `i` shows passenger `i + 1`.
    pax: u8,
    /// Phase offset of the indicator flash (s), so vehicles do not blink in unison.
    blink_phase: f32,
    /// Pose of the model origin in the OMSI frame, set every frame; trailing units follow it.
    pub(super) origin: DVec2,
    /// Compass heading (radians).
    pub(super) heading: f64,
    /// Height of the model origin including `[ai_deltaheight]`.
    pub(super) height: f32,
}

#[cfg(test)]
impl TrafficVehicle {
    /// A vehicle standing at a pose, for the tests of the systems that follow it.
    pub(super) fn at(origin: DVec2, heading: f64, height: f32) -> Self {
        Self {
            id: AgentId(0),
            delta_height: 0.0,
            wheel_radius: 0.31,
            rear_axle_y: -1.3,
            wheel_angle: 0.0,
            pax: 0,
            blink_phase: 0.0,
            origin,
            heading,
            height,
        }
    }
}

mod graph;
pub use graph::{configure_traffic, update_traffic_graph};
pub(super) use graph::{register_tile, reset_pool, tile_segments, unregister_tile};

mod variants;
pub use variants::load_traffic_variants;

mod spawn;
pub use spawn::spawn_traffic;

mod run_schedule;
pub use run_schedule::{active_runs, run_phase, RunPhase, RunRef};
mod scheduled;
pub use scheduled::{plan_scheduled, ScheduledStats, ScheduledTraffic};

mod motion;
pub use motion::{move_traffic, update_ai_variables};

#[cfg(test)]
mod density_tests;
