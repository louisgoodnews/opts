//! Loads real OMSI content (map tiles, splines, scenery objects, vehicle) at runtime.
//!
//! Every mesh, texture and position comes from the OMSI installation: nothing here fabricates
//! geometry. Assets that cannot be loaded are logged at `ERROR` and skipped.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::coords::{
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
};
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{
    attach_model_parts, build_terrain_mesh, extrude_spline_segment, sample_terrain,
    spawn_vehicle_model, terrain_material, tree_billboard_mesh, ContentCache, ContentLoader,
    OmsiInstall, PlayerVehicle, TextureOverride, VehicleController, TILE_SIZE_M,
};
use opts_core::formats::map::{MapManifest, MapObjectInstance, MapTerrain, MapTile};
use opts_core::formats::scenery::{SceneryObject, ScoTree, SplineProfile};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use crate::catalogue_service::CatalogueService;
use crate::in_game::attachments::{expand_attachments, SplineIndex};
use crate::in_game::types::InGameEntity;
use crate::new_game_screen::types::GameCatalogue;
use crate::omsi_root::locate_omsi_root;

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
    pub bindposes: ResMut<'w, Assets<bevy::render::mesh::skinning::SkinnedMeshInverseBindposes>>,
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
    /// `.hum` profile of the player's character, if the installation has one.
    pub human_path: Option<PathBuf>,
    /// Cached path index (the SQLite catalogue), if available.
    pub lookup: Option<opts_bevy::install::PathLookup>,
    /// Cached by-name index for repairing wrong references.
    pub name_lookup: Option<opts_bevy::install::NameLookup>,
    /// Cached all-files-by-name index (plain twins of scrambled meshes).
    pub twin_lookup: Option<opts_bevy::install::TwinLookup>,
}

/// Counters for the diagnostic overlay and the final report.
#[derive(Resource, Debug, Clone, Default)]
pub struct WorldStats {
    pub map_name: String,
    pub start_tile: (i32, i32),
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
    /// Tiles currently spawned, by tile coordinates (root entity of each tile).
    resident: HashMap<(i32, i32), Entity>,
    /// Chebyshev radius of tiles kept loaded around the camera.
    stream_radius: i32,
    total_tiles: usize,
    sli_cache: HashMap<PathBuf, Option<Arc<SplineProfile>>>,
    /// Map-wide spline lookup for attachments, built on first use.
    spline_index: Option<Arc<SplineIndex>>,
    sco_cache: HashMap<PathBuf, Option<Arc<SceneryObject>>>,
    spline_materials: HashMap<(PathBuf, usize), Handle<StandardMaterial>>,
    ground_material: Option<Handle<StandardMaterial>>,
    /// Decoded `[groundtex]` layers as one GPU array, built on first use.
    ground_array: Option<GroundArray>,
    spawn: Option<(Vec3, f32)>,
    start_tile: (i32, i32),
    /// Camera parameters, set once the vehicle is loaded.
    pub camera: Option<CameraSetup>,
    /// `[set_camera_std]` index of the loaded vehicle.
    pub camera_std_index: usize,
}

impl WorldLoadJob {
    /// Root folder of the OMSI installation the session reads from.
    #[must_use]
    pub fn omsi_root(&self) -> &std::path::Path {
        self.install.root()
    }

    /// Fraction of tiles already processed.
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.total_tiles == 0 {
            1.0
        } else {
            1.0 - self.queue.len() as f32 / self.total_tiles as f32
        }
    }

    /// Whether all queued tiles have been loaded.
    #[must_use]
    pub fn tiles_done(&self) -> bool {
        self.queue.is_empty()
    }

    /// Next tile to load, for status messages.
    #[must_use]
    pub fn next_tile(&self) -> Option<(i32, i32)> {
        self.queue.front().copied()
    }
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
}

/// Resolves map folder, installation root and vehicle for the session.
///
/// CLI paths win; otherwise the menu selection is mapped through the catalogue: a map's
/// catalogue id is its folder under `<OMSI>/maps`, a vehicle's id is its `.bus` file stem.
pub fn resolve_session_config(
    req: SessionRequest<'_>,
    catalogue: Option<&GameCatalogue>,
    service: Option<&CatalogueService>,
) -> Result<OmsiSessionConfig, String> {
    let root = req
        .omsi_path
        .map(Path::to_path_buf)
        .or_else(|| req.map_path.and_then(OmsiInstall::root_from_map_dir))
        .or_else(|| service.map(|s| s.root.clone()))
        .or_else(|| locate_omsi_root(None))
        .ok_or("OMSI installation not found: pass --omsi-path or set $OMSI_PATH")?;
    let mut install = OmsiInstall::new(&root);
    if let Some(s) = service {
        install = install
            .with_lookup(s.path_lookup())
            .with_name_lookup(s.name_lookup())
            .with_twin_lookup(s.twin_lookup());
    }

    let map_dir = match req.map_path {
        Some(p) => p.to_path_buf(),
        None => {
            let id = catalogue
                .and_then(|c| {
                    c.maps.iter().find(|m| {
                        m.id == req.selected_map
                            || m.name == req.selected_map
                            || m.friendly_name == req.selected_map
                    })
                })
                .map_or(req.selected_map, |m| m.id.as_str());
            install.find_in(&root.join("maps"), id).ok_or_else(|| {
                format!(
                    "map folder '{id}' not found under {}",
                    root.join("maps").display()
                )
            })?
        }
    };
    if !map_dir.is_dir() {
        return Err(format!("map folder does not exist: {}", map_dir.display()));
    }

    let vehicle_path = match req.vehicle_path {
        Some(p) => Some(p.to_path_buf()),
        None => {
            let id = catalogue
                .and_then(|c| {
                    c.vehicles.iter().find(|v| {
                        v.id == req.selected_vehicle || v.full_name == req.selected_vehicle
                    })
                })
                .map(|v| v.id.as_str());
            id.and_then(|id| {
                service
                    .and_then(|s| s.catalogue.vehicle_marker(id).ok().flatten())
                    .map(|rel| root.join(rel))
                    .or_else(|| find_bus_file(&install, id))
            })
        }
    };
    if vehicle_path.is_none() {
        tracing::warn!(
            selected = req.selected_vehicle,
            "selected vehicle has no .bus file in the installation; falling back to the map's ailists.cfg"
        );
    }
    let human_path = pick_driver_human(&root, service);
    Ok(OmsiSessionConfig {
        root,
        map_dir,
        vehicle_path,
        entry_point: req.selected_entry_point.to_string(),
        human_path,
        lookup: service.map(CatalogueService::path_lookup),
        name_lookup: service.map(CatalogueService::name_lookup),
        twin_lookup: service.map(CatalogueService::twin_lookup),
    })
}

/// Chooses the player's character: a `.hum` whose name says *driver*, else the first one.
fn pick_driver_human(root: &Path, service: Option<&CatalogueService>) -> Option<PathBuf> {
    let list = service?.catalogue.human_markers().ok()?;
    let rel = list
        .iter()
        .find(|p| p.to_lowercase().contains("driver"))
        .or_else(|| list.first())?;
    Some(root.join(rel))
}

/// Finds `<OMSI>/Vehicles/<any folder>/<id>.bus`.
fn find_bus_file(install: &OmsiInstall, id: &str) -> Option<PathBuf> {
    let vehicles = install.find_file("Vehicles")?;
    let file = format!("{id}.bus");
    std::fs::read_dir(vehicles)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .find_map(|e| install.find_in(&e.path(), &file))
}

/// Parses `global.cfg`, picks the start tile and queues its 3×3 neighbourhood.
pub fn begin_world_load(config: OmsiSessionConfig) -> Result<WorldLoadJob, String> {
    let mut install = OmsiInstall::new(&config.root);
    if let Some(l) = &config.lookup {
        install = install.with_lookup(l.clone());
    }
    if let Some(l) = &config.name_lookup {
        install = install.with_name_lookup(l.clone());
    }
    if let Some(l) = &config.twin_lookup {
        install = install.with_twin_lookup(l.clone());
    }
    let cfg_path = install
        .find_in(&config.map_dir, "global.cfg")
        .ok_or_else(|| format!("global.cfg not found in {}", config.map_dir.display()))?;
    let bytes = std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let manifest =
        MapManifest::parse(&bytes).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    tracing::info!(
        path = %cfg_path.display(),
        name = %manifest.name,
        friendly = ?manifest.friendly_name,
        tiles = manifest.tiles.len(),
        entry_points = manifest.entry_points.len(),
        "global.cfg parsed"
    );

    let tile_files: HashMap<(i32, i32), String> = manifest
        .tiles
        .iter()
        .map(|t| ((t.x, t.y), t.file_name.clone()))
        .collect();

    let mut job = WorldLoadJob {
        install,
        cache: ContentCache::default(),
        config,
        manifest,
        tile_files,
        queue: VecDeque::new(),
        resident: HashMap::new(),
        stream_radius: 1,
        total_tiles: 0,
        sli_cache: HashMap::new(),
        spline_index: None,
        sco_cache: HashMap::new(),
        spline_materials: HashMap::new(),
        ground_material: None,
        ground_array: None,
        spawn: None,
        start_tile: (0, 0),
        camera: None,
        camera_std_index: 0,
    };

    // Start tile and spawn: the chosen `[entrypoints]` record if it resolves to a placed
    // `entrypoint_*.sco` object, otherwise the first road spline of the first usable tile.
    let mut keys: Vec<(i32, i32)> = job.manifest.tiles.iter().map(|t| (t.x, t.y)).collect();
    keys.dedup();
    let mut start = job.find_entry_spawn(&keys);
    if start.is_none() {
        tracing::warn!(
            entry = %job.config.entry_point,
            "entry point not resolvable; falling back to the first road spline"
        );
        for key in keys {
            let Some(tile) = job.read_tile(key) else {
                continue;
            };
            if let Some(spawn) = job.find_drivable_spawn(key, &tile) {
                start = Some((key, spawn));
                break;
            }
        }
    }
    let (start_tile, spawn) = start.ok_or("no entry point or drivable road spline found")?;
    tracing::info!(
        tile = ?start_tile,
        spawn_omsi = ?spawn.0,
        heading_deg = spawn.1,
        "start tile and spawn point selected"
    );

    // Floating origin: all scene coordinates are relative to the start tile's south-west
    // corner. Real maps use world coordinates in the millions of metres (Berlin-Spandau:
    // 3.4e6 north), where f32 resolution is a quarter of a metre.
    let abs = tile_origin(start_tile);
    job.spawn = Some((
        Vec3::new(spawn.0.x - abs.0, spawn.0.y - abs.1, spawn.0.z),
        spawn.1,
    ));
    job.start_tile = start_tile;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let key = (start_tile.0 + dx, start_tile.1 + dy);
            if job.tile_path(key).is_some() {
                job.queue.push_back(key);
            } else {
                tracing::debug!(tile = ?key, "neighbour tile not present in map");
            }
        }
    }
    job.total_tiles = job.queue.len();
    tracing::info!(
        tiles = job.total_tiles,
        "tiles discovered for the 3x3 neighbourhood"
    );
    Ok(job)
}

impl WorldLoadJob {
    /// Display name of the loaded map.
    #[must_use]
    pub fn map_name(&self) -> String {
        self.manifest
            .friendly_name
            .clone()
            .unwrap_or_else(|| self.manifest.name.clone())
    }

    /// A raised viewpoint over the spawn point, used when no vehicle could be loaded so the
    /// loaded world can still be inspected with the free-roam camera.
    #[must_use]
    pub fn inspection_camera(&self) -> Option<Transform> {
        let (pos, _) = self.spawn?;
        let target = omsi_to_bevy_pos(pos);
        Some(
            Transform::from_translation(target + Vec3::new(0.0, 25.0, 25.0))
                .looking_at(target, Vec3::Y),
        )
    }

    /// Tile the player starts on.
    #[must_use]
    pub fn start_tile(&self) -> (i32, i32) {
        self.start_tile
    }

    /// Index of every spline on the map, built by reading all tiles once.
    fn spline_index(&mut self) -> Arc<SplineIndex> {
        if let Some(i) = &self.spline_index {
            return i.clone();
        }
        let mut index = SplineIndex::default();
        for key in self.tile_files.keys() {
            let parsed = self
                .tile_path(*key)
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| MapTile::parse(&b).ok());
            if let Some(tile) = parsed {
                index.add_tile(*key, &tile.splines);
            }
        }
        let index = Arc::new(index);
        self.spline_index = Some(index.clone());
        index
    }

    fn tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        let name = self.tile_files.get(&key)?;
        self.install.find_in(&self.config.map_dir, name)
    }

    fn read_tile(&self, key: (i32, i32)) -> Option<MapTile> {
        let path = self.tile_path(key)?;
        let bytes = std::fs::read(&path).ok()?;
        match MapTile::parse(&bytes) {
            Ok(t) => Some(t),
            Err(e) => {
                tracing::error!(path = %path.display(), "tile parse failed: {e}");
                None
            }
        }
    }

    fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
        let path = self.install.find_file(rel);
        let Some(path) = path else {
            tracing::error!(sli = rel, "spline profile not found");
            return None;
        };
        if let Some(c) = self.sli_cache.get(&path) {
            return c.clone();
        }
        let parsed = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| SplineProfile::parse(&b).map_err(|e| e.to_string()));
        let out = match parsed {
            Ok(p) => {
                tracing::debug!(path = %path.display(), profiles = p.graphical_profiles.len(), paths = p.paths.len(), "loaded sli");
                Some(Arc::new(p))
            }
            Err(e) => {
                tracing::error!(path = %path.display(), "sli parse failed: {e}");
                None
            }
        };
        self.sli_cache.insert(path, out.clone());
        out
    }

    fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
        let Some(path) = self.install.find_file(rel) else {
            tracing::error!(sco = rel, "scenery object not found");
            return None;
        };
        if let Some(c) = self.sco_cache.get(&path) {
            return c.clone().map(|o| (path, o));
        }
        let parsed = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| SceneryObject::parse(&b).map_err(|e| e.to_string()));
        let out = match parsed {
            Ok(o) => {
                tracing::debug!(path = %path.display(), meshes = o.meshes.len(), "loaded sco");
                Some(Arc::new(o))
            }
            Err(e) => {
                tracing::error!(path = %path.display(), "sco parse failed: {e}");
                None
            }
        };
        self.sco_cache.insert(path.clone(), out.clone());
        out.map(|o| (path, o))
    }

    /// Resolves the configured entry point to `(tile, world position, heading)`.
    ///
    /// A `global.cfg` `[entrypoints]` record references an `entrypoint_*.sco` object by its id
    /// code (verified: all 51 Berlin-Spandau records match a placed object). The record's
    /// position is `(x, elevation, y)` in that object's tile; its heading is the object's.
    fn find_entry_spawn(&mut self, keys: &[(i32, i32)]) -> Option<((i32, i32), (Vec3, f32))> {
        let eps = &self.manifest.entry_points;
        // Developer aid: `OPTS_DEV_ENTRY=<substring>` picks the first entry point containing it.
        let dev = std::env::var("OPTS_DEV_ENTRY").ok();
        let ep = eps
            .iter()
            .find(|e| {
                dev.as_deref()
                    .map_or(e.name == self.config.entry_point, |d| e.name.contains(d))
            })
            .or_else(|| eps.first())?
            .clone();
        for &key in keys {
            let Some(tile) = self.read_tile(key) else {
                continue;
            };
            if let Some(obj) = tile
                .objects
                .iter()
                .find(|o| o.id_code == u64::from(ep.id_code))
            {
                let o = tile_origin(key);
                let pos = Vec3::new(o.0 + obj.position.x, o.1 + obj.position.y, ep.position.y);
                tracing::info!(entry = %ep.name, id_code = ep.id_code, sco = %obj.sco_file, tile = ?key, "entry point resolved");
                return Some((key, (pos, obj.heading_deg)));
            }
        }
        None
    }

    /// Finds the first road-vehicle path (type 0, forward or bidirectional) in a tile.
    fn find_drivable_spawn(&mut self, key: (i32, i32), tile: &MapTile) -> Option<(Vec3, f32)> {
        for inst in &tile.splines {
            let Some(profile) = self.sli(&inst.sli_file) else {
                continue;
            };
            let Some(path) = profile
                .paths
                .iter()
                .find(|p| p.path_type == 0 && p.direction != 1)
            else {
                continue;
            };
            let h = inst.heading_deg.to_radians();
            let origin = tile_origin(key);
            // Lateral offset along the spline's right vector.
            let x = origin.0 + inst.position.x + h.cos() * path.offset_x;
            let y = origin.1 + inst.position.y - h.sin() * path.offset_x;
            return Some((
                Vec3::new(x, y, inst.position.z + path.offset_z),
                inst.heading_deg,
            ));
        }
        None
    }
}

fn tile_origin(key: (i32, i32)) -> (f32, f32) {
    (key.0 as f32 * TILE_SIZE_M, key.1 as f32 * TILE_SIZE_M)
}

/// Layer textures of a map shared by all its tiles (one GPU array with mip chains).
struct GroundArray {
    layers: Handle<Image>,
    /// Repeats per tile edge of each array layer.
    repeats: Vec<f32>,
    /// Whether the layer's texture decoded; layers that did not are never blended in.
    usable: Vec<bool>,
}

/// Builds the layered ground material of a tile from its layer masks; `None` if the tile has
/// none (the caller then repeats the base texture) or the base layer cannot be read.
fn compose_tile_material(
    job: &mut WorldLoadJob,
    key: (i32, i32),
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    ground_materials: &mut Assets<GroundMaterial>,
) -> Option<Handle<GroundMaterial>> {
    use opts_bevy::ground::CpuImage;
    use opts_bevy::ground_material::{
        ground_extension, layer_array, mask_array, MAX_GROUND_LAYERS,
    };
    let file = job.tile_files.get(&key)?.clone();
    let layer_count = job.manifest.ground_textures.len().min(MAX_GROUND_LAYERS);
    let mut masks: Vec<Option<CpuImage>> = vec![None];
    for n in 1..layer_count {
        let mask = [
            format!("texture/map/{file}.{n}.dds"),
            format!("texture/{file}.{n}.dds"),
        ]
        .iter()
        .find_map(|rel| job.install.find_in(&job.config.map_dir, rel))
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| opts_bevy::load_bevy_image(&b).ok())
        .and_then(|img| CpuImage::from_image(&img));
        masks.push(mask);
    }
    if masks.iter().all(Option::is_none) {
        return None;
    }
    if job.ground_array.is_none() {
        // Decode every layer once per map and upload them as one mipmapped array.
        let dirs = vec![
            job.config.map_dir.clone(),
            job.config.map_dir.join("texture"),
        ];
        let mut decoded: Vec<Option<CpuImage>> = Vec::new();
        for n in 0..layer_count {
            let name = job.manifest.ground_textures[n].ground_texture.clone();
            let img = {
                let mut loader = ContentLoader {
                    install: &job.install,
                    cache: &mut job.cache,
                    meshes,
                    materials,
                    images,
                };
                loader.texture(&name, &dirs)
            }
            .and_then(|h| images.get(&h).and_then(CpuImage::from_image));
            if img.is_none() {
                tracing::warn!(layer = n, texture = %name, "ground layer texture cannot be decoded; layer skipped");
            }
            decoded.push(img);
        }
        let base = decoded.first().cloned().flatten()?;
        let size = decoded
            .iter()
            .flatten()
            .map(|d| d.width.max(d.height))
            .max()
            .unwrap_or(512)
            .clamp(256, 1024)
            .next_power_of_two();
        let refs: Vec<&CpuImage> = decoded
            .iter()
            .map(|d| d.as_ref().unwrap_or(&base))
            .collect();
        let layers = images.add(layer_array(&refs, size));
        job.ground_array = Some(GroundArray {
            layers,
            repeats: (0..layer_count)
                .map(|n| job.manifest.ground_textures[n].params[2].max(1.0))
                .collect(),
            usable: decoded.iter().map(Option::is_some).collect(),
        });
    }
    let array = job.ground_array.as_ref()?;
    if !array.usable.first().copied().unwrap_or(false) {
        return None;
    }
    let size = masks
        .iter()
        .flatten()
        .map(|m| m.width.max(m.height))
        .max()
        .unwrap_or(512)
        .clamp(256, 1024);
    let mask_refs: Vec<Option<&CpuImage>> = masks
        .iter()
        .enumerate()
        .map(|(n, m)| {
            m.as_ref()
                .filter(|_| array.usable.get(n).copied().unwrap_or(false))
        })
        .collect();
    let mask_handle = images.add(mask_array(&mask_refs, size));
    Some(ground_materials.add(GroundMaterial {
        base: terrain_material(None),
        extension: ground_extension(array.layers.clone(), mask_handle, &array.repeats),
    }))
}

/// Loads the next queued tile: terrain, splines and scenery objects.
pub fn load_next_tile(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    ground_materials: Option<&mut Assets<GroundMaterial>>,
) {
    let Some(key) = job.queue.pop_front() else {
        return;
    };
    if job.resident.contains_key(&key) {
        return;
    }
    let Some(tile) = job.read_tile(key) else {
        stats.tiles_missing += 1;
        return;
    };
    let mut children: Vec<Entity> = Vec::new();
    let origin = {
        let (a, b) = (tile_origin(key), tile_origin(job.start_tile));
        (a.0 - b.0, a.1 - b.1)
    };
    let tile_path = job.tile_path(key).unwrap_or_default();
    tracing::info!(
        tile = ?key,
        path = %tile_path.display(),
        objects = tile.objects.len(),
        splines = tile.splines.len(),
        "tile parsed"
    );

    // --- Terrain ---------------------------------------------------------------------------
    let terrain_rel = format!("{}.terrain", job.tile_files[&key]);
    let terrain: Option<MapTerrain> = job
        .install
        .find_in(&job.config.map_dir, &terrain_rel)
        .and_then(|p| match std::fs::read(&p).map(|b| MapTerrain::parse(&b)) {
            Ok(Ok(t)) => Some(t),
            Ok(Err(e)) => {
                tracing::error!(path = %p.display(), "terrain parse failed: {e}");
                None
            }
            Err(e) => {
                tracing::error!(path = %p.display(), "terrain read failed: {e}");
                None
            }
        });

    if let Some(t) = &terrain {
        if job.ground_material.is_none() {
            let tex = job
                .manifest
                .ground_textures
                .first()
                .map(|g| g.ground_texture.clone());
            let dirs = vec![
                job.config.map_dir.clone(),
                job.config.map_dir.join("texture"),
            ];
            let handle = tex.and_then(|name| {
                let mut loader = ContentLoader {
                    install: &job.install,
                    cache: &mut job.cache,
                    meshes,
                    materials,
                    images,
                };
                loader.texture(&name, &dirs)
            });
            job.ground_material = Some(materials.add(terrain_material(handle)));
        }
        // Ground painting: per-layer alpha masks of this tile (road markings, pavements, paths).
        let tile_material = ground_materials
            .and_then(|gm| compose_tile_material(job, key, images, materials, meshes, gm));
        let mesh = build_terrain_mesh(t, origin, tile_material.is_some());
        stats.scene_vertices += mesh.count_vertices();
        let name = Name::new(format!("Terrain_{}_{}", key.0, key.1));
        children.push(match tile_material {
            Some(material) => commands
                .spawn((
                    MaterialMeshBundle {
                        mesh: meshes.add(mesh),
                        material,
                        ..default()
                    },
                    name,
                ))
                .id(),
            None => commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: job.ground_material.clone().unwrap_or_default(),
                        ..default()
                    },
                    name,
                ))
                .id(),
        });
    } else {
        tracing::warn!(tile = ?key, "tile has no readable .terrain grid; objects use z as absolute height");
    }

    // --- Splines ---------------------------------------------------------------------------
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            stats.splines_failed += 1;
            continue;
        };
        if profile.only_editor {
            continue;
        }
        let spline_meshes = extrude_spline_segment(&profile, inst, origin);
        if spline_meshes.is_empty() {
            // Profiles without a graphical part (path-only helpers) have nothing to draw.
            tracing::debug!(sli = %inst.sli_file, "spline has no graphical profile");
            continue;
        }
        let sli_key = job.install.find_file(&inst.sli_file).unwrap_or_default();
        // Spline textures live in the profile's own pack folder (`Splines/<Pack>/texture`).
        let sli_dir = sli_key.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            sli_dir.join("texture"),
            sli_dir,
            job.config.map_dir.join("texture"),
        ];
        for (tex_idx, mesh) in spline_meshes {
            let mat = if let Some(m) = job.spline_materials.get(&(sli_key.clone(), tex_idx)) {
                m.clone()
            } else {
                let tex = profile.textures.get(tex_idx);
                let handle = tex.and_then(|t| {
                    let mut loader = ContentLoader {
                        install: &job.install,
                        cache: &mut job.cache,
                        meshes,
                        materials,
                        images,
                    };
                    loader.texture(&t.file, &tex_dirs)
                });
                let alpha = tex.is_some_and(|t| t.alpha_mode.is_some());
                let m = materials.add(StandardMaterial {
                    base_color_texture: handle,
                    perceptual_roughness: 0.9,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: if alpha {
                        AlphaMode::Mask(0.5)
                    } else {
                        AlphaMode::Opaque
                    },
                    // Road surfaces share their plane with the terrain deformed to them.
                    depth_bias: 4.0,
                    ..default()
                });
                job.spline_materials
                    .insert((sli_key.clone(), tex_idx), m.clone());
                m
            };
            stats.scene_vertices += mesh.count_vertices();
            children.push(
                commands
                    .spawn((
                        PbrBundle {
                            mesh: meshes.add(mesh),
                            material: mat,
                            ..default()
                        },
                        Name::new(format!("Spline_{}_{}", inst.id_code, tex_idx)),
                    ))
                    .id(),
            );
        }
        stats.splines_loaded += 1;
    }

    // --- Scenery objects -------------------------------------------------------------------
    // Spline attachments become ordinary placements once resolved against their carrier spline.
    let attached = if tile.spline_attachments.is_empty() {
        Vec::new()
    } else {
        let index = job.spline_index();
        expand_attachments(
            &tile.spline_attachments,
            &tile.splines,
            &index,
            key,
            |x, y| terrain.as_ref().map_or(0.0, |t| sample_terrain(t, x, y)),
        )
    };
    let placements = tile
        .objects
        .iter()
        .map(|o| (o, None))
        .chain(attached.iter().map(|(o, z)| (o, Some(*z))));
    for (obj, attached_z) in placements {
        let Some((sco_path, sco)) = job.sco(&obj.sco_file) else {
            stats.objects_failed += 1;
            continue;
        };
        if let Some(tree) = &sco.tree {
            children.extend(spawn_tree(
                job,
                stats,
                commands,
                meshes,
                materials,
                images,
                tree,
                obj,
                origin,
                terrain.as_ref(),
            ));
            continue;
        }
        if sco.only_editor {
            tracing::debug!(sco = %sco_path.display(), "editor-only object skipped");
            continue;
        }
        let sco_dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
        let model_dir = job
            .install
            .find_in(&sco_dir, "model")
            .unwrap_or_else(|| sco_dir.join("model"));
        let tex_dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];

        // `[absheight]` objects (crossings, bridges, ...) store an absolute elevation.
        let terrain_h = if sco.abs_height {
            0.0
        } else {
            terrain
                .as_ref()
                .map_or(0.0, |t| sample_terrain(t, obj.position.x, obj.position.y))
        };
        let z = match (attached_z, sco.abs_height) {
            (Some(z), true) => z,
            _ => terrain_h + obj.position.z,
        };
        let world = Vec3::new(origin.0 + obj.position.x, origin.1 + obj.position.y, z);
        let transform = Transform {
            translation: omsi_to_bevy_pos(world),
            // OMSI headings are clockwise-positive (compass-style); the helper is CCW.
            rotation: omsi_euler_to_bevy_quat(
                -obj.heading_deg,
                obj.pitch_deg.unwrap_or(0.0),
                obj.roll_deg.unwrap_or(0.0),
            ),
            scale: Vec3::ONE,
        };

        tracing::debug!(
            sco = %sco_path.display(),
            tile = ?key,
            pos = ?obj.position,
            terrain_h,
            heading = obj.heading_deg,
            pitch = ?obj.pitch_deg,
            roll = ?obj.roll_deg,
            "placing scenery object"
        );
        let protected_before = job.cache.stats.protected_meshes;
        let mut parts = Vec::new();
        for mesh_def in &sco.meshes {
            let Some(o3d_path) = job.install.find_in(&model_dir, &mesh_def.file) else {
                job.cache
                    .stats
                    .failures
                    .push(format!("o3d not found: {}", mesh_def.file));
                tracing::error!(sco = %sco_path.display(), mesh = %mesh_def.file, "o3d not found");
                continue;
            };
            let overrides: Vec<TextureOverride> = mesh_def
                .materials
                .iter()
                .map(|m| TextureOverride {
                    slot: m.material_index,
                    texture: m.texture.clone(),
                    alpha: m.alpha_mode,
                    hidden_by_variable: m.alpha_scale.is_some(),
                    trans_map: None,
                })
                .collect();
            let mut loader = ContentLoader {
                install: &job.install,
                cache: &mut job.cache,
                meshes,
                materials,
                images,
            };
            parts.extend(loader.o3d_parts(&o3d_path, &tex_dirs, &overrides));
        }
        if parts.is_empty() {
            if sco.meshes.is_empty() {
                tracing::debug!(sco = %sco_path.display(), "object has no mesh (helper/path object)");
                stats.objects_no_geometry += 1;
            } else if job.cache.stats.protected_meshes > protected_before {
                stats.objects_protected += 1;
                tracing::warn!(sco = %sco_path.display(), tile = ?key, pos = ?obj.position, "object skipped: all meshes are scrambled");
            } else {
                tracing::error!(sco = %sco_path.display(), "object has meshes but none could be loaded");
                stats.objects_failed += 1;
            }
            continue;
        }

        let name = sco
            .friendly_name
            .clone()
            .unwrap_or_else(|| obj.sco_file.clone());
        let parent = commands
            .spawn((SpatialBundle::from_transform(transform), Name::new(name)))
            .id();
        children.push(parent);
        stats.scene_vertices += parts.iter().map(|p| p.vertices).sum::<usize>();
        attach_model_parts(commands, parent, &parts, "Mesh");
        stats.objects_loaded += 1;
    }

    let tile_root = commands
        .spawn((
            SpatialBundle::default(),
            InGameEntity,
            TileRoot(key),
            Name::new(format!("Tile_{}_{}", key.0, key.1)),
        ))
        .id();
    commands.entity(tile_root).push_children(&children);
    job.resident.insert(key, tile_root);
    stats.tiles_loaded = job.resident.len();
    stats.asset_failures = job.cache.stats.failures.len();
}

/// Marks the root entity of a streamed tile.
#[derive(Component, Debug, Clone, Copy)]
pub struct TileRoot(pub (i32, i32));

/// Keeps the tiles around the camera loaded and unloads distant ones (one tile load per frame).
///
/// The observer is the 3D camera; tiles within `stream_radius` (Chebyshev distance) are
/// resident, tiles beyond `stream_radius + 1` are despawned (hysteresis avoids thrashing at
/// borders). Parsed objects and textures stay cached, so revisiting a tile is cheap.
#[allow(clippy::too_many_arguments)]
pub fn stream_world_tiles(
    job: Option<ResMut<WorldLoadJob>>,
    state: Res<crate::in_game::types::SimulationLoadState>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut stats: ResMut<WorldStats>,
    mut commands: Commands,
    mut assets: LoadAssets,
) {
    let Some(mut job) = job else { return };
    if !state.is_complete {
        return;
    }
    let Ok(cam) = camera.get_single() else { return };
    let p = cam.translation();
    let centre = (
        job.start_tile.0 + (p.x / TILE_SIZE_M).floor() as i32,
        job.start_tile.1 + ((-p.z) / TILE_SIZE_M).floor() as i32,
    );
    let dist = |k: (i32, i32)| (k.0 - centre.0).abs().max((k.1 - centre.1).abs());

    let radius = job.stream_radius;
    let far: Vec<(i32, i32)> = job
        .resident
        .keys()
        .copied()
        .filter(|k| dist(*k) > radius + 1)
        .collect();
    for key in far {
        if let Some(root) = job.resident.remove(&key) {
            commands.entity(root).despawn_recursive();
            tracing::debug!(tile = ?key, "tile unloaded");
        }
    }
    stats.tiles_loaded = job.resident.len();

    if job.queue.is_empty() {
        let mut wanted: Vec<(i32, i32)> = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let k = (centre.0 + dx, centre.1 + dy);
                if !job.resident.contains_key(&k) && job.tile_path(k).is_some() {
                    wanted.push(k);
                }
            }
        }
        wanted.sort_by_key(|k| dist(*k));
        if let Some(k) = wanted.first() {
            job.queue.push_back(*k);
        }
    }
    if !job.queue.is_empty() {
        let (meshes, materials, images) = (
            &mut assets.meshes,
            &mut assets.materials,
            &mut assets.images,
        );
        load_next_tile(
            &mut job,
            &mut stats,
            &mut commands,
            meshes,
            materials,
            images,
            assets.ground_materials.as_deref_mut(),
        );
    }
}

// ============================================================================
// Vehicle
// ============================================================================

/// Reads the first vehicle listed in the map's `ailists.cfg` (fallback when no `--vehicle-path`).
fn first_ai_list_vehicle(job: &WorldLoadJob) -> Option<PathBuf> {
    let cfg = job.install.find_in(&job.config.map_dir, "ailists.cfg")?;
    let text = opts_core::formats::text::decode_bytes(&std::fs::read(cfg).ok()?);
    text.lines()
        .map(str::trim)
        .filter(|l| l.to_lowercase().contains(".bus"))
        .find_map(|l| {
            let rel = l.split('\t').next()?.trim();
            job.install.find_file(rel)
        })
}

/// Loads the visible lod0 meshes of one vehicle segment as children of `root`.
/// Returns `(mesh_count, vertex_count, bounds_min, bounds_max)` (Bevy space, body meshes only).
#[allow(clippy::too_many_arguments)]
fn load_segment_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    model_dir: &Path,
    tex_dirs: &[PathBuf],
) -> (usize, usize, Vec3, Vec3) {
    let mut mesh_count = 0usize;
    let mut vertex_count = 0usize;
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for mesh_def in &lod0.meshes {
        // Shadow-only and AI-only meshes are not part of the visible player model.
        let vp = mesh_def.viewpoint;
        if mesh_def.isshadow || !(vp.is_outside() || vp.is_inside()) {
            continue;
        }
        // Developer aid: `OPTS_HIDE_MESH=<substring>[,<substring>]` skips matching vehicle meshes
        // (used to bisect rendering problems).
        if std::env::var("OPTS_HIDE_MESH").is_ok_and(|v| {
            v.split(',')
                .filter(|t| !t.is_empty())
                .any(|t| mesh_def.path.to_lowercase().contains(&t.to_lowercase()))
        }) {
            continue;
        }
        // Static pose: every animation/visibility variable is at its default of 0.
        if mesh_def.visibility.iter().any(|v| v.expected_value != 0.0) {
            tracing::debug!(mesh = %mesh_def.path, visibility = ?mesh_def.visibility, "vehicle mesh hidden in default state");
            continue;
        }
        let Some(o3d_path) = job.install.find_in(model_dir, &mesh_def.path) else {
            tracing::error!(mesh = %mesh_def.path, "vehicle o3d not found");
            job.cache
                .stats
                .failures
                .push(format!("vehicle o3d not found: {}", mesh_def.path));
            continue;
        };
        let overrides: Vec<TextureOverride> = mesh_def
            .materials
            .iter()
            .map(|m| TextureOverride {
                slot: m.slot_index as usize,
                texture: m.texture.clone(),
                alpha: m.alpha_mode,
                hidden_by_variable: m.alpha_scale_var.is_some(),
                trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
            })
            .collect();
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        let parts = loader.o3d_parts(&o3d_path, tex_dirs, &overrides);
        for part in &parts {
            if let Some(mesh) = meshes.get(&part.mesh) {
                if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                    for p in pos {
                        lo = lo.min(Vec3::from(*p));
                        hi = hi.max(Vec3::from(*p));
                    }
                    // Camera framing uses the vehicle body only: meshes that are huge or far
                    // from the origin (shadow volumes, sky/helper geometry) would skew it.
                    if (hi - lo).max_element() < 16.0 && ((hi + lo) * 0.5).abs().max_element() < 8.0
                    {
                        min = min.min(lo);
                        max = max.max(hi);
                    }
                }
            }
            vertex_count += part.vertices;
            mesh_count += 1;
        }
        attach_model_parts(commands, root, &parts, &mesh_def.path);
    }
    (mesh_count, vertex_count, min, max)
}

/// Spawns the trailers named by `[couple_back]` (recursively) behind `root`. Returns the
/// passenger camera presets of all trailers with the segment entity they are defined in.
#[allow(clippy::too_many_arguments)]
fn spawn_trailer_chain(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    bus_dir: &Path,
    tractor: &BusManifest,
) -> Vec<(Entity, opts_core::formats::vehicles::bus::BusCamera)> {
    const MAX_SEGMENTS: usize = 4;
    let mut cams = Vec::new();
    let (mut parent, mut parent_dir, mut parent_manifest) =
        (root, bus_dir.to_path_buf(), tractor.clone());
    for _ in 0..MAX_SEGMENTS {
        let Some(name) = parent_manifest.couple_back.clone() else {
            break;
        };
        let Some(path) = job.install.find_in(&parent_dir, &name) else {
            tracing::error!(trailer = %name, "trailer .bus not found; vehicle stays without it");
            break;
        };
        let loaded = (|| -> Result<_, String> {
            let manifest =
                BusManifest::parse_bytes(&std::fs::read(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
            let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
            let cfg = job
                .install
                .find_in(&dir, &cfg_rel)
                .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
            let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok((manifest, dir, cfg, model))
        })();
        let (manifest, dir, cfg, model) = match loaded {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(trailer = %path.display(), "cannot load trailer: {e}");
                break;
            }
        };
        let Some(lod0) = model.lods.first() else {
            break;
        };
        // Both coupling points are in their own vehicle frame; the trailer origin sits where
        // its front joint meets the parent's rear joint.
        let offset = match (parent_manifest.coupling_back, manifest.coupling_front) {
            (Some(b), Some(f)) => omsi_to_bevy_pos(b - f),
            _ => {
                tracing::error!(trailer = %path.display(), "coupling points missing; trailer not attached");
                break;
            }
        };
        let seg = commands
            .spawn((
                Name::new(name.clone()),
                SpatialBundle::from_transform(Transform::from_translation(offset)),
                InGameEntity,
            ))
            .id();
        commands.entity(parent).add_child(seg);
        let model_dir = cfg.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            job.install
                .find_in(&dir, "texture")
                .unwrap_or_else(|| dir.join("texture")),
            model_dir.join("texture"),
        ];
        let (count, verts, _, _) = load_segment_meshes(
            job, commands, meshes, materials, images, seg, lod0, &model_dir, &tex_dirs,
        );
        tracing::info!(trailer = %name, meshes = count, vertices = verts, "trailer segment attached");
        cams.extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
        (parent, parent_dir, parent_manifest) = (seg, dir, manifest);
    }
    cams
}

/// Loads the player's vehicle from its `.bus` file and spawns it at the map's start point.
pub fn load_player_vehicle(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::render::mesh::skinning::SkinnedMeshInverseBindposes>,
) -> Result<(), String> {
    let bus_path = match job.config.vehicle_path.clone() {
        Some(p) => p,
        None => first_ai_list_vehicle(job)
            .ok_or("no --vehicle-path given and the map's ailists.cfg lists no vehicle")?,
    };
    let bus_bytes = std::fs::read(&bus_path).map_err(|e| format!("{}: {e}", bus_path.display()))?;
    let manifest =
        BusManifest::parse_bytes(&bus_bytes).map_err(|e| format!("{}: {e}", bus_path.display()))?;
    tracing::info!(path = %bus_path.display(), mass_t = ?manifest.mass_tonnes, axles = manifest.axles.len(), "vehicle .bus parsed");

    let bus_dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let cfg_rel = manifest
        .model_cfg
        .clone()
        .ok_or("the .bus file has no [model] entry")?;
    let cfg_path = job
        .install
        .find_in(&bus_dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let cfg_bytes = std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let model =
        ModelConfig::parse_bytes(&cfg_bytes).map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let lod0 = model.lods.first().ok_or("model config has no meshes")?;
    tracing::info!(path = %cfg_path.display(), lods = model.lods.len(), meshes = lod0.meshes.len(), "vehicle model config parsed");

    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        job.install
            .find_in(&bus_dir, "texture")
            .unwrap_or_else(|| bus_dir.join("texture")),
        model_dir.join("texture"),
    ];

    // Spawn point: the start road spline's lane position, converted to Bevy space.
    let (spawn_world, heading_deg) = job.spawn.ok_or("no spawn point")?;
    let bevy_pos = omsi_to_bevy_pos(spawn_world);
    let root_tf = Transform {
        translation: bevy_pos,
        rotation: omsi_euler_to_bevy_quat(-heading_deg, 0.0, 0.0),
        scale: Vec3::ONE,
    };

    let root = spawn_vehicle_model(commands, &manifest, &model, root_tf, &[], &[]);

    let (mesh_count, vertex_count, min, max) = load_segment_meshes(
        job, commands, meshes, materials, images, root, lod0, &model_dir, &tex_dirs,
    );
    if mesh_count == 0 {
        return Err("no vehicle meshes could be loaded".into());
    }

    let mut controller = VehicleController::from_manifest(&manifest);
    controller.sim.chassis.position = Vec2::new(spawn_world.x, spawn_world.y);
    controller.sim.chassis.elevation_z = spawn_world.z;
    // The simulation uses compass-style heading (clockwise from +Y), like OMSI map headings.
    controller.sim.chassis.heading_rad = heading_deg.to_radians();
    commands
        .entity(root)
        .insert((controller, PlayerVehicle, InGameEntity));
    let trailer_cams = spawn_trailer_chain(
        job, commands, meshes, materials, images, root, &bus_dir, &manifest,
    );
    if !trailer_cams.is_empty() {
        commands.add(move |world: &mut World| {
            if let Some(mut bus) = world.get_mut::<opts_bevy::BusVehicle>(root) {
                bus.trailer_pax_cameras = trailer_cams;
            }
        });
    }
    match load_driver_human(
        job,
        commands,
        meshes,
        materials,
        images,
        bindposes,
        root,
        &bus_dir,
        manifest.passengercabin_cfg.as_deref(),
    ) {
        Ok(name) => tracing::info!(human = %name, "player character seated at the driver position"),
        Err(e) => tracing::error!("cannot load the player character: {e}"),
    }

    tracing::debug!(min = ?min, max = ?max, "vehicle bounding box (Bevy space)");
    let length = (max.z - min.z).abs().max(1.0);
    let height = (max.y - min.y).abs().max(1.0);
    let driver_cam = manifest
        .driver_cameras
        .get(manifest.std_camera_index.unwrap_or(0))
        .or_else(|| manifest.driver_cameras.first());
    // Driver camera preset converted by the shared helper (vehicle space → Bevy).
    let cockpit_offset = driver_cam
        .map(|c| {
            camera_preset_to_bevy_transform(
                Vec3::new(c.x, c.y, c.z),
                c.dist,
                c.yaw_deg,
                c.pitch_deg,
            )
            .translation
        })
        .unwrap_or(Vec3::new(0.0, height * 0.8, 0.0));
    let chase_distance = length * 1.3;
    let chase_height = height * 1.6;
    job.camera_std_index = manifest.std_camera_index.unwrap_or(0);
    job.camera = Some(CameraSetup {
        start: Transform::from_translation(
            bevy_pos + root_tf.rotation * Vec3::new(0.0, chase_height, chase_distance),
        )
        .looking_at(bevy_pos + Vec3::Y * (height * 0.5), Vec3::Y),
        chase_distance,
        chase_height,
        cockpit_offset,
    });

    stats.vehicle_name = manifest
        .friendly_name
        .as_ref()
        .map(|f| format!("{} {}", f.manufacturer, f.model))
        .unwrap_or_else(|| {
            bus_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
    stats.vehicle_meshes = mesh_count;
    stats.vehicle_vertices = vertex_count;
    stats.scene_vertices += vertex_count;
    stats.asset_failures = job.cache.stats.failures.len();
    tracing::info!(
        vehicle = %stats.vehicle_name,
        meshes = mesh_count,
        vertices = vertex_count,
        "vehicle meshes loaded"
    );
    Ok(())
}

// ============================================================================
// Diagnostic overlay
// ============================================================================

/// Marker for the toggleable diagnostic overlay text.
#[derive(Component)]
pub struct DebugOverlayText;

/// Spawns the (initially visible) diagnostic overlay.
pub fn spawn_debug_overlay(commands: &mut Commands) {
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 14.0,
                color: Color::srgb(0.85, 1.0, 0.85),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(70.0),
            left: Val::Px(12.0),
            ..default()
        }),
        DebugOverlayText,
        InGameEntity,
    ));
}

/// Refreshes the overlay text; F3 toggles visibility.
pub fn update_debug_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<WorldStats>,
    mut query: Query<(&mut Text, &mut Visibility), With<DebugOverlayText>>,
) {
    if keyboard.just_pressed(KeyCode::F10) {
        stats.overlay_visible = !stats.overlay_visible;
    }
    for (mut text, mut vis) in &mut query {
        *vis = if stats.overlay_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        text.sections[0].value = format!(
            "[F10] map: {}\ntile: {:?} | tiles loaded: {} (missing {})\nscenery objects: {} (trees {}, protected-skipped {}, failed {}, no mesh {}) | splines: {} (failed {})\nvehicle: {} ({} meshes, {} verts)\nscene vertices: {} | asset failures: {}",
            stats.map_name,
            stats.start_tile,
            stats.tiles_loaded,
            stats.tiles_missing,
            stats.objects_loaded,
            stats.trees_loaded,
            stats.objects_protected,
            stats.objects_failed,
            stats.objects_no_geometry,
            stats.splines_loaded,
            stats.splines_failed,
            if stats.vehicle_name.is_empty() { "N/A" } else { &stats.vehicle_name },
            stats.vehicle_meshes,
            stats.vehicle_vertices,
            stats.scene_vertices,
            stats.asset_failures,
        );
    }
}

/// Spawns a `[tree]` billboard. The instance's string variables carry
/// `[texture, height, width/height ratio]`; the `.sco` ranges are the fallback.
#[allow(clippy::too_many_arguments)]
fn spawn_tree(
    job: &mut WorldLoadJob,
    stats: &mut WorldStats,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tree: &ScoTree,
    obj: &MapObjectInstance,
    origin: (f32, f32),
    terrain: Option<&MapTerrain>,
) -> Option<Entity> {
    let var = |i: usize| {
        obj.string_vars
            .get(i)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    };
    let texture = var(0).unwrap_or(&tree.texture).to_string();
    let height = var(1)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or((tree.min_height + tree.max_height) * 0.5);
    let ratio = var(2)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or((tree.min_width_ratio + tree.max_width_ratio) * 0.5);

    let sco_dir = job
        .install
        .find_file(&obj.sco_file)
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let dirs = vec![sco_dir.join("texture"), job.config.map_dir.join("texture")];
    let handle = {
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        loader.texture(&texture, &dirs)
    };
    let Some(handle) = handle else {
        stats.objects_failed += 1;
        return None;
    };

    let ground = terrain.map_or(0.0, |t| sample_terrain(t, obj.position.x, obj.position.y));
    let world = Vec3::new(
        origin.0 + obj.position.x,
        origin.1 + obj.position.y,
        ground + obj.position.z,
    );
    let mesh = tree_billboard_mesh(height, height * ratio);
    stats.scene_vertices += mesh.count_vertices();
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(handle),
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 1.0,
        ..default()
    });
    let id = commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(mesh),
                material,
                transform: Transform::from_translation(omsi_to_bevy_pos(world))
                    .with_rotation(omsi_euler_to_bevy_quat(-obj.heading_deg, 0.0, 0.0)),
                ..default()
            },
            Name::new(format!("Tree_{}", obj.id_code)),
        ))
        .id();
    stats.trees_loaded += 1;
    stats.objects_loaded += 1;
    Some(id)
}

/// Loads the player's character and seats it at the vehicle's `[drivpos]`.
///
/// The skinned mesh, joint pivots and head anchor come from the `.hum`, its model manifest and
/// the `.o3d` bone weights; the seat contact point comes from the vehicle's passenger cabin.
#[allow(clippy::too_many_arguments)]
fn load_driver_human(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<bevy::render::mesh::skinning::SkinnedMeshInverseBindposes>,
    vehicle_root: Entity,
    bus_dir: &Path,
    cabin_rel: Option<&str>,
) -> Result<String, String> {
    use opts_bevy::{hip_offset, spawn_human, HumanPose};
    use opts_core::formats::humans::HumanProfile;
    use opts_core::formats::text::{decode_bytes, scan_blocks};

    let hum_path = job
        .config
        .human_path
        .clone()
        .ok_or("no .hum profile in the installation")?;
    let profile = HumanProfile::parse(
        &std::fs::read(&hum_path).map_err(|e| format!("{}: {e}", hum_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", hum_path.display()))?;
    let hum_dir = hum_path.parent().map(Path::to_path_buf).unwrap_or_default();

    // Driver seat: `[drivpos]` x, y, z, h, rot in the vehicle's passenger cabin.
    let cabin = cabin_rel
        .and_then(|c| job.install.find_in(bus_dir, c))
        .ok_or("vehicle has no passenger cabin file")?;
    let text =
        decode_bytes(&std::fs::read(&cabin).map_err(|e| format!("{}: {e}", cabin.display()))?);
    let blocks = scan_blocks(&text);
    let seat: Vec<f32> = blocks
        .iter()
        .find(|b| b.keyword == "drivpos")
        .ok_or("passenger cabin has no [drivpos]")?
        .parameters
        .iter()
        .take(5)
        .map(|p| {
            p.trim()
                .replace(',', ".")
                .parse::<f32>()
                .map_err(|_| format!("bad [drivpos] value '{p}'"))
        })
        .collect::<Result<_, _>>()?;
    if seat.len() < 5 {
        return Err("[drivpos] needs 5 values".into());
    }

    // Model manifest and skinned meshes (highest-detail LOD).
    let cfg_path = job
        .install
        .find_in(&hum_dir, &profile.model)
        .ok_or_else(|| format!("human model config not found: {}", profile.model))?;
    let cfg = opts_core::formats::vehicles::model::ModelConfig::parse_bytes(
        &std::fs::read(&cfg_path).map_err(|e| format!("{}: {e}", cfg_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", cfg_path.display()))?;
    let model_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let tex_dirs = vec![
        model_dir.clone(),
        model_dir.join("texture"),
        hum_dir.join("texture"),
        model_dir
            .parent()
            .map(|p| p.join("Texture"))
            .unwrap_or_default(),
    ];
    let lod = cfg.lods.first().ok_or("human model has no meshes")?;
    let mut parts = Vec::new();
    for mesh_def in &lod.meshes {
        let Some(o3d_path) = job.install.find_in(&model_dir, &mesh_def.path) else {
            tracing::error!(mesh = %mesh_def.path, "human mesh not found");
            continue;
        };
        let overrides: Vec<TextureOverride> = mesh_def
            .materials
            .iter()
            .map(|m| TextureOverride {
                slot: m.slot_index as usize,
                texture: m.texture.clone(),
                alpha: m.alpha_mode,
                hidden_by_variable: m.alpha_scale_var.is_some(),
                trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
            })
            .collect();
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        parts.extend(loader.o3d_skinned_parts(&o3d_path, &tex_dirs, &overrides));
    }
    if parts.is_empty() {
        return Err("no human meshes could be loaded".into());
    }

    tracing::info!(parts = parts.len(), vertices = parts.iter().map(|p| p.vertices).sum::<usize>(), seat = ?seat, "human meshes loaded");
    let name = hum_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let human = spawn_human(
        commands,
        bindposes,
        &profile,
        &parts,
        &name,
        HumanPose::Seated,
    );
    // Hips on the seat contact point, facing along `rot` (clockwise like headings).
    let contact = omsi_to_bevy_pos(Vec3::new(seat[0], seat[1], seat[2]));
    commands.entity(human).insert(Transform {
        translation: contact - hip_offset(&profile),
        rotation: omsi_euler_to_bevy_quat(-seat[4], 0.0, 0.0),
        scale: Vec3::ONE,
    });
    commands.entity(vehicle_root).add_child(human);
    Ok(name)
}
