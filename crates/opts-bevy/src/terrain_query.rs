//! Ground elevation queries over the terrain of several tiles (M-0025 `GroundElevationQuery`).
//!
//! Heights are bilinear over the 61 × 61 grid of the tile that contains the point, normals come
//! from the central differences of the same surface, so both agree with each other and with
//! [`sample_terrain`]. Specification: `docs/src/architecture/terrain-rendering.md`, section 5.

use std::collections::HashMap;

use bevy::math::Vec3;
use opts_core::formats::map::MapTerrain;

use crate::terrain::{sample_terrain, tile_size};

/// Terrain of the resident tiles, queried by position in tile-grid metres.
///
/// Positions are measured from the south-west corner of tile `(0, 0)` (`x` east, `y` north); a
/// caller that uses a floating origin adds the origin of its start tile first.
#[derive(Debug, Clone, Default)]
pub struct GroundElevationQuery {
    tiles: HashMap<(i32, i32), MapTerrain>,
}

impl GroundElevationQuery {
    /// An empty query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces the terrain of tile `key`.
    pub fn insert(&mut self, key: (i32, i32), terrain: MapTerrain) {
        self.tiles.insert(key, terrain);
    }

    /// Removes a tile (evicted by the streamer).
    pub fn remove(&mut self, key: (i32, i32)) {
        self.tiles.remove(&key);
    }

    /// Number of tiles held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Whether no tile is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    fn locate(&self, x: f32, y: f32) -> Option<(&MapTerrain, f32, f32)> {
        let size = tile_size();
        let key = ((x / size).floor() as i32, (y / size).floor() as i32);
        let t = self.tiles.get(&key)?;
        Some((t, x - key.0 as f32 * size, y - key.1 as f32 * size))
    }

    /// Terrain height at `(x, y)`; `None` when no tile holds the point.
    #[must_use]
    pub fn height_at(&self, x: f32, y: f32) -> Option<f32> {
        let (t, lx, ly) = self.locate(x, y)?;
        Some(sample_terrain(t, lx, ly))
    }

    /// Unit surface normal at `(x, y)` (`x` east, `y` north, `z` up); `None` when no tile holds
    /// the point. Central differences over one terrain cell, taken across tile borders when the
    /// neighbour is held.
    #[must_use]
    pub fn normal_at(&self, x: f32, y: f32) -> Option<Vec3> {
        self.height_at(x, y)?;
        let d = crate::terrain::terrain_cell();
        let h = |px: f32, py: f32| self.height_at(px, py).or_else(|| self.height_at(x, y));
        let (hx0, hx1) = (h(x - d, y)?, h(x + d, y)?);
        let (hy0, hy1) = (h(x, y - d)?, h(x, y + d)?);
        let (dzdx, dzdy) = ((hx1 - hx0) / (2.0 * d), (hy1 - hy0) / (2.0 * d));
        Some(Vec3::new(-dzdx, -dzdy, 1.0).normalize())
    }
}
