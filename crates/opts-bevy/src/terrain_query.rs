//! Ground elevation queries over the terrain of several tiles (M-0025 `GroundElevationQuery`).
//!
//! Heights are those of the drawn ground ([`TileTerrain::height`]: the two triangles of each
//! cell) of the tile that contains the point; normals come from the central differences of the
//! same surface, so both agree with each other and with the mesh. Specification:
//! `docs/src/architecture/terrain-rendering.md`, section 5.

use std::collections::HashMap;

use bevy::math::{DVec2, Vec3};
use opts_core::formats::map::TileGrid;

use crate::terrain::TileTerrain;

/// Terrain of the resident tiles, queried by absolute position.
///
/// Positions are measured from the south-west corner of tile `(0, 0)` (`x` east, `y` north) on the
/// map's [`TileGrid`] (`TileGrid::absolute`); a caller that uses a floating origin adds the
/// origin of its start tile first.
#[derive(Debug, Clone, Default)]
pub struct GroundElevationQuery {
    grid: TileGrid,
    tiles: HashMap<(i32, i32), TileTerrain>,
}

impl GroundElevationQuery {
    /// An empty query over `grid`.
    #[must_use]
    pub fn new(grid: TileGrid) -> Self {
        Self {
            grid,
            tiles: HashMap::new(),
        }
    }

    /// Adds or replaces the terrain of tile `key`.
    pub fn insert(&mut self, key: (i32, i32), terrain: TileTerrain) {
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

    fn locate(&self, x: f32, y: f32) -> Option<(&TileTerrain, f32, f32)> {
        let (key, local) = self
            .grid
            .tile_at(DVec2::new(f64::from(x), f64::from(y)), (0, 0));
        let t = self.tiles.get(&key)?;
        Some((t, local.x as f32, local.y as f32))
    }

    /// Terrain height at `(x, y)`; `None` when no tile holds the point.
    #[must_use]
    pub fn height_at(&self, x: f32, y: f32) -> Option<f32> {
        let (t, lx, ly) = self.locate(x, y)?;
        Some(t.height(lx, ly))
    }

    /// Unit surface normal at `(x, y)` (`x` east, `y` north, `z` up); `None` when no tile holds
    /// the point. Central differences over one terrain cell, taken across tile borders when the
    /// neighbour is held.
    #[must_use]
    pub fn normal_at(&self, x: f32, y: f32) -> Option<Vec3> {
        let (t, _, _) = self.locate(x, y)?;
        let d = t.cell;
        let h = |px: f32, py: f32| self.height_at(px, py).or_else(|| self.height_at(x, y));
        let (hx0, hx1) = (h(x - d, y)?, h(x + d, y)?);
        let (hy0, hy1) = (h(x, y - d)?, h(x, y + d)?);
        let (dzdx, dzdy) = ((hx1 - hx0) / (2.0 * d), (hy1 - hy0) / (2.0 * d));
        Some(Vec3::new(-dzdx, -dzdy, 1.0).normalize())
    }
}
