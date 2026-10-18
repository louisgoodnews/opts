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
            .with_twin_lookup(s.twin_lookup())
            .with_relocate_lookup(s.relocate_lookup());
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
        None if req.no_vehicle => None,
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
    if vehicle_path.is_none() && !req.no_vehicle {
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
        livery: req.selected_livery.to_string(),
        setvars: Vec::new(),
        human_path,
        lookup: service.map(CatalogueService::path_lookup),
        name_lookup: service.map(CatalogueService::name_lookup),
        twin_lookup: service.map(CatalogueService::twin_lookup),
        relocate_lookup: service.map(CatalogueService::relocate_lookup),
    })
}

/// Chooses the player's character: a `.hum` whose name says *driver*, else the first one.
pub(super) fn pick_driver_human(
    root: &Path,
    service: Option<&CatalogueService>,
) -> Option<PathBuf> {
    let list = service?.catalogue.human_markers().ok()?;
    let rel = list
        .iter()
        .find(|p| p.to_lowercase().contains("driver"))
        .or_else(|| list.first())?;
    Some(root.join(rel))
}

/// Finds `<OMSI>/Vehicles/<any folder>/<id>.bus`.
pub(super) fn find_bus_file(install: &OmsiInstall, id: &str) -> Option<PathBuf> {
    let vehicles = install.find_file("Vehicles")?;
    let file = format!("{id}.bus");
    std::fs::read_dir(vehicles)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .find_map(|e| install.find_in(&e.path(), &file))
}

/// Parses `global.cfg`, picks the start tile and queues its 3×3 neighbourhood (the base map,
/// without Chrono).
pub fn begin_world_load(config: OmsiSessionConfig) -> Result<WorldLoadJob, String> {
    begin_world_load_at(config, None)
}

/// [`begin_world_load`] for a map date: the Chrono entries active on it are applied to every
/// tile read (`None`: the base map).
pub fn begin_world_load_at(
    config: OmsiSessionConfig,
    map_date: Option<opts_core::formats::chrono::ChronoDate>,
) -> Result<WorldLoadJob, String> {
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
        placed: HashMap::new(),
        sco_cache: HashMap::new(),
        crossing_bases: HashMap::new(),
        ground_faces: HashMap::new(),
        tile_objects: HashMap::new(),
        tile_surfaces: HashMap::new(),
        collision_bounds: HashMap::new(),
        scenery: super::scenery_objects::SceneryObjectCache::new(),
        parklists: HashMap::new(),
        chrono: super::chrono::ChronoContext::default(),
        spline_materials: HashMap::new(),
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
        resolved_entry: None,
        start_entry: None,
    };

    opts_bevy::scene_trace::begin_session(&job.map_name(), job.install.root());
    job.init_chrono(map_date);

    // Tile grid: 300 m, except for maps with world coordinates (Mercator rows when the splines'
    // border chains agree with them, else the measured pitch).
    {
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

    // Start tile and spawn: the chosen `[entrypoints]` record if it resolves to a placed
    // `entrypoint_*.sco` object, otherwise the first road spline of the first usable tile.
    let mut keys: Vec<(i32, i32)> = job.manifest.tiles.iter().map(|t| (t.x, t.y)).collect();
    keys.dedup();
    let mut start = job.find_entry_spawn(&keys);
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

    /// The map's tile grid.
    #[must_use]
    pub fn grid(&self) -> opts_core::formats::map::TileGrid {
        self.grid
    }

    /// South-west corner of tile `key` in the session frame (OMSI axes, metres from the start
    /// tile's corner).
    #[must_use]
    pub fn tile_offset(&self, key: (i32, i32)) -> (f32, f32) {
        let o = self.grid.offset(key, self.start_tile);
        (o.x, o.y)
    }

    /// Absolute OMSI position (see `TileGrid::absolute`) of a session-frame point.
    #[must_use]
    pub fn local_to_absolute(&self, local: Vec2) -> bevy::math::DVec2 {
        self.grid
            .local_to_absolute(local.as_dvec2(), self.start_tile)
    }

    /// Session-frame position of a point given in the frame of tile `key`.
    #[must_use]
    pub fn session_pos(&self, key: (i32, i32), local: Vec3) -> Vec3 {
        let o = self.tile_offset(key);
        Vec3::new(o.0 + local.x, o.1 + local.y, local.z)
    }

    /// Terrain cell of tile row `ty` (m).
    #[must_use]
    pub fn terrain_cell(&self, ty: i32) -> f32 {
        self.grid.cell(ty) as f32
    }

    /// The map's entry points `(record number, name)`, in `global.cfg` order.
    #[must_use]
    pub fn entry_records(&self) -> Vec<(u32, String)> {
        self.manifest
            .entry_points
            .iter()
            .map(|e| (e.id, e.name.clone()))
            .collect()
    }

    /// Where the next vehicle is put: session-frame position (OMSI axes) and compass heading.
    #[must_use]
    pub fn spawn_point(&self) -> Option<(Vec3, f32)> {
        self.spawn
    }

    /// The repaint the next vehicle is built with (empty: the vehicle's default).
    #[must_use]
    pub fn config_livery(&self) -> &str {
        &self.config.livery
    }

    /// The equipment options the next vehicle is built with.
    #[must_use]
    pub fn config_setvars(&self) -> &[(String, i32)] {
        &self.config.setvars
    }

    /// The entry point the session started at: `(record number, name)`; `None` when the start
    /// fell back to the first road spline (no entry point resolved).
    #[must_use]
    pub fn start_entry(&self) -> Option<(u32, &str)> {
        self.start_entry.as_ref().map(|(id, n)| (*id, n.as_str()))
    }

    /// The entry record whose name equals `name` (case-insensitive), else the only one that
    /// contains it. `None` when none or several match.
    #[must_use]
    pub fn entry_by_name(&self, name: &str) -> Option<(u32, String)> {
        let wanted = name.trim().to_lowercase();
        let eps = &self.manifest.entry_points;
        eps.iter()
            .find(|e| e.name.to_lowercase() == wanted)
            .map(|e| (e.id, e.name.clone()))
            .or_else(|| {
                let mut hits = eps
                    .iter()
                    .filter(|e| e.name.to_lowercase().contains(&wanted));
                let first = hits.next()?;
                hits.next()
                    .is_none()
                    .then(|| (first.id, first.name.clone()))
            })
    }

    /// Name of the entry record with this number.
    #[must_use]
    pub fn entry_name(&self, id: u32) -> Option<String> {
        self.manifest
            .entry_points
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.name.clone())
    }

    /// The entry record the session was asked to start at: by name from the setup, else the first.
    #[must_use]
    pub fn configured_entry(&self) -> Option<(u32, String)> {
        self.entry_by_name(&self.config.entry_point.clone())
            .or_else(|| {
                self.manifest
                    .entry_points
                    .first()
                    .map(|e| (e.id, e.name.clone()))
            })
    }

    /// The `.bus` file the session's vehicle was loaded from.
    #[must_use]
    pub fn vehicle_file(&self) -> Option<&Path> {
        self.config.vehicle_path.as_deref()
    }

    /// The `.bus` file of a catalogue vehicle (`bus_file` is relative to the OMSI root).
    #[must_use]
    pub fn bus_file(&self, rel: &str) -> Option<PathBuf> {
        if rel.is_empty() {
            return None;
        }
        self.install.find_file(rel)
    }

    /// A vehicle file named relative to `base` (`[couple_back]` and the like).
    #[must_use]
    pub fn find_vehicle_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        self.install.find_in(base, rel)
    }

    /// Sets the cars that follow the head of the train formation spawned next (cleared by the
    /// loader when it has used them).
    pub fn set_formation_cars(&mut self, cars: Vec<FormationCar>) {
        self.formation_cars = cars;
    }

    /// Tile the player starts on.
    #[must_use]
    pub fn start_tile(&self) -> (i32, i32) {
        self.start_tile
    }

    /// Index of every spline on the map with its chain offset, built by reading all tiles once
    /// (through [`Self::read_tile`], so it sees what the tiles hold for the session).
    pub(super) fn spline_index(&mut self) -> Arc<SplineIndex> {
        if let Some(i) = &self.spline_index {
            return i.clone();
        }
        let t = std::time::Instant::now();
        let mut index = SplineIndex::default();
        // Through `read_tile`: the splines as Chrono leaves them on the map date.
        let mut keys: Vec<(i32, i32)> = self.tile_files.keys().copied().collect();
        keys.sort_unstable();
        for key in keys {
            if let Some(tile) = self.read_tile(key) {
                index.add_tile(key, &tile.splines);
            }
        }
        index.finish();
        tracing::info!(
            splines = index.len(),
            ms = t.elapsed().as_millis() as u64,
            "map-wide spline chain index built"
        );
        let index = Arc::new(index);
        self.spline_index = Some(index.clone());
        index
    }

    /// The tile's file: the base file, or for a tile that only Chrono adds, its first overlay.
    pub(super) fn tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        self.base_tile_path(key)
            .or_else(|| self.chrono_new_tile(key))
    }

    fn base_tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        let name = self.tile_files.get(&key)?;
        self.install.find_in(&self.config.map_dir, name)
    }

    /// Parses a tile and applies the Chrono overlays active on the map date.
    pub(super) fn read_tile(&self, key: (i32, i32)) -> Option<MapTile> {
        let mut tile = match self.base_tile_path(key) {
            Some(path) => {
                let bytes = std::fs::read(&path).ok()?;
                match MapTile::parse(&bytes) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "tile parse failed: {e}");
                        return None;
                    }
                }
            }
            None if !self.chrono_overlays(key).is_empty() => MapTile::default(),
            None => return None,
        };
        self.apply_chrono(key, &mut tile);
        Some(tile)
    }

    /// The splines of a tile: kept from its last read, otherwise read now (`None` when the tile
    /// has no readable file).
    pub(super) fn splines_of(&mut self, key: (i32, i32)) -> Option<Arc<Vec<MapSplineInstance>>> {
        if let Some(s) = self.tile_splines.get(&key) {
            return Some(s.clone());
        }
        self.tile_files.get(&key)?;
        let splines = Arc::new(self.read_tile(key)?.splines);
        self.tile_splines.insert(key, splines.clone());
        Some(splines)
    }

    /// Where a referenced file really is: at its path, or, failing that, under another folder name.
    pub(super) fn locate(&self, rel: &str) -> Option<PathBuf> {
        self.install
            .find_file(rel)
            .or_else(|| self.install.find_relocated(rel))
    }

    pub(super) fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
        let path = self.install.find_file(rel).or_else(|| {
            let found = self.install.find_relocated(rel)?;
            tracing::warn!(sli = rel, found = %found.display(), "spline profile found under another folder name");
            Some(found)
        });
        let Some(path) = path else {
            tracing::error!(at = %opts_bevy::position_tag(), sli = rel, "spline profile not found");
            return None;
        };
        self.loading_deps
            .insert(opts_bevy::hot_reload::cache::normalize(&path));
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
                tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "sli parse failed: {e}");
                None
            }
        };
        self.sli_cache.insert(path, out.clone());
        out
    }

    /// Reads the parking list with this selector (`0` = `parklist_p.txt`), once.
    fn parklist(&mut self, selector: u32) -> &[String] {
        if !self.parklists.contains_key(&selector) {
            let name = parklist_file_name(selector);
            let list = match self
                .data_file(&name)
                .map(|p| std::fs::read(&p).map(|b| (p, b)))
            {
                Some(Ok((p, bytes))) => {
                    self.loading_deps
                        .insert(opts_bevy::hot_reload::cache::normalize(&p));
                    let text: String = bytes.iter().map(|&b| char::from(b)).collect();
                    text.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect()
                }
                Some(Err(e)) => {
                    tracing::error!(at = %opts_bevy::position_tag(), "{name} unreadable: {e}");
                    Vec::new()
                }
                None if selector == 0 => {
                    tracing::error!(at = %opts_bevy::position_tag(), map = %self.config.map_dir.display(), "map has parking bays but no parklist_p.txt");
                    Vec::new()
                }
                None => {
                    tracing::warn!(map = %self.config.map_dir.display(), "parking bays select {name}, which the map lacks: parklist_p.txt instead");
                    self.parklist(0).to_vec()
                }
            };
            self.parklists.insert(selector, list);
        }
        self.parklists.get(&selector).map_or(&[], Vec::as_slice)
    }

    /// Picks the parked vehicle for a parking-bay marker.
    ///
    /// The marker's first string value selects the list (blank or `0`: `parklist_p.txt`, `N`:
    /// `parklist_p_N.txt`); one `.sco` per line, duplicates raise the pick probability
    /// (`docs/src/formats/map/lists.md`). The draw is a hash of the marker's id code and position
    /// (the instances of an attachment share the id), so a bay shows the same car on every load.
    pub(super) fn parked_vehicle(&mut self, marker: &MapObjectInstance) -> Option<String> {
        let selector = parklist_selector(marker.string_vars.first().map(String::as_str));
        let list = self.parklist(selector);
        if list.is_empty() {
            return None;
        }
        let cm = |v: f32| (f64::from(v) * 100.0).round() as i64 as u64;
        let mut h = marker.id_code;
        for v in [cm(marker.position.x), cm(marker.position.y)] {
            h = h.rotate_left(21) ^ v;
        }
        // splitmix64 finaliser: spreads consecutive ids evenly.
        h = h.wrapping_add(0x9E37_79B9_7F4A_7C15);
        h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 31;
        Some(list[(h % list.len() as u64) as usize].clone())
    }

    pub(super) fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
        let Some(path) = self.install.find_file(rel).or_else(|| {
            let found = self.install.find_relocated(rel)?;
            tracing::warn!(sco = rel, found = %found.display(), "scenery object found under another folder name");
            Some(found)
        }) else {
            tracing::error!(at = %opts_bevy::position_tag(), sco = rel, "scenery object not found");
            return None;
        };
        self.loading_deps
            .insert(opts_bevy::hot_reload::cache::normalize(&path));
        if let Some(c) = self.sco_cache.get(&path) {
            return c.clone().map(|o| (path, o));
        }
        let parsed = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| SceneryObject::parse(&b).map_err(|e| e.to_string()));
        let out = match parsed {
            Ok(o) => {
                tracing::debug!(path = %path.display(), meshes = o.meshes.len(), "loaded sco");
                // A bent crossing's paths and arms follow its base surface.
                let o = Arc::new(o);
                match self.crossing_base(&path, &o) {
                    Some((_, base)) => Some(Arc::new(o.bent(&base))),
                    None => Some(o),
                }
            }
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "sco parse failed: {e}");
                None
            }
        };
        self.sco_cache.insert(path.clone(), out.clone());
        out.map(|o| (path, o))
    }

    /// Resolves the configured entry point to `(tile, position in that tile, heading)`.
    ///
    /// A `global.cfg` `[entrypoints]` record references an `entrypoint_*.sco` object by its id
    /// code (verified: all 51 Berlin-Spandau records match a placed object). The record's
    /// position is `(x, elevation, y)` in that object's tile; its heading is the object's.
    pub(super) fn find_entry_spawn(
        &mut self,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), (Vec3, f32))> {
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
        self.find_entry_spawn_of(&ep, keys)
    }

    /// [`Self::find_entry_spawn`] for the entry record with this number.
    pub(super) fn find_entry_spawn_by_id(
        &mut self,
        id: u32,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), (Vec3, f32))> {
        let ep = self
            .manifest
            .entry_points
            .iter()
            .find(|e| e.id == id)?
            .clone();
        self.find_entry_spawn_of(&ep, keys)
    }

    fn find_entry_spawn_of(
        &mut self,
        ep: &opts_core::formats::map::MapEntryPoint,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), (Vec3, f32))> {
        for &key in keys {
            let Some(tile) = self.read_tile(key) else {
                continue;
            };
            if let Some(obj) = tile
                .objects
                .iter()
                .find(|o| o.id_code == u64::from(ep.id_code))
            {
                let pos = Vec3::new(obj.position.x, obj.position.y, ep.position.y);
                tracing::info!(entry = %ep.name, id_code = ep.id_code, sco = %obj.sco_file, tile = ?key, "entry point resolved");
                self.resolved_entry = Some((ep.id, ep.name.clone()));
                return Some((key, (pos, obj.heading_deg)));
            }
            // The entry-point object may be attached to a spline (`[splineAttachement]`, e.g.
            // `entrypoint_bus.sco` along a bus lane in Wuppertal 639): its place follows from the
            // carrier spline.
            if let Some(att) = tile
                .spline_attachments
                .iter()
                .find(|a| a.id_code == u64::from(ep.id_code))
            {
                let index = self.spline_index();
                let placed = crate::in_game::attachments::expand_attachments(
                    std::slice::from_ref(att),
                    &tile.splines,
                    &index,
                    key,
                    &self.grid,
                    |_, _| 0.0,
                );
                if let Some((obj, _)) = placed.first() {
                    let pos = Vec3::new(obj.position.x, obj.position.y, ep.position.y);
                    tracing::info!(entry = %ep.name, id_code = ep.id_code, sco = %obj.sco_file, tile = ?key, "entry point resolved (spline attachment)");
                    self.resolved_entry = Some((ep.id, ep.name.clone()));
                    return Some((key, (pos, obj.heading_deg)));
                }
            }
        }
        None
    }

    /// Finds the first road-vehicle path (type 0, forward or bidirectional) in a tile; the
    /// position is in that tile's frame.
    pub(super) fn find_drivable_spawn(&mut self, tile: &MapTile) -> Option<(Vec3, f32)> {
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
            // Lateral offset along the spline's right vector.
            let x = inst.position.x + h.cos() * path.offset_x;
            let y = inst.position.y - h.sin() * path.offset_x;
            return Some((
                Vec3::new(x, y, inst.position.z + path.offset_z),
                inst.heading_deg,
            ));
        }
        None
    }
}

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
mod parklist_tests {
    use super::*;

    #[test]
    fn car_park_value_selects_the_parking_list() {
        let file = |v| parklist_file_name(parklist_selector(v));
        assert_eq!(file(None), "parklist_p.txt");
        assert_eq!(file(Some("")), "parklist_p.txt");
        assert_eq!(file(Some("0")), "parklist_p.txt");
        assert_eq!(file(Some("1")), "parklist_p_1.txt");
        assert_eq!(file(Some(" 2 ")), "parklist_p_2.txt");
        assert_eq!(file(Some("Trabant")), "parklist_p.txt");
    }
}
