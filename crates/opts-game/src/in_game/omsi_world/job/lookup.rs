//! Lookups on the load job: grid and tile positions, start entries, vehicle files and the parking list.

use super::*;

/// How far behind a bus stop object a vehicle starting there stands when its `.bus` gives no
/// bounding box (m): about half a 12 m bus, so its front is at the stop.
pub(in super::super::super) const STOP_SPAWN_BACK_M: f32 = 6.0;

/// The distance from a vehicle's origin to its front (m): the `.bus` bounding box's centre plus
/// half its length along the vehicle (`y`). For an articulated bus that is the front unit.
#[must_use]
pub(in super::super::super) fn front_of(
    bus: &opts_core::formats::vehicles::BusManifest,
) -> Option<f32> {
    bus.bounding_box
        .map(|(size, centre)| centre.y + size.y * 0.5)
        .filter(|f| f.is_finite() && (0.5..30.0).contains(f))
}

impl WorldLoadJob {
    /// The map's timetable files (`TTData`), read on the first call and shared by every vehicle
    /// of the session; `None` when the map has none.
    pub fn timetables(
        &mut self,
    ) -> Option<std::sync::Arc<opts_core::formats::timetable::Timetables>> {
        self.timetables
            .get_or_insert_with(|| {
                let t = opts_bevy::route::find_timetables(&self.config.map_dir);
                if let Some(t) = &t {
                    tracing::info!(
                        stops = t.stops.len(),
                        lines = t.lines.len(),
                        "timetable files (TTData) loaded"
                    );
                }
                t.map(std::sync::Arc::new)
            })
            .clone()
    }

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

    /// The parsed sound configuration that `rel` names (relative to the object's folder
    /// `base`), with the folder it was found in; `None` when it is missing or unreadable.
    pub fn sound_config(
        &mut self,
        base: &Path,
        rel: &str,
    ) -> Option<(Arc<opts_core::formats::sound::SoundConfig>, PathBuf)> {
        let path = self.install.find_in(base, rel)?;
        let dir = path.parent()?.to_path_buf();
        if let Some(c) = self.sound_cfgs.get(&path) {
            return c.clone().map(|c| (c, dir));
        }
        let parsed = std::fs::read(&path).ok().map(|bytes| {
            // Windows-1252 text; the keywords and file names are ASCII.
            let text: String = bytes.iter().map(|b| char::from(*b)).collect();
            Arc::new(opts_core::formats::sound::SoundConfig::parse(&text))
        });
        if let Some(c) = &parsed {
            for w in &c.warnings {
                tracing::debug!(file = %path.display(), "sound config: {w}");
            }
        }
        self.sound_cfgs.insert(path, parsed.clone());
        parsed.map(|c| (c, dir))
    }

    /// Like [`Self::sound_config`] without the cache, for callers that only hold `&self` (the
    /// AI vehicle types parse theirs once when they load).
    #[must_use]
    pub fn read_sound_config(
        &self,
        base: &Path,
        rel: &str,
    ) -> Option<(Arc<opts_core::formats::sound::SoundConfig>, PathBuf)> {
        let path = self.install.find_in(base, rel)?;
        let dir = path.parent()?.to_path_buf();
        let bytes = std::fs::read(&path).ok()?;
        let text: String = bytes.iter().map(|b| char::from(*b)).collect();
        Some((
            Arc::new(opts_core::formats::sound::SoundConfig::parse(&text)),
            dir,
        ))
    }

    /// The sound file a configuration names: relative to the configuration's folder (names are
    /// matched case-insensitively), else a file of that name in the shared `Sounds` folder.
    #[must_use]
    pub fn find_sound_file(&self, cfg_dir: &Path, name: &str) -> Option<PathBuf> {
        let name = name.trim().replace('\\', "/");
        self.install.find_in(cfg_dir, &name).or_else(|| {
            let base = name.rsplit('/').next().unwrap_or(&name);
            self.install
                .find_in(&self.install.root().join("Sounds"), base)
        })
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
    pub(in super::super::super) fn spline_index(&mut self) -> Arc<SplineIndex> {
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

    /// What a background reader needs to list the roads of every tile of the map (the overview of
    /// the full-size map): the installation root, the map folder and, for each listed tile, its
    /// key, its file name and its south-west corner in the session frame (OMSI axes, metres).
    #[must_use]
    pub(crate) fn tile_listing(
        &self,
    ) -> (
        PathBuf,
        PathBuf,
        Vec<crate::hud::fullmap::overview::ListedTile>,
    ) {
        let mut tiles: Vec<_> = self
            .tile_files
            .iter()
            .map(|(k, f)| (*k, f.clone(), self.tile_offset(*k)))
            .collect();
        tiles.sort_by_key(|t| t.0);
        (
            self.install.root().to_path_buf(),
            self.config.map_dir.clone(),
            tiles,
        )
    }

    /// The tile's file: the base file, or for a tile that only Chrono adds, its first overlay.
    pub(in super::super::super) fn tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        self.base_tile_path(key)
            .or_else(|| self.chrono_new_tile(key))
    }

    fn base_tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        let name = self.tile_files.get(&key)?;
        self.install.find_in(&self.config.map_dir, name)
    }

    /// Parses a tile and applies the Chrono overlays active on the map date.
    pub(in super::super::super) fn read_tile(&self, key: (i32, i32)) -> Option<MapTile> {
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
    pub(in super::super::super) fn splines_of(
        &mut self,
        key: (i32, i32),
    ) -> Option<Arc<Vec<MapSplineInstance>>> {
        if let Some(s) = self.tile_splines.get(&key) {
            return Some(s.clone());
        }
        self.tile_files.get(&key)?;
        let tile = self.read_tile(key)?;
        let splines = Arc::new(tile.splines);
        self.tile_splines.insert(key, splines.clone());
        self.tile_aligned
            .insert(key, Arc::new(tile.terrain_aligned));
        Some(splines)
    }

    /// The ID codes of tile `key`'s splines the terrain is aligned to (`[spline_terrain_align]`).
    pub(in super::super::super) fn aligned_of(&mut self, key: (i32, i32)) -> Arc<Vec<u64>> {
        if !self.tile_aligned.contains_key(&key) {
            self.tile_splines.remove(&key);
            self.splines_of(key);
        }
        self.tile_aligned.get(&key).cloned().unwrap_or_default()
    }

    /// Where a referenced file really is: at its path, or, failing that, under another folder name.
    pub(in super::super::super) fn locate(&self, rel: &str) -> Option<PathBuf> {
        self.install
            .find_file(rel)
            .or_else(|| self.install.find_relocated(rel))
    }
}

mod spawn;
