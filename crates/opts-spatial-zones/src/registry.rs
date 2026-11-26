//! Zones per tile: the zone entities are built when a tile loads and dropped when it unloads.
//!
//! Tiles are keyed by their OMSI tile coordinates `(x, y)`, the same key the streaming code uses.
//! The registry remembers which entities belong to which tile, so an unload removes exactly the
//! zones that the load spawned and nothing else. The bookkeeping is the core's [`TileBook`]
//! (ISS-092); this resource issues the spawn and despawn commands.

use bevy::prelude::*;

use super::model::{SpatialZone, ZoneId};
use opts_spatial_zones_core::registry::TileBook;

pub use opts_spatial_zones_core::registry::TileKey;

/// The zone entities of every loaded tile.
#[derive(Resource, Debug, Default)]
pub struct ZoneRegistry {
    book: TileBook<Entity>,
}

impl ZoneRegistry {
    /// Spawns one zone entity per zone of `tile`, each with a fresh id. A tile that is already
    /// built is first dropped, so a double load does not duplicate zones. Returns the number spawned.
    pub fn build_tile(
        &mut self,
        commands: &mut Commands,
        tile: TileKey,
        zones: &[SpatialZone],
    ) -> usize {
        self.drop_tile(commands, tile);
        let entities: Vec<Entity> = zones
            .iter()
            .map(|zone| {
                let id: ZoneId = self.book.next_zone_id();
                commands.spawn(SpatialZone { id, ..*zone }).id()
            })
            .collect();
        let count = self.book.insert(tile, entities);
        tracing::debug!(tile = ?tile, zones = count, "spatial zones built for tile");
        count
    }

    /// Despawns the zones of `tile`. Returns the number of zones removed (0 for an unknown tile).
    pub fn drop_tile(&mut self, commands: &mut Commands, tile: TileKey) -> usize {
        let Some(entities) = self.book.remove(tile) else {
            return 0;
        };
        for entity in &entities {
            commands.entity(*entity).despawn();
        }
        entities.len()
    }

    /// Brings the registry in line with the resident tiles: drops the tiles that are no longer
    /// resident and builds the resident ones it does not hold yet, with the zones `zones_of`
    /// gives for them. Returns `(built, dropped)` tile counts.
    pub fn reconcile(
        &mut self,
        commands: &mut Commands,
        resident: &[TileKey],
        mut zones_of: impl FnMut(TileKey) -> Vec<SpatialZone>,
    ) -> (usize, usize) {
        let stale = self.book.stale(resident);
        for key in &stale {
            self.drop_tile(commands, *key);
        }
        let fresh = self.book.fresh(resident);
        for key in &fresh {
            self.build_tile(commands, *key, &zones_of(*key));
        }
        (fresh.len(), stale.len())
    }

    /// Whether `tile` has zones built.
    #[must_use]
    pub fn contains(&self, tile: TileKey) -> bool {
        self.book.contains(tile)
    }

    /// The number of tiles with zones built.
    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.book.tile_count()
    }

    /// The number of zone entities the registry owns.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.book.entity_count()
    }
}
