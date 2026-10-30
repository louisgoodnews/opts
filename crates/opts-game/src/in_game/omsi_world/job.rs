//! Session resolution and the `WorldLoadJob` implementation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapObjectInstance, MapSplineInstance, MapTile};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

use super::vehicle::FormationCar;
use crate::catalogue_service::CatalogueService;
use crate::in_game::attachments::SplineIndex;
use crate::new_game_screen::types::GameCatalogue;
use crate::omsi_root::locate_omsi_root;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{OmsiSessionConfig, SessionRequest, WorldLoadJob};

impl WorldLoadJob {
    /// The materials of the road (spline) surfaces built so far; the weather darkens and
    /// whitens them when wet or snowy (`in_game::weather::surfaces`).
    pub fn spline_material_handles(&self) -> impl Iterator<Item = &Handle<StandardMaterial>> {
        self.spline_materials.values()
    }

    /// Root folder of the OMSI installation the session reads from.
    #[must_use]
    pub fn omsi_root(&self) -> &std::path::Path {
        self.install.root()
    }

    /// Fraction of tiles already processed (the tile in progress counts by how far it is).
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.total_tiles == 0 {
            1.0
        } else {
            let in_progress = self.building.as_ref().map_or(0.0, |b| 1.0 - b.fraction());
            let left = self.queue.len() as f32 + in_progress;
            (1.0 - left / self.total_tiles as f32).clamp(0.0, 1.0)
        }
    }

    /// Whether all queued tiles have been loaded (and none is in progress).
    #[must_use]
    pub fn tiles_done(&self) -> bool {
        self.queue.is_empty() && self.building.is_none()
    }

    /// Whether there is a tile to work on: one queued or one in progress.
    #[must_use]
    pub fn has_tile_work(&self) -> bool {
        !self.queue.is_empty() || self.building.is_some()
    }

    /// Tiles loaded so far and tiles asked for (the 3x3 neighbourhood), for the loading screen.
    #[must_use]
    pub fn tile_counts(&self) -> (usize, usize) {
        (
            self.total_tiles
                .saturating_sub(self.queue.len() + usize::from(self.building.is_some())),
            self.total_tiles,
        )
    }

    /// The first `limit` files that could not be read, for the loading report.
    #[must_use]
    pub fn failed_files(&self, limit: usize) -> Vec<String> {
        self.cache
            .stats
            .failures
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Tiles around the start that the map lists but whose file is missing.
    #[must_use]
    pub fn tiles_listed_missing(&self) -> usize {
        self.missing_listed
    }

    /// Whether the session names a vehicle file (otherwise the map's AI list supplies one).
    #[must_use]
    pub fn has_vehicle_file(&self) -> bool {
        self.config.vehicle_path.is_some()
    }

    /// Next tile to load, for status messages.
    #[must_use]
    pub fn next_tile(&self) -> Option<(i32, i32)> {
        self.queue.front().copied()
    }
}

mod resolve;
pub use resolve::resolve_session_config;
// (no unused resolve imports)

/// Parses `global.cfg`, picks the start tile and queues its 3×3 neighbourhood (the base map,
/// without Chrono).
pub fn begin_world_load(config: OmsiSessionConfig) -> Result<WorldLoadJob, String> {
    begin_world_load_at(config, None)
}

/// The installation with the catalogue's lookups, when the session has them.
fn install_of(config: &OmsiSessionConfig) -> OmsiInstall {
    let mut install = OmsiInstall::new(&config.root);
    if let Some(l) = &config.lookup {
        install = install.with_lookup(l.clone());
    }
    if let Some(l) = &config.name_lookup {
        install = install.with_name_lookup(l.clone());
    }
    if let Some(l) = &config.relocate_lookup {
        install = install.with_relocate_lookup(l.clone());
    }
    if let Some(l) = &config.twin_lookup {
        install = install.with_twin_lookup(l.clone());
    }
    install
}

/// Reads and parses the map's `global.cfg`.
fn read_manifest(install: &OmsiInstall, config: &OmsiSessionConfig) -> Result<MapManifest, String> {
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
    Ok(manifest)
}

/// A job with nothing loaded yet.
fn new_job(
    install: OmsiInstall,
    config: OmsiSessionConfig,
    manifest: MapManifest,
    tile_files: HashMap<(i32, i32), String>,
) -> WorldLoadJob {
    WorldLoadJob {
        install,
        cache: ContentCache::default(),
        config,
        manifest,
        tile_files,
        queue: VecDeque::new(),
        tile_water: HashMap::new(),
        water_material: None,
        water_enabled: true,
        resident: HashMap::new(),
        tile_mode: crate::in_game::tile_loading::TileMode::default(),
        missing_tiles: HashSet::new(),
        building: None,
        stream_budget: crate::in_game::tile_loading::DEFAULT_STREAM_BUDGET,
        total_tiles: 0,
        sli_cache: HashMap::new(),
        tile_deps: HashMap::new(),
        loading_deps: HashSet::new(),
        vehicle_dirs: Vec::new(),
        spline_index: None,
        tile_splines: HashMap::new(),
        tile_aligned: HashMap::new(),
        placed: HashMap::new(),
        tile_ground: None,
        sco_cache: HashMap::new(),
        sound_cfgs: HashMap::new(),
        crossing_bases: HashMap::new(),
        ground_faces: HashMap::new(),
        tile_objects: HashMap::new(),
        tile_surfaces: HashMap::new(),
        timetables: None,
        collision_bounds: HashMap::new(),
        scenery: super::scenery_objects::SceneryObjectCache::new(),
        parklists: HashMap::new(),
        chrono: super::chrono::ChronoContext::default(),
        spline_materials: HashMap::new(),
        surf_strips: HashMap::new(),
        surf_maps: HashMap::new(),
        plate_tris: HashMap::new(),
        fonts: opts_bevy::text_texture::FontRegistry::default(),
        vehicle_text_defs: Vec::new(),
        vehicle_script_defs: Vec::new(),
        vehicle_script_textures: Arc::default(),
        ground_material: None,
        ground_array: None,
        spawn: None,
        repaint: None,
        model_repaints: HashMap::new(),
        formation_cars: Vec::new(),
        start_tile: (0, 0),
        grid: opts_core::formats::map::TileGrid::default(),
        missing_listed: 0,
        camera: None,
        camera_std_index: 0,
        no_vehicle: false,
        parked_permille: 1000,
        resolved_entry: None,
        start_entry: None,
    }
}

/// Tile grid: 300 m, except for maps with world coordinates (Mercator rows when the splines'
/// border chains agree with them, else the measured pitch).
fn measure_grid(job: &mut WorldLoadJob) {
    let mut sample = Vec::new();
    let keys: Vec<(i32, i32)> = job.manifest.tiles.iter().map(|t| (t.x, t.y)).collect();
    for key in keys {
        if let Some(tile) = job.read_tile(key) {
            sample.push((key, tile.splines));
        }
        if sample.len() >= 80 {
            break;
        }
    }
    let (grid, fit) =
        opts_core::formats::map::TileGrid::measure(&sample, job.manifest.world_coordinates);
    job.grid = grid;
    tracing::info!(
        ?grid,
        pairs = fit.pairs,
        median_step_m = ?fit.median_step,
        mercator_residual_m = ?fit.mercator_residual,
        sampled_tiles = sample.len(),
        "tile grid"
    );
}

/// Start tile and spawn: a chosen trip's first bus stop, else the chosen `[entrypoints]` record if
/// it resolves to a placed `entrypoint_*.sco` object, otherwise the first road spline of the first
/// usable tile. Sets the job's spawn point and start tile (the floating origin: all scene
/// coordinates are relative to the start tile's south-west corner).
fn choose_start(job: &mut WorldLoadJob) -> Result<(), String> {
    let mut keys: Vec<(i32, i32)> = job.manifest.tiles.iter().map(|t| (t.x, t.y)).collect();
    keys.dedup();
    let at_stop = job
        .config
        .start_station
        .and_then(|id| job.find_stop_spawn(id, &keys));
    if job.config.start_station.is_some() && at_stop.is_none() {
        tracing::warn!(
            station = ?job.config.start_station,
            "the trip's first stop is not on the map; starting at the entry point"
        );
    }
    let mut start = at_stop.or_else(|| job.find_entry_spawn(&keys));
    job.start_entry = job.resolved_entry.take();
    if start.is_none() {
        tracing::warn!(
            entry = %job.config.entry_point,
            "entry point not resolvable; falling back to the first road spline"
        );
        for key in keys {
            let Some(tile) = job.read_tile(key) else {
                continue;
            };
            if let Some(spawn) = job.find_drivable_spawn(&tile) {
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
    // 3.4e6 north), where f32 resolution is a quarter of a metre. The spawn point is already in
    // the start tile's frame.
    job.spawn = Some(spawn);
    job.start_tile = start_tile;
    Ok(())
}

/// [`begin_world_load`] for a map date: the Chrono entries active on it are applied to every
/// tile read (`None`: the base map).
pub fn begin_world_load_at(
    config: OmsiSessionConfig,
    map_date: Option<opts_core::formats::chrono::ChronoDate>,
) -> Result<WorldLoadJob, String> {
    let install = install_of(&config);
    let manifest = read_manifest(&install, &config)?;

    let tile_files: HashMap<(i32, i32), String> = manifest
        .tiles
        .iter()
        .map(|t| ((t.x, t.y), t.file_name.clone()))
        .collect();

    let mut job = new_job(install, config, manifest, tile_files);

    opts_bevy::scene_trace::begin_session(&job.map_name(), job.install.root());
    job.init_chrono(map_date);

    measure_grid(&mut job);

    choose_start(&mut job)?;
    let start_tile = job.start_tile;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let key = (start_tile.0 + dx, start_tile.1 + dy);
            if job.tile_path(key).is_some() {
                job.queue.push_back(key);
            } else {
                if job.manifest.tiles.iter().any(|t| (t.x, t.y) == key) {
                    job.missing_listed += 1;
                }
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

mod lookup;
mod parked;

/// The parking list a `car_park` marker's first string value selects: blank, `0` or no value is
/// `0` (`parklist_p.txt`), a number `N` is `N` (`parklist_p_N.txt`). Anything else counts as `0`.
pub(super) fn parklist_selector(value: Option<&str>) -> u32 {
    value
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

/// File name of the parking list with this selector.
pub(super) fn parklist_file_name(selector: u32) -> String {
    if selector == 0 {
        "parklist_p.txt".to_owned()
    } else {
        format!("parklist_p_{selector}.txt")
    }
}

#[cfg(test)]
mod tests;
