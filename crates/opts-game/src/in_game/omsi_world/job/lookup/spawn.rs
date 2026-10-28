//! Profile, scenery object and spawn-point lookups on `WorldLoadJob`.

use super::*;

impl WorldLoadJob {
    pub(crate) fn sli(&mut self, rel: &str) -> Option<Arc<SplineProfile>> {
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

    pub(crate) fn sco(&mut self, rel: &str) -> Option<(PathBuf, Arc<SceneryObject>)> {
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
    pub(crate) fn find_entry_spawn(
        &mut self,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), (Vec3, f32))> {
        let eps = &self.manifest.entry_points;
        // Developer aid: `OPTS_DEV_ENTRY=<substring>` picks the first entry point containing it.
        let dev = std::env::var("OPTS_DEV_ENTRY").ok();
        let wanted = self.config.entry_point.trim().to_lowercase();
        // The configured start point: its exact name, else the same without regard to case, else
        // the first one whose name contains the text (`--entry "Ahlheim Hbf"`), else the first.
        let named = |e: &&opts_core::formats::map::MapEntryPoint| e.name == self.config.entry_point;
        let folded = |e: &&opts_core::formats::map::MapEntryPoint| e.name.to_lowercase() == wanted;
        let part = || {
            eps.iter()
                .find(|e| !wanted.is_empty() && e.name.to_lowercase().contains(&wanted))
        };
        let ep = eps
            .iter()
            .find(|e| dev.as_deref().is_some_and(|d| e.name.contains(d)))
            .or_else(|| eps.iter().find(named))
            .or_else(|| eps.iter().find(folded))
            .or_else(part)
            .or_else(|| eps.first())?
            .clone();
        self.find_entry_spawn_of(&ep, keys)
    }

    /// [`Self::find_entry_spawn`] for the entry record with this number.
    pub(crate) fn find_entry_spawn_by_id(
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
        let (key, position, heading, sco) = self.find_placed_object(u64::from(ep.id_code), keys)?;
        tracing::info!(entry = %ep.name, id_code = ep.id_code, %sco, tile = ?key, "entry point resolved");
        self.resolved_entry = Some((ep.id, ep.name.clone()));
        Some((
            key,
            (Vec3::new(position.x, position.y, ep.position.y), heading),
        ))
    }

    /// The spawn at the bus stop object with this station id (the trip's first stop, M-0145):
    /// the stop marks where a bus's front comes to rest, so the vehicle stands with its front
    /// there ([`front_of`] its `.bus`, else [`STOP_SPAWN_BACK_M`] behind the stop).
    pub(crate) fn find_stop_spawn(
        &mut self,
        station_id: u32,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), (Vec3, f32))> {
        let (key, position, heading, sco) = self.find_placed_object(u64::from(station_id), keys)?;
        let h = heading.to_radians();
        let front = self
            .config
            .vehicle_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| opts_core::formats::vehicles::BusManifest::parse_bytes(&b).ok())
            .and_then(|m| front_of(&m))
            .unwrap_or(STOP_SPAWN_BACK_M);
        let back = Vec3::new(h.sin(), h.cos(), 0.0) * front;
        tracing::info!(station_id, %sco, tile = ?key, front_m = front, "trip start stop resolved");
        Some((key, (position - back, heading)))
    }

    /// The object with this ID code in the first of `keys` that has it: placed in its tile, or
    /// attached to a spline (`[splineAttachement]`, e.g. `entrypoint_bus.sco` along a bus lane
    /// in Wuppertal 639, or a bus stop along a road), whose place follows from the carrier
    /// spline. Position (tile frame), heading and `.sco` file.
    fn find_placed_object(
        &mut self,
        id_code: u64,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), Vec3, f32, String)> {
        for &key in keys {
            let Some(tile) = self.read_tile(key) else {
                continue;
            };
            if let Some(obj) = tile.objects.iter().find(|o| o.id_code == id_code) {
                return Some((key, obj.position, obj.heading_deg, obj.sco_file.clone()));
            }
            if let Some(att) = tile
                .spline_attachments
                .iter()
                .find(|a| a.id_code == id_code)
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
                    return Some((key, obj.position, obj.heading_deg, obj.sco_file.clone()));
                }
            }
        }
        None
    }

    /// Finds the first road-vehicle path (type 0, forward or bidirectional) in a tile; the
    /// position is in that tile's frame.
    pub(crate) fn find_drivable_spawn(&mut self, tile: &MapTile) -> Option<(Vec3, f32)> {
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
