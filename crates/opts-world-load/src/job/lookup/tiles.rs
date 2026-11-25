//! The map's tile files: listing and reading tiles, their splines and alignments.

use super::*;

/// A listed tile of the full map: its key, its file name and its south-west corner. The same
/// shape as the full map's `hud::fullmap::overview::ListedTile` (a type alias, so both are one type).
type ListedTile = ((i32, i32), String, (f32, f32));

impl WorldLoadJob {
    /// Index of every spline on the map with its chain offset, built by reading all tiles once
    /// (through [`Self::read_tile`], so it sees what the tiles hold for the session).
    pub fn spline_index(&mut self) -> Arc<SplineIndex> {
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
    pub fn tile_listing(
        &self,
    ) -> (
        PathBuf,
        PathBuf,
        Vec<ListedTile>,
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
    pub fn tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        self.base_tile_path(key)
            .or_else(|| self.chrono_new_tile(key))
    }

    fn base_tile_path(&self, key: (i32, i32)) -> Option<PathBuf> {
        let name = self.tile_files.get(&key)?;
        self.install.find_in(&self.config.map_dir, name)
    }

    /// Parses a tile and applies the Chrono overlays active on the map date.
    pub fn read_tile(&self, key: (i32, i32)) -> Option<MapTile> {
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
    pub fn splines_of(
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
    pub fn aligned_of(&mut self, key: (i32, i32)) -> Arc<Vec<u64>> {
        if !self.tile_aligned.contains_key(&key) {
            self.tile_splines.remove(&key);
            self.splines_of(key);
        }
        self.tile_aligned.get(&key).cloned().unwrap_or_default()
    }

    /// Where a referenced file really is: at its path, or, failing that, under another folder name.
    pub fn locate(&self, rel: &str) -> Option<PathBuf> {
        self.install
            .find_file(rel)
            .or_else(|| self.install.find_relocated(rel))
    }
}
