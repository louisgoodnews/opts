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
use opts_core::formats::map::MapManifest;
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

use crate::in_game::attachments::SplineIndex;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
mod collision;
mod dev_look;
mod ground;
mod ground_height;
mod job;
mod overlay;
mod pedestrians;
pub(crate) mod rail;
mod reload;
mod respawn;
mod scenery_objects;
mod tile;
mod tile_ground;
mod traffic;
#[cfg(test)]
pub(crate) use pedestrians::DEFAULT_MAX_PEDESTRIANS;
#[cfg(test)]
pub(crate) use traffic::DEFAULT_MAX_TRAFFIC;
mod trees;
mod vehicle;
mod vehicle_text;

pub use collision::{
    attach_traffic_colliders, player_collisions, BodyBox, CollisionIndex, PlayerBox,
    VehicleCollider,
};
pub use dev_look::place_dev_camera;
use ground::GroundArray;
pub use ground_height::{follow_ground, GroundIndex};
pub use job::{begin_world_load, resolve_session_config};
pub use overlay::{
    log_final_position, spawn_debug_overlay, track_position, update_debug_overlay, DebugOverlayText,
};
pub use pedestrians::{
    configure_pedestrians, load_variants, move_pedestrians, spawn_pedestrians, Pedestrian,
    PedestrianNetwork, PedestrianPool, PedestrianSettings,
};
pub use rail::{
    configure_rail, is_rail, load_rail_variants, move_ai_trains, rail_player_pose,
    rail_player_step, spawn_ai_trains, AiTrain, BikeAiPool, BikeNetwork, BikeSettings, Part,
    RailAiPool, RailNetwork, RailPlayer, RailSettings, VehicleComposition,
};
pub use reload::ReloadPlan;
pub use respawn::{
    handle_vehicle_requests, DespawnVehicleRequest, EntryIndex, ResolvedEntry, SpawnVehicleRequest,
};
pub use scenery_objects::{sync_scenery_clock, SceneryWiringStats};
pub use tile::{load_next_tile, stream_world_tiles, TileRoot};
pub use traffic::{
    configure_traffic, load_traffic_variants, move_traffic, spawn_traffic, update_ai_variables,
    update_traffic_graph, TrafficPool, TrafficSettings, TrafficState, TrafficVehicle,
};
pub use vehicle::{decodes_pending, load_player_vehicle, prewarm_vehicle_meshes};
pub use vehicle_text::update_vehicle_text_textures;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
#[derive(bevy::ecs::system::SystemParam)]
pub struct LoadAssets<'w> {
    /// Meshes.
    pub meshes: ResMut<'w, Assets<Mesh>>,
    /// Materials.
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    /// Images.
    pub images: ResMut<'w, Assets<Image>>,
    /// Layered ground materials (absent when the renderer lacks the material plugin).
    pub ground_materials: Option<ResMut<'w, Assets<GroundMaterial>>>,
    /// Skin inverse bind poses.
    pub bindposes: ResMut<'w, Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
}

/// Where the session content comes from, resolved from CLI arguments.
#[derive(Clone)]
pub struct OmsiSessionConfig {
    /// OMSI installation root.
    pub root: PathBuf,
    /// Map folder (`<root>/maps/<name>`).
    pub map_dir: PathBuf,
    /// Explicit `.bus` file, if given.
    pub vehicle_path: Option<PathBuf>,
    /// Entry point name chosen in the New Game screen.
    pub entry_point: String,
    /// Repaint of the player's vehicle (empty: the default of its `.bus`).
    pub livery: String,
    /// Equipment options (`[setvar]`-style script variables) set on the vehicle's scripts after
    /// the repaint's own.
    pub setvars: Vec<(String, i32)>,
    /// `.hum` profile of the player's character, if the installation has one.
    pub human_path: Option<PathBuf>,
    /// Cached path index (the SQLite catalogue), if available.
    pub lookup: Option<opts_bevy::install::PathLookup>,
    /// Cached by-name index for repairing wrong references.
    pub name_lookup: Option<opts_bevy::install::NameLookup>,
    /// Cached all-files-by-name index (plain twins of scrambled meshes).
    pub twin_lookup: Option<opts_bevy::install::TwinLookup>,
    /// Finder of files installed under other folder names than their references say.
    pub relocate_lookup: Option<opts_bevy::install::RelocateLookup>,
}

/// Counters for the diagnostic overlay and the final report.
/// OMSI-frame position (x east, y north, z up, metres) of something in the world.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldPos {
    /// Tile coordinates.
    pub tile: (i32, i32),
    /// Absolute OMSI world position.
    pub world: Vec3,
    /// Position inside the tile (x, y in 0..300).
    pub local: Vec2,
}

/// Camera and vehicle positions for the diagnostic overlay and the log.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PositionReadout {
    /// The 3D camera.
    pub camera: Option<WorldPos>,
    /// The player's vehicle and its heading in degrees (clockwise from north).
    pub vehicle: Option<(WorldPos, f32)>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct WorldStats {
    pub map_name: String,
    pub start_tile: (i32, i32),
    /// Where the camera and the player's vehicle are (updated every frame).
    pub position: PositionReadout,
    pub tiles_loaded: usize,
    pub tiles_missing: usize,
    pub objects_loaded: usize,
    pub objects_failed: usize,
    /// Objects whose meshes are all scrambled (protected) and therefore not rendered.
    pub objects_protected: usize,
    /// Objects that legitimately have no mesh (path/traffic-light/helper objects).
    pub objects_no_geometry: usize,
    /// `[tree]` billboards spawned (included in `objects_loaded`).
    pub trees_loaded: usize,
    pub splines_loaded: usize,
    pub splines_failed: usize,
    pub scene_vertices: usize,
    pub vehicle_name: String,
    pub vehicle_meshes: usize,
    pub vehicle_vertices: usize,
    pub asset_failures: usize,
    /// Scrambled meshes recovered by the decoder / replaced by a plain twin / still decoding.
    pub meshes_decoded: usize,
    pub meshes_twin: usize,
    pub meshes_decoding: usize,
    pub overlay_visible: bool,
}

/// Vehicle-derived camera parameters handed from vehicle loading to camera setup.
#[derive(Debug, Clone, Copy)]
pub struct CameraSetup {
    pub start: Transform,
    pub chase_distance: f32,
    pub chase_height: f32,
    pub cockpit_offset: Vec3,
}

/// In-progress world load, advanced one tile per frame.
#[derive(Resource)]
pub struct WorldLoadJob {
    install: OmsiInstall,
    cache: ContentCache,
    config: OmsiSessionConfig,
    manifest: MapManifest,
    tile_files: HashMap<(i32, i32), String>,
    queue: VecDeque<(i32, i32)>,
    /// Tiles waiting for background mesh decoding, with the meshes they wait for.
    waiting_decode: HashMap<(i32, i32), HashSet<opts_bevy::descramble_cache::Key>>,
    /// Tiles currently spawned, by tile coordinates (root entity of each tile).
    resident: HashMap<(i32, i32), Entity>,
    /// Chebyshev radius of tiles kept loaded around the camera.
    stream_radius: i32,
    total_tiles: usize,
    sli_cache: HashMap<PathBuf, Option<Arc<SplineProfile>>>,
    /// Map-wide spline lookup for attachments, built on first use.
    spline_index: Option<Arc<SplineIndex>>,
    sco_cache: HashMap<PathBuf, Option<Arc<SceneryObject>>>,
    /// Bounding boxes of collision meshes in Bevy's local axes, by file.
    collision_bounds: collision::BoundsCache,
    /// Compiled scenery scripts, light assets and `[varparent]` links (scenery runtime).
    scenery: scenery_objects::SceneryObjectCache,
    /// Entries of the map's `parklist_p.txt`, read on first use (empty when absent).
    parklist: Option<Vec<String>>,
    spline_materials: HashMap<(PathBuf, usize), Handle<StandardMaterial>>,
    /// Bitmap fonts and rendered text textures (`[texttexture]`).
    fonts: opts_bevy::text_texture::FontRegistry,
    /// `[texttexture]` declarations of the vehicle model being loaded (set by its loader; the
    /// segment loader binds `[useTextTexture]` indices against them).
    vehicle_text_defs: Vec<opts_core::formats::vehicles::model::TextTexture>,
    ground_material: Option<Handle<StandardMaterial>>,
    /// Decoded `[groundtex]` layers as one GPU array, built on first use.
    ground_array: Option<GroundArray>,
    /// Files each resident tile was built from (normalised paths); drives hot reload.
    tile_deps: HashMap<(i32, i32), HashSet<PathBuf>>,
    /// Files collected while the current tile loads.
    loading_deps: HashSet<PathBuf>,
    /// Directories of the player vehicle; a changed file inside restarts the session.
    vehicle_dirs: Vec<PathBuf>,
    spawn: Option<(Vec3, f32)>,
    /// Repaints of the player's vehicle, read when it is loaded.
    repaint: Option<crate::new_game_screen::repaints::VehicleRepaints>,
    start_tile: (i32, i32),
    /// Camera parameters, set once the vehicle is loaded.
    pub camera: Option<CameraSetup>,
    /// `[set_camera_std]` index of the loaded vehicle.
    pub camera_std_index: usize,
}

/// Inputs for [`resolve_session_config`]: CLI overrides plus the menu selection.
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionRequest<'a> {
    /// `--map-path` override.
    pub map_path: Option<&'a Path>,
    /// `--omsi-path` override.
    pub omsi_path: Option<&'a Path>,
    /// `--vehicle-path` override.
    pub vehicle_path: Option<&'a Path>,
    /// Map chosen in the New Game screen (friendly name, name or folder id).
    pub selected_map: &'a str,
    /// Vehicle chosen in the New Game screen (full name or catalogue id).
    pub selected_vehicle: &'a str,
    /// Entry point chosen in the New Game screen (name from `global.cfg`).
    pub selected_entry_point: &'a str,
    /// Repaint chosen in the New Game screen (empty: the vehicle's default).
    pub selected_livery: &'a str,
}
