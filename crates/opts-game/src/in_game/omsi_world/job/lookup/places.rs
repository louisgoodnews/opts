//! Where things are on the load job: the grid and tiles, map and start entries, and the
//! configured livery and options.

use super::*;

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
}
