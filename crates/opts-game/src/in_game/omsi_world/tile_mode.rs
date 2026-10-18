//! The tile loading controls on the world load job (M-0080): the mode that decides what stays
//! resident, the initial queue and the reports the control protocol and the overlay read.

use crate::in_game::tile_loading::{self, TileMode};

use super::WorldLoadJob;

impl WorldLoadJob {
    /// What stays resident around the camera.
    #[must_use]
    pub fn tile_mode(&self) -> TileMode {
        self.tile_mode
    }

    /// Changes what stays resident; the streamer follows from the next frame (it loads what is
    /// missing and, in radius mode, unloads what is too far).
    pub fn set_tile_mode(&mut self, mode: TileMode) {
        if self.tile_mode != mode {
            self.tile_mode = mode;
            self.missing_tiles.clear();
        }
    }

    /// Every tile `global.cfg` lists, whether its file exists or not.
    pub fn listed_tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.tile_files.keys().copied()
    }

    /// The radius at which the map is covered from the start tile.
    #[must_use]
    pub fn map_radius(&self) -> u32 {
        tile_loading::max_radius(self.listed_tiles(), self.start_tile)
    }

    /// The resident tiles, in key order.
    #[must_use]
    pub fn resident_tiles(&self) -> Vec<(i32, i32)> {
        let mut keys: Vec<_> = self.resident.keys().copied().collect();
        keys.sort_unstable();
        keys
    }

    /// The tile whose build is in progress (it is resident already, with its root entity, and is
    /// filled in over a few frames), if any.
    #[must_use]
    pub fn building_tile(&self) -> Option<(i32, i32)> {
        self.building.as_ref().map(|b| b.key)
    }

    /// Tiles waiting to be built.
    #[must_use]
    pub fn queued_tiles(&self) -> Vec<(i32, i32)> {
        self.queue.iter().copied().collect()
    }

    /// Whether the tile's file exists. Misses are remembered while *all* tiles are wanted, so the
    /// streamer does not look for the same absent files every frame.
    pub(super) fn tile_available(&mut self, key: (i32, i32)) -> bool {
        if self.tile_path(key).is_some() {
            return true;
        }
        if self.tile_mode == TileMode::All {
            self.missing_tiles.insert(key);
        }
        false
    }

    /// Sets the mode and replaces the queue with every tile it wants around the start tile,
    /// nearest first: the loading screen then builds exactly those (`total_tiles` follows), so
    /// *load all* really has everything resident when the drive starts.
    pub fn queue_for_mode(&mut self, mode: TileMode) {
        self.set_tile_mode(mode);
        self.queue.clear();
        self.missing_listed = 0;
        let centre = self.start_tile;
        let wanted = tile_loading::wanted(mode, centre, self.listed_tiles(), |k| {
            self.resident.contains_key(&k)
        });
        for key in wanted {
            if self.tile_path(key).is_some() {
                self.queue.push_back(key);
            } else if self.tile_files.contains_key(&key) {
                self.missing_listed += 1;
            }
        }
        self.total_tiles = self.queue.len();
        tracing::info!(
            mode = mode.name(),
            radius = ?mode.radius(),
            tiles = self.total_tiles,
            "tiles queued for the tile loading mode"
        );
    }
}
