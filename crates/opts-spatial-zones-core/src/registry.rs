//! The per-tile bookkeeping of the zones: which handles belong to which tile, and the next zone
//! id. Generic over the handle type, so the core needs no engine entity type; the facade
//! `ZoneRegistry` instantiates it with `Entity` and issues the spawn and despawn commands (ISS-092).

use std::collections::HashMap;

use crate::model::ZoneId;

/// The tile key: OMSI tile coordinates.
pub type TileKey = (i32, i32);

/// The handles of every loaded tile, and the next zone id.
#[derive(Debug)]
pub struct TileBook<E> {
    tiles: HashMap<TileKey, Vec<E>>,
    /// The next zone id. Ids are unique across the resident tiles, not per tile (ISS-012).
    next_id: u32,
}

impl<E> Default for TileBook<E> {
    fn default() -> Self {
        Self {
            tiles: HashMap::new(),
            next_id: 0,
        }
    }
}

impl<E> TileBook<E> {
    /// The next zone id, which then advances.
    pub fn next_zone_id(&mut self) -> ZoneId {
        let id = ZoneId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Records the handles of `tile`, replacing any held. Returns the number recorded.
    pub fn insert(&mut self, tile: TileKey, handles: Vec<E>) -> usize {
        let count = handles.len();
        self.tiles.insert(tile, handles);
        count
    }

    /// Forgets `tile`, returning its handles when it was held.
    pub fn remove(&mut self, tile: TileKey) -> Option<Vec<E>> {
        self.tiles.remove(&tile)
    }

    /// Whether `tile` is held.
    #[must_use]
    pub fn contains(&self, tile: TileKey) -> bool {
        self.tiles.contains_key(&tile)
    }

    /// The held tiles that are not in `resident`.
    #[must_use]
    pub fn stale(&self, resident: &[TileKey]) -> Vec<TileKey> {
        self.tiles
            .keys()
            .filter(|key| !resident.contains(key))
            .copied()
            .collect()
    }

    /// The resident tiles that are not held yet.
    #[must_use]
    pub fn fresh(&self, resident: &[TileKey]) -> Vec<TileKey> {
        resident
            .iter()
            .filter(|key| !self.tiles.contains_key(key))
            .copied()
            .collect()
    }

    /// The number of tiles held.
    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// The number of handles held, over all tiles.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.tiles.values().map(Vec::len).sum()
    }
}
