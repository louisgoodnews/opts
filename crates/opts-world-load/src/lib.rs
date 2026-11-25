//! Loads real OMSI content (map tiles, splines, scenery objects, vehicle) at runtime.
//!
//! Every mesh, texture and position comes from the OMSI installation: nothing here fabricates
//! geometry. Assets that cannot be loaded are logged at `ERROR` and skipped.
//!
//! This crate is the former `opts-game` `in_game::omsi_world` closure of `WorldLoadJob` (ISS-064).
//! The modules that only consume the job (`ground_follow`, `overlay`, `overlay_info`,
//! `spatial_zone_overlay`) stay in `opts-game` and import from here.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapSplineInstance, TileGrid};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

pub use opts_attachments::SplineIndex;

pub mod map_source;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
pub mod chrono;
pub mod collision;
pub mod consist_plan;
pub mod crossing;
pub mod ai_weather;
pub mod dev_look;
pub mod formation;
pub mod ground;
pub mod ground_height;
pub mod job;
pub use opts_mesh_dir as mesh_dir;
pub mod plate_surf;
pub use opts_road_surf as road_surf;
pub mod road_surf_load;
pub mod passenger_hooks;
pub mod pedestrians;
pub mod rail;
pub mod reload;
pub mod respawn;
pub mod scenery_objects;
pub mod session_ops;
pub mod setvar_state;
pub mod sound_hooks;
pub mod tile;
pub mod tile_build;
pub mod tile_ground;
pub mod tile_mode;
pub mod trace;
pub mod traffic;
pub mod traffic_chain;
pub mod traffic_variant;
pub use opts_fleet_number as fleet_number;
pub use pedestrians::passenger_factor;
#[cfg(test)]
pub use pedestrians::DEFAULT_PEDESTRIAN_BASELINE;
pub use traffic::line_runs_on;
#[cfg(test)]
pub use traffic::DEFAULT_TRAFFIC_BASELINE;
pub mod trees;
pub mod vehicle;
pub use opts_vehicle_script_texture as vehicle_script_texture;
pub mod vehicle_text;
pub mod water;

pub use chrono::{apply_map_date_changes, chrono_date, track_chrono_rebuild, ChronoState};
pub use collision::{
    attach_traffic_colliders, player_collisions, publish_traffic_obstacles, BodyBox,
    CollisionIndex, PlayerBox, VehicleCollider,
};
pub use dev_look::place_dev_camera;
use ground::GroundArray;
pub use ground_height::GroundIndex;
pub use job::{begin_world_load, begin_world_load_at, resolve_session_config};
pub use pedestrians::{advance_phase, apply_gait, apply_rest_pose, apply_seated_pose};
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
    active_runs, configure_traffic, dispatch_responders, load_traffic_variants, move_traffic,
    plan_scheduled, run_phase, spawn_traffic, step_ai_scripts, update_ai_variables,
    update_traffic_graph, Incident, IncidentQueue, RunPhase, RunRef, ScheduledStats,
    ScheduledTraffic, ServiceKind, TrafficPool, TrafficSettings, TrafficState, TrafficVehicle,
};
use traffic::{engine_for, next_doors, Doors, DOORS_OPEN_SHARE, FRAME_S, SCRIPT_RADIUS_M};
pub use traffic_chain::move_traffic_chains;
pub use vehicle::{load_player_vehicle, FormationCar, SpawnedVehicle};
pub use vehicle_script_texture::{update_vehicle_script_textures, VehicleScriptTexture};
pub use vehicle_text::update_vehicle_text_textures;
pub use water::warn_vehicle_in_water;

pub mod load_job;
pub use opts_world_session as session;
pub use opts_world_stats as stats;
pub use load_job::{LoadAssets, WorldLoadJob};
pub use session::{OmsiSessionConfig, SessionRequest};
pub use stats::{CameraSetup, PositionReadout, WorldPos, WorldStats};
