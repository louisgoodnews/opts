//! Session resolution and the `WorldLoadJob` implementation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapSplineInstance, MapTile};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

use super::vehicle::FormationCar;
use crate::catalogue_service::CatalogueService;
use crate::in_game::attachments::SplineIndex;
use crate::new_game_screen::types::GameCatalogue;
use crate::omsi_root::locate_omsi_root;

use super::ground::tile_origin;
/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{OmsiSessionConfig, SessionRequest, WorldLoadJob};

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

    /// Tiles loaded so far and tiles asked for (the 3x3 neighbourhood), for the loading screen.
    #[must_use]
    pub fn tile_counts(&self) -> (usize, usize) {
        (
            self.total_tiles.saturating_sub(self.queue.len()),
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

/// Parses `global.cfg`, picks the start tile and queues its 3×3 neighbourhood.
pub fn begin_world_load(config: OmsiSessionConfig) -> Result<WorldLoadJob, String> {
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
        cache: decoder_cache(),
        config,
        manifest,
        tile_files,
        queue: VecDeque::new(),
        waiting_decode: HashMap::new(),
        resident: HashMap::new(),
        stream_radius: 1,
        total_tiles: 0,
        sli_cache: HashMap::new(),
        tile_deps: HashMap::new(),
        loading_deps: HashSet::new(),
        vehicle_dirs: Vec::new(),
        spline_index: None,
        tile_splines: HashMap::new(),
        placed: HashMap::new(),
        sco_cache: HashMap::new(),
        collision_bounds: HashMap::new(),
        scenery: super::scenery_objects::SceneryObjectCache::new(),
        parklist: None,
        spline_materials: HashMap::new(),
        fonts: opts_bevy::text_texture::FontRegistry::default(),
        vehicle_text_defs: Vec::new(),
        ground_material: None,
        ground_array: None,
        spawn: None,
        repaint: None,
        formation_cars: Vec::new(),
        start_tile: (0, 0),
        missing_listed: 0,
        camera: None,
        camera_std_index: 0,
        no_vehicle: false,
        resolved_entry: None,
        start_entry: None,
    };

    opts_bevy::scene_trace::begin_session(&job.map_name(), job.install.root());

    // Tile size: 300 m, except for maps with world coordinates (measured from the splines).
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
        let size = opts_core::formats::map::estimate_tile_size(&sample);
        opts_bevy::set_tile_size(size);
        tracing::info!(
            tile_size_m = size,
            sampled_tiles = sample.len(),
            "tile size"
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

    /// Absolute OMSI position of the session frame's origin (the start tile's corner).
    #[must_use]
    pub fn session_origin(&self) -> (f32, f32) {
        tile_origin(self.start_tile)
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

    /// Index of every spline on the map, built by reading all tiles once.
    pub(super) fn spline_index(&mut self) -> Arc<SplineIndex> {
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

    pub(super) fn tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        let name = self.tile_files.get(&key)?;
        self.install.find_in(&self.config.map_dir, name)
    }

    pub(super) fn read_tile(&self, key: (i32, i32)) -> Option<MapTile> {
        let path = self.tile_path(key)?;
        let bytes = std::fs::read(&path).ok()?;
        match MapTile::parse(&bytes) {
            Ok(t) => Some(t),
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "tile parse failed: {e}");
                None
            }
        }
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

    /// Picks the parked vehicle for a parking-bay marker from `parklist_p.txt`.
    ///
    /// One `.sco` per line; duplicates raise the pick probability (`docs/src/formats/map/lists.md`).
    /// The draw is a hash of the marker's id code, so a bay shows the same car on every load.
    pub(super) fn parked_vehicle(&mut self, id_code: u64) -> Option<String> {
        if self.parklist.is_none() {
            let path = self.install.find_in(&self.config.map_dir, "parklist_p.txt");
            let list = match path.map(|p| std::fs::read(&p).map(|b| (p, b))) {
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
                    tracing::error!(at = %opts_bevy::position_tag(), "parklist_p.txt unreadable: {e}");
                    Vec::new()
                }
                None => {
                    tracing::error!(at = %opts_bevy::position_tag(), map = %self.config.map_dir.display(), "map has parking bays but no parklist_p.txt");
                    Vec::new()
                }
            };
            self.parklist = Some(list);
        }
        let list = self.parklist.as_deref().unwrap_or_default();
        if list.is_empty() {
            return None;
        }
        // splitmix64 finaliser: spreads consecutive ids evenly.
        let mut h = id_code.wrapping_add(0x9E37_79B9_7F4A_7C15);
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
                Some(Arc::new(o))
            }
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), path = %path.display(), "sco parse failed: {e}");
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
                let o = tile_origin(key);
                let pos = Vec3::new(o.0 + obj.position.x, o.1 + obj.position.y, ep.position.y);
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
                    |_, _| 0.0,
                );
                if let Some((obj, _)) = placed.first() {
                    let o = tile_origin(key);
                    let pos = Vec3::new(o.0 + obj.position.x, o.1 + obj.position.y, ep.position.y);
                    tracing::info!(entry = %ep.name, id_code = ep.id_code, sco = %obj.sco_file, tile = ?key, "entry point resolved (spline attachment)");
                    self.resolved_entry = Some((ep.id, ep.name.clone()));
                    return Some((key, (pos, obj.heading_deg)));
                }
            }
        }
        None
    }

    /// Finds the first road-vehicle path (type 0, forward or bidirectional) in a tile.
    pub(super) fn find_drivable_spawn(
        &mut self,
        key: (i32, i32),
        tile: &MapTile,
    ) -> Option<(Vec3, f32)> {
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

/// Content cache with the decoder configured: parameters (developer overrides from the
/// environment) and a background pool sized to leave cores for the renderer.
fn decoder_cache() -> ContentCache {
    use opts_bevy::descramble_cache::DecodePool;
    use opts_core::formats::o3d::descramble::DecodeParams;
    let params = DecodeParams::default().with_overrides(|k| std::env::var(k).ok());
    let threads =
        std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(2).clamp(1, 4));
    let mut cache = ContentCache::default();
    cache.decode_pool = Some(std::sync::Arc::new(DecodePool::new(
        threads,
        params.clone(),
    )));
    cache.decode_params = params;
    cache
}
