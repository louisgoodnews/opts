//! Hot reload: which parts of the loaded world depend on a changed file, and the cache and
//! bookkeeping changes that make the streamer rebuild them (see `docs/src/architecture/hot-reload.md`).
//! The planning of a reload is in [`plan`], the tile bookkeeping in [`bookkeeping`].

use bevy::prelude::*;

mod bookkeeping;
mod plan;

/// What the application has to do after [`WorldLoadJob::plan_reload`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReloadPlan {
    /// Tiles taken down (the streamer re-queues and reloads them); [`ReloadPlan::apply`] despawns
    /// their roots and unregisters them.
    pub unloads: Vec<TileUnload>,
    /// Tiles being rebuilt.
    pub tiles: usize,
    /// Textures overwritten in place.
    pub textures: usize,
    /// Meshes dropped from the cache.
    pub meshes: usize,
    /// The player vehicle depends on the file: restart the session.
    pub restart_vehicle: bool,
}

/// A tile taken out of the world: its root entity and what it registered (ground, collision,
/// lanes, signals). Built by `WorldLoadJob::unload_tile`, carried out by [`TileUnload::apply`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileUnload {
    /// Tile key.
    pub key: (i32, i32),
    /// Root entity of the tile.
    pub root: Entity,
    /// Keep the tile's ground and collision until the rebuilt tile registers its own (the tile
    /// the player stands on).
    pub keep_ground: bool,
}

impl TileUnload {
    /// Despawns the tile and unregisters it from every index.
    pub fn apply(&self, commands: &mut Commands) {
        let key = self.key;
        if let Ok(mut e) = commands.get_entity(self.root) {
            e.despawn();
        }
        super::pedestrians::unregister_tile(commands, key);
        super::rail::unregister_tile(commands, key);
        super::rail::unregister_bike_tile(commands, key);
        super::traffic::unregister_tile(commands, key);
        if !self.keep_ground {
            super::ground_height::unregister(commands, key);
            super::collision::unregister_tile(commands, key);
        }
        opts_bevy::scene_trace::forget_tile(key);
        tracing::debug!(tile = ?key, keep_ground = self.keep_ground, "tile unloaded");
    }
}

impl ReloadPlan {
    /// Carries out the unloads.
    pub fn apply(&self, world: &mut World) {
        let mut commands = world.commands();
        for u in &self.unloads {
            u.apply(&mut commands);
        }
        world.flush();
    }

    /// A readable summary (`2 tiles, 1 texture in place`), or `nothing depends on the file`.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.tiles > 0 {
            parts.push(format!("{} tile(s) rebuilt", self.tiles));
        }
        if self.textures > 0 {
            parts.push(format!("{} texture(s) updated in place", self.textures));
        }
        if self.meshes > 0 {
            parts.push(format!("{} mesh(es) re-read", self.meshes));
        }
        if self.restart_vehicle {
            parts.push("vehicle changed: session restarted".to_string());
        }
        if parts.is_empty() {
            "nothing loaded depends on this file".to_string()
        } else {
            parts.join(", ")
        }
    }
}
