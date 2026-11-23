//! The static collider index: the boxes of the resident tiles, in a grid of cells.

use bevy::prelude::*;
use opts_collision::{ColliderGrid, Obb};

/// Cell size of the static index (m).
const CELL_M: f32 = 16.0;
/// The static colliders of the resident tiles, in the session frame (OMSI axes).
#[derive(Resource)]
pub struct CollisionIndex {
    pub(super) grid: ColliderGrid<(i32, i32)>,
}

impl Default for CollisionIndex {
    fn default() -> Self {
        Self {
            grid: ColliderGrid::new(CELL_M),
        }
    }
}

impl CollisionIndex {
    /// Number of stored entries (for diagnostics).
    #[must_use]
    pub fn len(&self) -> usize {
        self.grid.len()
    }

    /// Whether nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }

    /// Registers a static box under the key of its tile (tests and tools; tiles register their
    /// objects themselves).
    pub fn insert_box(&mut self, tile: (i32, i32), body: Obb) {
        self.grid.insert(tile, body);
    }
}
