//! The in-progress world load: its state, advanced one tile per frame.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapSplineInstance, TileGrid};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

use super::{chrono, collision, crossing, plate_surf, road_surf, scenery_objects};
use super::{tile_build, tile_ground, vehicle};
use super::{CameraSetup, GroundArray, GroundIndex, OmsiSessionConfig};
use opts_attachments::SplineIndex;

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

/// In-progress world load, advanced one tile per frame.
#[derive(Resource)]
pub struct WorldLoadJob {
    pub(super) install: OmsiInstall,
    pub(super) cache: ContentCache,
    pub(super) config: OmsiSessionConfig,
    pub(super) manifest: MapManifest,
    pub(super) tile_files: HashMap<(i32, i32), String>,
    pub(super) queue: VecDeque<(i32, i32)>,
    /// Tiles currently spawned, by tile coordinates (root entity of each tile).
    pub(super) resident: HashMap<(i32, i32), Entity>,
    /// What stays resident around the camera: a Chebyshev radius or the whole map (M-0080).
    pub(super) tile_mode: opts_tile_loading::TileMode,
    /// Listed tiles whose file is not there (found while loading everything; not asked again).
    pub(super) missing_tiles: HashSet<(i32, i32)>,
    /// The tile whose build is in progress (it is already in `resident`, with its root entity).
    pub(super) building: Option<tile_build::TileBuild>,
    /// How long streaming may work on tiles in one frame (zero: one whole tile per frame).
    pub stream_budget: std::time::Duration,
    /// `streaming.mode` (M-0114): auto follows the speed, tile or object forces the regime.
    pub stream_mode: opts_stream_budget::policy::StreamMode,
    /// The player's position in the scene frame while the object regime holds (M-0114): a tile
    /// built now places its objects nearest to it first. `None` at speed.
    pub(super) stream_focus: Option<(f32, f32)>,
    /// The regime of the last streaming frame, for the reports (M-0114).
    pub stream_status: opts_stream_budget::policy::StreamStatus,
    pub(super) total_tiles: usize,
    pub(super) sli_cache: HashMap<PathBuf, Option<Arc<SplineProfile>>>,
    /// Map-wide spline lookup for attachments, built on first use.
    pub(super) spline_index: Option<Arc<SplineIndex>>,
    /// Transform and object of everything placed on the tile being built, by ID code: the parents
    /// of its `[attachObj]` children.
    pub(super) placed: HashMap<u64, (Transform, Arc<SceneryObject>)>,
    /// The road surfaces of the tile being built: `[rendertype] on_surface` objects (road
    /// markings, text on the road) are laid on them.
    pub(super) tile_ground: Option<GroundIndex>,
    /// The splines of every tile read this session, for the terrain carve of its neighbours.
    pub(super) tile_splines: HashMap<(i32, i32), Arc<Vec<MapSplineInstance>>>,
    /// The ID codes of each such tile's `[spline_terrain_align]` splines (with `tile_splines`).
    pub(super) tile_aligned: HashMap<(i32, i32), Arc<Vec<u64>>>,
    pub(super) sco_cache: HashMap<PathBuf, Option<Arc<SceneryObject>>>,
    /// Parsed sound configurations by file (`None`: unreadable).
    pub(super) sound_cfgs: HashMap<PathBuf, Option<Arc<opts_core::formats::sound::SoundConfig>>>,
    /// The sound layer of `opts-game` the load calls (ISS-064); set by `set_sound_hooks`.
    pub(super) sound_hooks: Arc<dyn crate::sound_hooks::SoundHooks>,
    /// The passenger rig of `opts-game` the load attaches and joins (ISS-064); set by
    /// `set_passenger_hooks`.
    pub(super) passenger_hooks: Arc<dyn crate::passenger_hooks::PassengerHooks>,
    /// The session-vehicle layer of `opts-game` the load calls (ISS-064); set by `set_session_ops`.
    pub(super) session_ops: Arc<dyn crate::session_ops::SessionVehicleOps>,
    /// `[crossing_heightdeformation]` base surfaces by file.
    pub(super) crossing_bases:
        HashMap<PathBuf, Option<Arc<opts_core::formats::scenery::BaseSurface>>>,
    /// Ground faces of crossings and surface objects by `.sco` (mesh axes), for the carve.
    pub(super) ground_faces: HashMap<PathBuf, crossing::ObjectFaces>,
    /// The `[object]`s of tiles read for the carve of their neighbours.
    pub(super) tile_objects:
        HashMap<(i32, i32), Arc<Vec<opts_core::formats::map::MapObjectInstance>>>,
    /// The drawn ground surfaces of each tile (its splines' ribbons and its crossings' faces) in
    /// the tile's own frame, kept for the carve of the tile and its eight neighbours: one
    /// computation per tile instead of nine (M-0089 step 2). Pruned to the resident tiles'
    /// neighbourhood after every build; cleared with the splines and objects it derives from.
    pub(super) tile_surfaces: HashMap<(i32, i32), Arc<tile_ground::TileGround>>,
    /// The map's timetable files, read when the first vehicle needs them (`None` inside: the map
    /// has none).
    pub(super) timetables: Option<Option<Arc<opts_core::formats::timetable::Timetables>>>,
    /// Bounding boxes of collision meshes in Bevy's local axes, by file.
    pub(super) collision_bounds: collision::BoundsCache,
    /// Compiled scenery scripts, light assets and `[varparent]` links (scenery runtime).
    pub(super) scenery: scenery_objects::SceneryObjectCache,
    /// Chrono entries and what is active on the map date (`chrono.rs`).
    pub(super) chrono: chrono::ChronoContext,
    /// Entries of the map's parking lists by selector (`0` = `parklist_p.txt`, `N` =
    /// `parklist_p_N.txt`), read on first use (empty when absent).
    pub(super) parklists: HashMap<u32, Vec<String>>,
    pub(super) spline_materials: HashMap<(PathBuf, usize), Handle<StandardMaterial>>,
    /// The strips with a height map of each road profile (by `.sli` name) and the decoded
    /// `.surf` maps by file (M-0146).
    pub(super) surf_strips: HashMap<String, Arc<Vec<road_surf::TexStrip>>>,
    pub(super) surf_maps: HashMap<PathBuf, Option<Arc<road_surf::SurfMap>>>,
    /// The upward triangles of surface-plate meshes, by mesh file and crossing base (M-0146).
    pub(super) plate_tris: HashMap<plate_surf::PlateKey, Arc<Vec<plate_surf::LocalTri>>>,
    /// Bitmap fonts and rendered text textures (`[texttexture]`).
    pub(super) fonts: opts_bevy::text_texture::FontRegistry,
    /// `[texttexture]` declarations of the vehicle model being loaded (set by its loader; the
    /// segment loader binds `[useTextTexture]` indices against them).
    pub(super) vehicle_text_defs: Vec<opts_core::formats::vehicles::model::TextTexture>,
    /// `[scripttexture]` declarations of the vehicle model being loaded.
    pub(super) vehicle_script_defs: Vec<opts_core::formats::vehicles::model::ScriptTexture>,
    /// Script texture canvases for the active vehicle model.
    pub(super) vehicle_script_textures:
        Arc<std::sync::RwLock<Vec<opts_core::formats::script_texture::CpuScriptTexture>>>,
    pub(super) ground_material: Option<Handle<StandardMaterial>>,
    /// Water of the tiles built so far, by tile key (for [`WorldLoadJob::water_level_at`]).
    pub(super) tile_water: HashMap<(i32, i32), opts_core::formats::map::MapWater>,
    /// Shared material of the water surfaces.
    pub(super) water_material: Option<Handle<StandardMaterial>>,
    /// Whether `.map.water` surfaces are drawn.
    pub water_enabled: bool,
    /// Decoded `[groundtex]` layers as one GPU array, built on first use.
    pub(super) ground_array: Option<GroundArray>,
    /// Files each resident tile was built from (normalised paths); drives hot reload.
    pub(super) tile_deps: HashMap<(i32, i32), HashSet<PathBuf>>,
    /// Files collected while the current tile loads.
    pub(super) loading_deps: HashSet<PathBuf>,
    /// Directories of the player vehicle; a changed file inside restarts the session.
    pub(super) vehicle_dirs: Vec<PathBuf>,
    pub(super) spawn: Option<(Vec3, f32)>,
    /// The cars behind the head of a train formation about to be spawned (consumed by the loader).
    pub(super) formation_cars: Vec<vehicle::FormationCar>,
    /// Repaints of the player's vehicle, read when it is loaded.
    pub(super) repaint: Option<opts_repaints::VehicleRepaints>,
    /// Repaints of parked/static vehicle models, by model config path (read on first use).
    pub(super) model_repaints: HashMap<PathBuf, opts_repaints::VehicleRepaints>,
    pub(super) start_tile: (i32, i32),
    /// Tile grid of the map; the session frame is anchored at `start_tile`.
    pub(super) grid: TileGrid,
    /// Tiles of the 3x3 neighbourhood that `global.cfg` lists but whose file is missing.
    pub(super) missing_listed: usize,
    /// Camera parameters, set once the vehicle is loaded.
    pub camera: Option<CameraSetup>,
    /// `[set_camera_std]` index of the loaded vehicle.
    pub camera_std_index: usize,
    /// The session starts with the map only (no vehicle is loaded at the start).
    pub no_vehicle: bool,
    /// Share of the parking bays that hold a vehicle, per mille (setting `gameplay.parked`).
    pub parked_permille: u32,
    /// The entry record found by the last [`WorldLoadJob::find_entry_spawn`] search.
    pub(super) resolved_entry: Option<(u32, String)>,
    /// The entry the session started at (`None`: the start fell back to a road spline).
    pub(super) start_entry: Option<(u32, String)>,
}
