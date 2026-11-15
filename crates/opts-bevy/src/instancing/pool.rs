//! The asset pool of layer (a): one handle per distinct mesh and material content.

use std::collections::HashMap;

use bevy::asset::{Assets, Handle};
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;

use super::{dedupe, material_key, mesh_key};

/// Counters of the pool.
#[derive(Debug, Clone, Copy, Default)]
pub struct PoolStats {
    /// Meshes requested / found equal to an earlier one.
    pub meshes: (usize, usize),
    /// Materials requested / found equal to an earlier one.
    pub materials: (usize, usize),
}

/// Hands out one handle per distinct mesh and material content (layer (a) of the prototype).
///
/// With [`dedupe`] off every call adds a new asset, as the loader did before.
#[derive(Default)]
pub struct AssetPool {
    meshes: HashMap<(u64, u64), Handle<Mesh>>,
    materials: HashMap<(u64, u64), Handle<StandardMaterial>>,
    /// Hit counters.
    pub stats: PoolStats,
}

impl AssetPool {
    /// The shared handle of a mesh with this content (adds it on first sight).
    pub fn mesh(&mut self, store: &mut Assets<Mesh>, mesh: Mesh) -> Handle<Mesh> {
        self.stats.meshes.0 += 1;
        if !dedupe() {
            return store.add(mesh);
        }
        let key = mesh_key(&mesh);
        if let Some(h) = self.meshes.get(&key) {
            self.stats.meshes.1 += 1;
            return h.clone();
        }
        let h = store.add(mesh);
        self.meshes.insert(key, h.clone());
        h
    }

    /// The shared handle of a material with this content (adds it on first sight).
    pub fn material(
        &mut self,
        store: &mut Assets<StandardMaterial>,
        material: StandardMaterial,
    ) -> Handle<StandardMaterial> {
        self.stats.materials.0 += 1;
        if !dedupe() {
            return store.add(material);
        }
        let key = material_key(&material);
        if let Some(h) = self.materials.get(&key) {
            self.stats.materials.1 += 1;
            return h.clone();
        }
        let h = store.add(material);
        self.materials.insert(key, h.clone());
        h
    }
}
