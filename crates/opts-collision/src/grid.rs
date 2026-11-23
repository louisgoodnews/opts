//! A spatial grid of static boxes: the broad phase of the collision queries.

use super::Obb;
use glam::Vec2;
use std::collections::HashMap;

/// Static boxes in a uniform grid; entries carry a key (the tile) so a tile's boxes can be
/// dropped together.
#[derive(Debug, Clone)]
pub struct ColliderGrid<K> {
    cell: f32,
    cells: HashMap<(i32, i32), Vec<(K, Obb)>>,
}

impl<K: Clone + PartialEq> ColliderGrid<K> {
    /// A grid with square cells of `cell` metres.
    #[must_use]
    pub fn new(cell: f32) -> Self {
        Self {
            cell,
            cells: HashMap::new(),
        }
    }

    fn index(&self, p: Vec2) -> (i32, i32) {
        (
            (p.x / self.cell).floor() as i32,
            (p.y / self.cell).floor() as i32,
        )
    }

    /// Adds a box to every cell its bounding circle touches.
    pub fn insert(&mut self, key: K, obb: Obb) {
        let r = obb.radius();
        let (lo, hi) = (
            self.index(obb.centre - Vec2::splat(r)),
            self.index(obb.centre + Vec2::splat(r)),
        );
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                self.cells
                    .entry((x, y))
                    .or_default()
                    .push((key.clone(), obb));
            }
        }
    }

    /// Drops every box inserted under `key`.
    pub fn remove(&mut self, key: &K) {
        self.cells.retain(|_, v| {
            v.retain(|(k, _)| k != key);
            !v.is_empty()
        });
    }

    /// The boxes in the cells within `radius` of `centre` (a box can appear in several cells).
    pub fn query(&self, centre: Vec2, radius: f32) -> impl Iterator<Item = &Obb> {
        let (lo, hi) = (
            self.index(centre - Vec2::splat(radius)),
            self.index(centre + Vec2::splat(radius)),
        );
        (lo.0..=hi.0)
            .flat_map(move |x| (lo.1..=hi.1).map(move |y| (x, y)))
            .filter_map(|c| self.cells.get(&c))
            .flatten()
            .map(|(_, o)| o)
    }

    /// Number of stored (box, cell) entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.values().map(Vec::len).sum()
    }

    /// Whether the grid is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
