//! Overlay plan of a Chrono map date: which overlay files a tile reads and how they are applied.

use super::*;

impl WorldLoadJob {
    /// The overlay files of a tile, in application order.
    pub(in crate::in_game::omsi_world) fn chrono_overlays(&self, key: (i32, i32)) -> &[PathBuf] {
        self.chrono.plan.overlays(key)
    }

    /// The overlay a tile without a base file comes from.
    pub(in crate::in_game::omsi_world) fn chrono_new_tile(
        &self,
        key: (i32, i32),
    ) -> Option<PathBuf> {
        self.chrono.new_tiles.get(&key).cloned()
    }

    /// Applies the active overlays of `key` to its base tile; what could not be applied goes to
    /// the tile's `warnings` (the loader logs them).
    pub(in crate::in_game::omsi_world) fn apply_chrono(&self, key: (i32, i32), tile: &mut MapTile) {
        for path in self.chrono_overlays(key) {
            let overlay = match std::fs::read(path) {
                Ok(bytes) => TileOverlay::parse(&bytes),
                Err(e) => {
                    tile.warnings
                        .push(format!("Chrono overlay {} unreadable: {e}", path.display()));
                    continue;
                }
            };
            if tile.version.is_none() {
                tile.version = overlay.version;
            }
            let report = tile.apply_overlay(&overlay);
            let name = path
                .parent()
                .and_then(|p| p.file_name())
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            tile.warnings.extend(
                overlay
                    .warnings
                    .iter()
                    .chain(&report.warnings)
                    .map(|w| format!("Chrono {name}: {w}")),
            );
        }
    }
}
