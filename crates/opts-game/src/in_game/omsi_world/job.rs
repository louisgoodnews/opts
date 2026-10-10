//! Session resolution and the `WorldLoadJob` implementation.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::{ContentCache, OmsiInstall};
use opts_core::formats::map::{MapManifest, MapTile};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};

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
                tracing::error!(path = %path.display(), "tile parse failed: {e}");
                None
            }
        }
    }

    pub(super) fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
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

    pub(super) fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
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
