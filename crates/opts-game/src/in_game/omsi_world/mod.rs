//! Loads real OMSI content (map tiles, splines, scenery objects, vehicle) at runtime.
//!
//! Every mesh, texture and position comes from the OMSI installation: nothing here fabricates
//! geometry. Assets that cannot be loaded are logged at `ERROR` and skipped.
//!
//! The world-load closure (the `WorldLoadJob` and the code it reaches) is in the crate
//! `opts-world-load` (ISS-064). This module keeps the consumers (`ground_follow`, `overlay`,
//! `overlay_info`, `spatial_zone_overlay`) and re-exports the moved items under their old paths.

mod ground_follow;
mod overlay;
mod overlay_info;
pub(crate) mod spatial_zone_overlay;

pub use overlay_info::DeveloperDetail;

pub use ground_follow::{follow_ground, sample_consist_ground};
pub use overlay::{
    log_final_position, spawn_debug_overlay, toggle_developer_detail, toggle_developer_panel,
    track_position, update_debug_overlay, update_wheel_vars_overlay, DebugOverlayText,
    DebugWheelVars, WheelVarsText,
};

// Moved modules, kept under their old paths for the consumers.
pub(crate) use opts_world_load::{
    chrono, collision, consist_plan, crossing, dev_look, fleet_number, ground, ground_height, job,
    load_job, mesh_dir, pedestrians, plate_surf, rail, reload, respawn, road_surf, road_surf_load,
    scenery_objects, session, setvar_state, stats, tile, tile_build, tile_ground, tile_mode, trace,
    traffic, traffic_chain, traffic_variant, trees, vehicle, vehicle_script_texture, vehicle_text,
    water,
};

// Moved items, re-exported with their old paths.
pub(crate) use opts_world_load::{
    advance_phase, apply_gait, apply_rest_pose, apply_seated_pose, line_runs_on, passenger_factor,
};
#[cfg(test)]
pub(crate) use opts_world_load::{DEFAULT_PEDESTRIAN_BASELINE, DEFAULT_TRAFFIC_BASELINE};
pub use opts_world_load::{
    active_runs, advance_tile_loading, apply_map_date_changes, attach_traffic_colliders,
    begin_world_load, begin_world_load_at, chrono_date, click_points_system, configure_pedestrians,
    configure_rail, configure_traffic, dispatch_responders, handle_vehicle_requests, is_rail,
    load_next_tile, load_player_vehicle, load_rail_variants, load_traffic_variants, load_variants,
    move_ai_trains, move_pedestrians, move_traffic, move_traffic_chains, player_collisions,
    plan_scheduled, points_ahead, points_visual_system, publish_traffic_obstacles,
    rail_player_pose, rail_player_step, resolve_session_config, run_phase, set_crossing_aspects,
    set_points_system, spawn_ai_trains, spawn_pedestrians, spawn_scheduled_trains, spawn_traffic,
    step_ai_scripts, step_rail_scripts, stream_world_tiles, switch_hint_system, sync_scenery_clock,
    sync_tile_zones,
    place_dev_camera, track_chrono_rebuild, update_ai_variables, update_traffic_graph,
    update_vehicle_script_textures, update_vehicle_text_textures, warn_vehicle_in_water,
    way_by_id, way_id, AiTrain, BikeAiPool, BikeNetwork, BikeSettings, BodyBox, ChronoState,
    CollisionIndex, EntryIndex, FormationCar, GroundIndex, Incident, IncidentQueue, LoadAssets,
    OmsiSessionConfig, Part, PedestrianNetwork, PedestrianPool, PedestrianSettings, Pedestrian,
    PlayerBox, PositionReadout, RailAiPool, RailNetwork, RailPlayer, RailSettings, ReloadPlan,
    ResolvedEntry, RunPhase, RunRef, ScheduledStats, ScheduledTrain, SceneryWiringStats,
    SessionRequest, SpawnVehicleRequest, SpawnedVehicle, TileRoot, TileUnload, TrafficPool,
    TrafficSettings, TrafficState, TrafficVehicle, VehicleCollider, VehicleComposition,
    VehicleScriptTexture, WorldLoadJob, WorldPos, WorldStats, CameraSetup, ScheduledTraffic,
    ServiceKind,
};
