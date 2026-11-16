//! # OPTS Bevy (`opts-bevy`)
//!
//! Bevy engine integration, coordinate conversion bridges, and asset loaders
//! for Open Public Transport Simulator (OPTS).
//!
//! This crate bridges pure OMSI data structures parsed by `opts-core` into native
//! Bevy ECS components and rendering resources (`Mesh`, `Material`, `AssetLoader`).
//!
//! ## Coordinate Conversions
//!
//! - **OMSI Meshes (`.o3d`, `.x`)**: Left-handed Y-up (+X right, +Y up, +Z forward).
//! - **OMSI Entities & Vehicles (`.bus`, `.map`)**: Right-handed Z-up (+X right, +Y forward, +Z up).
//! - **Bevy Engine Space**: Right-handed Y-up (+X right, +Y up, -Z forward, +Z back).
//!
//! See [`coordinates.md`](../../../docs/src/architecture/coordinates.md) for full details.

#![deny(unsafe_code)]

pub mod action_keys;
pub mod ai_variables;
pub mod articulation;
pub mod audio;
pub mod bus_stops;
pub mod cab_switch;
pub mod camera;
pub mod capture;
pub mod consist_scripts;
pub mod control;
pub mod coords;
pub mod door_camera;
pub mod environment;
pub mod free_texture;
pub mod glass;
pub mod ground;
pub mod ground_material;
pub mod host_callbacks;
pub mod hot_reload;
pub mod human;
pub mod install;
pub mod instancing;
pub mod light_cones;
pub mod bloom;
pub mod light_quality;
pub mod light_rooms;
pub mod loader;
pub mod material;
pub mod mesh;
pub mod mirrors;
pub mod model;
pub mod moon;
pub mod moon_material;
pub mod mouse_driving;
pub mod plugin;
pub mod position_tag;
pub mod rail_controls;
pub mod reflections;
pub mod render_stats;
pub mod reporting;
pub mod route;
pub mod scene_trace;
pub mod scenery;
pub mod scenery_runtime;
pub mod script_text;
pub mod sky_cube;
pub mod spatial_zones;
pub mod sun_rays;
pub mod terrain;
pub mod terrain_query;
pub mod text_texture;
pub mod texture;
pub mod texture_filter;
pub mod texture_sentinel;
pub mod tree_wind;
pub mod vehicle;
pub mod vehicle_anim;
pub mod vehicle_glow;
pub mod vehicle_lights;
pub mod vehicle_remote;
pub mod vehicle_script;
pub mod vehicle_setup;
pub mod vehicle_sim;
pub mod vehicle_triggers;
pub mod view_scope;
pub mod water;
pub mod weather;

mod reexports;

pub use reexports::*;


/// System set of the rail-bound player's track step: runs after the road physics and before the
/// articulation of the following units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bevy::ecs::schedule::SystemSet)]
pub struct RailSet;
