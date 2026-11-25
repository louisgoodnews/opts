//! Bookkeeping of a reload: taking a resident tile out of the job, and forgetting the parsed files
//! a changed file leaves behind.

use std::path::Path;

use opts_bevy::hot_reload::cache::normalize;

use super::super::WorldLoadJob;
use super::{ReloadPlan, TileUnload};

impl WorldLoadJob {
    /// Takes a resident tile out of the job's bookkeeping; the returned [`TileUnload`] removes it
    /// from the world. `None` when the tile is not resident.
    pub(in super::super) fn unload_tile(
        &mut self,
        key: (i32, i32),
        keep_ground: bool,
    ) -> Option<TileUnload> {
        let root = self.resident.remove(&key)?;
        self.tile_deps.remove(&key);
        // A tile that is still being built stops here; its root and the children added so far
        // are despawned with the unload.
        if self.building.as_ref().is_some_and(|b| b.key == key) {
            self.building = None;
            opts_bevy::set_loading_tile(None);
        }
        Some(TileUnload {
            key,
            root,
            keep_ground,
        })
    }

    pub(super) fn respawn_tiles(&mut self, keys: &[(i32, i32)], plan: &mut ReloadPlan) {
        for key in keys {
            if let Some(unload) = self.unload_tile(*key, false) {
                plan.unloads.push(unload);
                if !self.queue.contains(key) {
                    self.queue.push_front(*key);
                }
                plan.tiles += 1;
            }
        }
    }

    /// Drops the cached parse results of `path` (`.sco`/`.sli`) and the spline materials built from it.
    pub(super) fn forget_parsed(&mut self, path: &Path) {
        self.sco_cache.retain(|p, _| normalize(p) != *path);
        self.sli_cache.retain(|p, _| normalize(p) != *path);
        // Scripts are compiled per `.sco`; recompiling on demand is cheap.
        self.scenery.clear_templates();
        self.spline_materials
            .retain(|(p, _), _| normalize(p) != *path);
    }
}
