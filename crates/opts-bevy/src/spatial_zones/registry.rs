//! Zones per tile: the zone entities are built when a tile loads and dropped when it unloads.
//!
//! Tiles are keyed by their OMSI tile coordinates `(x, y)`, the same key the streaming code uses.
//! The registry remembers which entities belong to which tile, so an unload removes exactly the
//! zones that the load spawned and nothing else.

use std::collections::HashMap;

use bevy::prelude::*;

use super::model::SpatialZone;

/// The tile key: OMSI tile coordinates.
pub type TileKey = (i32, i32);

/// The zone entities of every loaded tile.
#[derive(Resource, Debug, Default)]
pub struct ZoneRegistry {
    tiles: HashMap<TileKey, Vec<Entity>>,
}

impl ZoneRegistry {
    /// Spawns one zone entity per zone of `tile`. A tile that is already built is first dropped,
    /// so a double load does not duplicate zones. Returns the number of zones spawned.
    pub fn build_tile(
        &mut self,
        commands: &mut Commands,
        tile: TileKey,
        zones: &[SpatialZone],
    ) -> usize {
        self.drop_tile(commands, tile);
        let entities: Vec<Entity> = zones
            .iter()
            .map(|zone| commands.spawn(*zone).id())
            .collect();
        let count = entities.len();
        self.tiles.insert(tile, entities);
        count
    }

    /// Despawns the zones of `tile`. Returns the number of zones removed (0 for an unknown tile).
    pub fn drop_tile(&mut self, commands: &mut Commands, tile: TileKey) -> usize {
        let Some(entities) = self.tiles.remove(&tile) else {
            return 0;
        };
        for entity in &entities {
            commands.entity(*entity).despawn();
        }
        entities.len()
    }

    /// Whether `tile` has zones built.
    #[must_use]
    pub fn contains(&self, tile: TileKey) -> bool {
        self.tiles.contains_key(&tile)
    }

    /// The number of tiles with zones built.
    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// The number of zone entities the registry owns.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.tiles.values().map(Vec::len).sum()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
