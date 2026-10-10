//! Hot reload: which parts of the loaded world depend on a changed file, and the cache and
//! bookkeeping changes that make the streamer rebuild them (see `docs/src/architecture/hot-reload.md`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::hot_reload::cache::normalize;
use opts_bevy::hot_reload::ReloadKind;

use super::WorldLoadJob;

/// What the application has to do after [`WorldLoadJob::plan_reload`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReloadPlan {
    /// Tile roots to despawn (the streamer re-queues and reloads the tiles).
    pub despawn: Vec<Entity>,
    /// Tiles being rebuilt.
    pub tiles: usize,
    /// Textures overwritten in place.
    pub textures: usize,
    /// Meshes dropped from the cache.
    pub meshes: usize,
    /// The player vehicle depends on the file: restart the session.
    pub restart_vehicle: bool,
}

impl ReloadPlan {
    /// A readable summary (`2 tiles, 1 texture in place`), or `nothing depends on the file`.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.tiles > 0 {
            parts.push(format!("{} tile(s) rebuilt", self.tiles));
        }
        if self.textures > 0 {
            parts.push(format!("{} texture(s) updated in place", self.textures));
        }
        if self.meshes > 0 {
            parts.push(format!("{} mesh(es) re-read", self.meshes));
        }
        if self.restart_vehicle {
            parts.push("vehicle changed: session restarted".to_string());
        }
        if parts.is_empty() {
            "nothing loaded depends on this file".to_string()
        } else {
            parts.join(", ")
        }
    }
}

impl WorldLoadJob {
    fn respawn_tiles(&mut self, keys: &[(i32, i32)], plan: &mut ReloadPlan) {
        for key in keys {
            if let Some(root) = self.resident.remove(key) {
                plan.despawn.push(root);
                self.tile_deps.remove(key);
                if !self.queue.contains(key) {
                    self.queue.push_front(*key);
                }
                plan.tiles += 1;
            }
        }
    }

    /// Drops the cached parse results of `path` (`.sco`/`.sli`) and the spline materials built from it.
    fn forget_parsed(&mut self, path: &Path) {
        self.sco_cache.retain(|p, _| normalize(p) != *path);
        self.sli_cache.retain(|p, _| normalize(p) != *path);
        self.spline_materials
            .retain(|(p, _), _| normalize(p) != *path);
    }

    /// Updates caches for a validated change of `path` and lists what must be rebuilt.
    ///
    /// # Errors
    /// The decoder's message when a texture cannot be re-read (nothing is changed then).
    pub fn plan_reload(
        &mut self,
        path: &Path,
        kind: ReloadKind,
        images: &mut Assets<Image>,
    ) -> Result<ReloadPlan, String> {
        let path = normalize(path);
        let mut plan = ReloadPlan::default();
        let in_place = kind == ReloadKind::Texture && self.cache.reload_texture(&path, images)?;
        if in_place {
            plan.textures = 1;
        }
        plan.meshes = self.cache.invalidate_mesh(&path);
        self.forget_parsed(&path);
        let mut rebuild: Vec<(i32, i32)> = self
            .tile_deps
            .iter()
            .filter(|(_, deps)| deps.contains(&path))
            .map(|(k, _)| *k)
            .collect();
        // A texture that was patched in place needs no tile rebuild unless the tile built it into
        // its own masks (those are tracked as dependencies too).
        if in_place && kind == ReloadKind::Texture {
            rebuild.retain(|k| self.tile_deps.get(k).is_some_and(|d| d.contains(&path)));
        }
        rebuild.sort_unstable();
        self.respawn_tiles(&rebuild, &mut plan);
        let ground = self.manifest.ground_textures.iter().any(|g| {
            self.install
                .find_in(&self.config.map_dir, &g.ground_texture)
                .is_some_and(|p| normalize(&p) == path)
        });
        if ground && self.ground_array.take().is_some() {
            let all: Vec<_> = self.resident.keys().copied().collect();
            self.respawn_tiles(&all, &mut plan);
        }
        plan.restart_vehicle = !in_place && self.vehicle_dirs.iter().any(|d| path.starts_with(d));
        Ok(plan)
    }

    /// Rebuilds every resident tile and forgets all parsed objects, meshes and ground layers.
    pub fn plan_scene_reload(&mut self) -> ReloadPlan {
        let mut plan = ReloadPlan {
            meshes: self.cache.mesh_count(),
            ..ReloadPlan::default()
        };
        self.cache.clear_meshes();
        self.sco_cache.clear();
        self.sli_cache.clear();
        self.spline_materials.clear();
        self.fonts = opts_bevy::text_texture::FontRegistry::default();
        self.ground_array = None;
        let all: Vec<_> = self.resident.keys().copied().collect();
        self.respawn_tiles(&all, &mut plan);
        plan
    }

    /// Directories that hold loaded content (cache entries, parsed objects, map and vehicle folders).
    #[must_use]
    pub fn watch_dirs(&self) -> HashSet<PathBuf> {
        let mut dirs = self.cache.loaded_dirs();
        dirs.extend(
            self.sco_cache
                .keys()
                .chain(self.sli_cache.keys())
                .filter_map(|p| p.parent().map(Path::to_path_buf)),
        );
        dirs.extend(
            self.tile_deps
                .values()
                .flatten()
                .filter_map(|p| p.parent().map(Path::to_path_buf)),
        );
        dirs.extend(self.vehicle_dirs.iter().cloned());
        dirs
    }

    /// The map folder (watched recursively).
    #[must_use]
    pub fn map_dir(&self) -> &Path {
        &self.config.map_dir
    }
}
