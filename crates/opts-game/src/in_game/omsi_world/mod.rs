//! Loads real OMSI content (map tiles, splines, scenery objects, vehicle) at runtime.
//!
//! Every mesh, texture and position comes from the OMSI installation: nothing here fabricates
//! geometry. Assets that cannot be loaded are logged at `ERROR` and skipped.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapSplineInstance, TileGrid};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

use crate::in_game::attachments::SplineIndex;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
mod chrono;
mod collision;
mod consist_plan;
mod crossing;
mod dev_look;
mod ground;
mod ground_follow;
mod ground_height;
mod job;
pub(crate) mod mesh_dir;
mod overlay;
mod overlay_info;
mod plate_surf;
mod road_surf;
mod road_surf_load;
pub use overlay_info::DeveloperDetail;
mod pedestrians;
pub(crate) mod rail;
mod reload;
mod respawn;
mod scenery_objects;
mod setvar_state;
pub(crate) mod spatial_zone_overlay;
mod tile;
mod tile_build;
mod tile_ground;
mod tile_mode;
pub(crate) mod trace;
pub(crate) mod traffic;
mod traffic_chain;
mod traffic_variant;
pub(crate) use pedestrians::passenger_factor;
#[cfg(test)]
pub(crate) use pedestrians::DEFAULT_PEDESTRIAN_BASELINE;
pub(crate) use traffic::line_runs_on;
#[cfg(test)]
pub(crate) use traffic::DEFAULT_TRAFFIC_BASELINE;
mod fleet_number;
mod trees;
mod vehicle;
pub mod vehicle_script_texture;
mod vehicle_text;
mod water;

pub use chrono::{apply_map_date_changes, chrono_date, track_chrono_rebuild, ChronoState};
pub use collision::{
    attach_traffic_colliders, player_collisions, publish_traffic_obstacles, BodyBox,
    CollisionIndex, PlayerBox, VehicleCollider,
};
pub use dev_look::place_dev_camera;
use ground::GroundArray;
pub use ground_follow::{follow_ground, sample_consist_ground};
pub use ground_height::GroundIndex;
pub use job::{begin_world_load, begin_world_load_at, resolve_session_config};
pub use overlay::{
    log_final_position, spawn_debug_overlay, toggle_developer_detail, toggle_developer_panel,
    track_position, update_debug_overlay, update_wheel_vars_overlay, DebugOverlayText,
    DebugWheelVars, WheelVarsText,
};
pub(crate) use pedestrians::{advance_phase, apply_gait, apply_rest_pose, apply_seated_pose};
pub use pedestrians::{
    configure_pedestrians, load_variants, move_pedestrians, spawn_pedestrians, Pedestrian,
    PedestrianNetwork, PedestrianPool, PedestrianSettings,
};
pub use rail::{
    click_points_system, configure_rail, is_rail, load_rail_variants, move_ai_trains, points_ahead,
    points_visual_system, rail_player_pose, rail_player_step, set_points_system, spawn_ai_trains,
    spawn_scheduled_trains, step_rail_scripts, switch_hint_system, way_by_id, way_id, AiTrain,
    BikeAiPool, BikeNetwork, BikeSettings, Part, RailAiPool, RailNetwork, RailPlayer, RailSettings,
    ScheduledTrain, VehicleComposition,
};
pub use reload::{ReloadPlan, TileUnload};
pub use respawn::{handle_vehicle_requests, EntryIndex, ResolvedEntry, SpawnVehicleRequest};
pub use scenery_objects::{sync_scenery_clock, SceneryWiringStats};
pub use tile::{
    advance_tile_loading, load_next_tile, stream_world_tiles, sync_tile_zones, TileRoot,
};
pub use traffic::{
    active_runs, configure_traffic, load_traffic_variants, move_traffic, plan_scheduled, run_phase,
    spawn_traffic, step_ai_scripts, update_ai_variables, update_traffic_graph, RunPhase, RunRef,
    ScheduledStats, ScheduledTraffic, TrafficPool, TrafficSettings, TrafficState, TrafficVehicle,
};
use traffic::{engine_for, next_doors, Doors, DOORS_OPEN_SHARE, FRAME_S, SCRIPT_RADIUS_M};
pub use traffic_chain::move_traffic_chains;
pub use vehicle::{load_player_vehicle, FormationCar, SpawnedVehicle};
pub use vehicle_script_texture::{update_vehicle_script_textures, VehicleScriptTexture};
pub use vehicle_text::update_vehicle_text_textures;
pub use water::warn_vehicle_in_water;

mod load_job;
mod session;
mod stats;
pub use load_job::{LoadAssets, WorldLoadJob};
pub use session::{OmsiSessionConfig, SessionRequest};
pub use stats::{CameraSetup, PositionReadout, WorldPos, WorldStats};
