//! Switch for the mesh-instancing prototype layers (M-0082,
//! `docs/src/architecture/instancing-research.md`).
//!
//! `OPTS_INSTANCING` selects the layer for the whole run (read once):
//!
//! | Value | Layer |
//! | :--- | :--- |
//! | unset, `off` | baseline: the loader as before |
//! | `dedupe` | (a) materials and meshes with equal content share one asset handle |
//! | `merge` | (a) plus (c) static scenery baked into one mesh per (material, 64 m cell, tile) |
//!
//! Layer (b), Bevy's automatic batching, is always on; the prototype only measures it.

use std::sync::OnceLock;

/// The active prototype layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstancingMode {
    /// The loader as before.
    Off,
    /// Equal materials and meshes share handles.
    Dedupe,
    /// De-duplication plus static merging: one mesh per (material, cell) and tile.
    Merge,
}

/// The mode from `OPTS_INSTANCING` (read once).
#[must_use]
pub fn mode() -> InstancingMode {
    static MODE: OnceLock<InstancingMode> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("OPTS_INSTANCING").as_deref() {
        Ok("dedupe") => InstancingMode::Dedupe,
        Ok("merge") => InstancingMode::Merge,
        _ => InstancingMode::Off,
    })
}

/// Whether handles are shared by content.
#[must_use]
pub fn dedupe() -> bool {
    mode() != InstancingMode::Off
}

/// Name of the active mode, for `render.stats`.
#[must_use]
pub fn mode_name() -> &'static str {
    match mode() {
        InstancingMode::Off => "off",
        InstancingMode::Dedupe => "dedupe",
        InstancingMode::Merge => "merge",
    }
}

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bevy::asset::{Assets, Handle};
use bevy::mesh::{Indices, Mesh};
use bevy::pbr::StandardMaterial;

/// Hash pair of arbitrary bytes (two differently seeded 64-bit hashes: a collision needs both).
fn hash_pair(parts: &[&[u8]]) -> (u64, u64) {
    let mut a = DefaultHasher::new();
    let mut b = DefaultHasher::new();
    0x9e37_79b9_7f4a_7c15_u64.hash(&mut b);
    for p in parts {
        p.hash(&mut a);
        p.hash(&mut b);
    }
    (a.finish(), b.finish())
}

/// Content key of a mesh: topology, every attribute's bytes and the indices.
#[must_use]
pub fn mesh_key(mesh: &Mesh) -> (u64, u64) {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    chunks.push(format!("{:?}", mesh.primitive_topology()).into_bytes());
    let mut attrs: Vec<_> = mesh.attributes().collect();
    attrs.sort_by_key(|(a, _)| a.id);
    for (attr, values) in attrs {
        chunks.push(format!("{}:{:?}", attr.name, attr.format).into_bytes());
        chunks.push(values.get_bytes().to_vec());
    }
    match mesh.indices() {
        Some(Indices::U16(v)) => chunks.push(v.iter().flat_map(|i| i.to_le_bytes()).collect()),
        Some(Indices::U32(v)) => chunks.push(v.iter().flat_map(|i| i.to_le_bytes()).collect()),
        None => {}
    }
    let refs: Vec<&[u8]> = chunks.iter().map(Vec::as_slice).collect();
    hash_pair(&refs)
}

/// Content key of a material: its full `Debug` form (colours, texture handles, alpha mode, culling).
#[must_use]
pub fn material_key(material: &StandardMaterial) -> (u64, u64) {
    hash_pair(&[format!("{material:?}").as_bytes()])
}

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

/// Whether static scenery is merged.
#[must_use]
pub fn merge() -> bool {
    mode() == InstancingMode::Merge
}

/// Edge of a merge cell in metres (`OPTS_MERGE_CELL`, default 64).
#[must_use]
pub fn merge_cell_m() -> f32 {
    static CELL: OnceLock<f32> = OnceLock::new();
    *CELL.get_or_init(|| {
        std::env::var("OPTS_MERGE_CELL")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|v: &f32| *v > 1.0)
            .unwrap_or(64.0)
    })
}

use bevy::asset::AssetId;
use bevy::asset::RenderAssetUsages;
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::mesh::{Mesh3d, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::AlphaMode;
use bevy::transform::components::Transform;

/// Vertices per merged mesh at most (keeps one entity's bounds and draw size reasonable).
const MERGE_VERTEX_CAP: usize = 200_000;

#[derive(Default)]
struct MergeGroup {
    material: Option<Handle<StandardMaterial>>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

/// Counters of the merger.
#[derive(Debug, Clone, Copy, Default)]
pub struct MergeStats {
    /// Parts baked into merged meshes.
    pub parts_merged: usize,
    /// Merged mesh entities spawned.
    pub entities: usize,
    /// Parts refused (blended, unusual vertex layout).
    pub refused: usize,
    /// Vertices written into merged meshes.
    pub vertices: usize,
}

/// Collects the static parts of one tile and bakes them into few meshes (layer (c) of the prototype).
///
/// A part can be merged when its material is not blended (blended parts are sorted per entity) and
/// its mesh has exactly positions, normals, uv0 and indices. The caller applies the object's
/// world transform; scale is assumed to be one (OMSI placements have none).
#[derive(Default)]
pub struct StaticMerger {
    groups: HashMap<(AssetId<StandardMaterial>, i32, i32, usize), MergeGroup>,
    /// Counters.
    pub stats: MergeStats,
}

impl StaticMerger {
    /// Whether a mesh has the layout the merger can bake (triangle list with indices and exactly
    /// position, normal and uv0).
    #[must_use]
    pub fn mesh_ok(m: &Mesh) -> bool {
        m.primitive_topology() == PrimitiveTopology::TriangleList
            && m.indices().is_some()
            && m.attributes().count() == 3
            && m.attribute(Mesh::ATTRIBUTE_POSITION).is_some()
            && m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some()
            && m.attribute(Mesh::ATTRIBUTE_UV_0).is_some()
    }

    /// Whether a material can be merged: blended materials are sorted per entity and stay apart.
    #[must_use]
    pub fn material_ok(mat: &StandardMaterial) -> bool {
        !matches!(
            mat.alpha_mode,
            AlphaMode::Blend | AlphaMode::Premultiplied | AlphaMode::Add | AlphaMode::Multiply
        )
    }

    /// Whether this part can be merged.
    #[must_use]
    pub fn can_merge(
        meshes: &Assets<Mesh>,
        materials: &Assets<StandardMaterial>,
        mesh: &Handle<Mesh>,
        material: &Handle<StandardMaterial>,
    ) -> bool {
        match (meshes.get(mesh), materials.get(material)) {
            (Some(m), Some(mat)) => Self::mesh_ok(m) && Self::material_ok(mat),
            _ => false,
        }
    }

    /// Bakes one stored part, placed with `xf`. Returns `false` (nothing added) when it cannot be
    /// merged.
    pub fn add(
        &mut self,
        meshes: &Assets<Mesh>,
        materials: &Assets<StandardMaterial>,
        mesh: &Handle<Mesh>,
        material: &Handle<StandardMaterial>,
        xf: &Transform,
    ) -> bool {
        if !Self::can_merge(meshes, materials, mesh, material) {
            self.stats.refused += 1;
            return false;
        }
        match meshes.get(mesh) {
            Some(m) => self.bake(m, material, xf),
            None => false,
        }
    }

    /// Bakes a mesh that is not in an asset store (`mesh_ok` and `material_ok` already checked).
    pub fn bake(&mut self, m: &Mesh, material: &Handle<StandardMaterial>, xf: &Transform) -> bool {
        let (
            Some(VertexAttributeValues::Float32x3(pos)),
            Some(VertexAttributeValues::Float32x3(nor)),
            Some(VertexAttributeValues::Float32x2(uv)),
        ) = (
            m.attribute(Mesh::ATTRIBUTE_POSITION),
            m.attribute(Mesh::ATTRIBUTE_NORMAL),
            m.attribute(Mesh::ATTRIBUTE_UV_0),
        )
        else {
            self.stats.refused += 1;
            return false;
        };
        let cell = merge_cell_m();
        // The cell of the part's first vertex (objects are small against the cell; ribbons are
        // already in world space, so the placement transform alone would put them all in cell 0).
        let anchor = pos
            .first()
            .map_or(xf.translation, |p| xf.transform_point((*p).into()));
        let cx = (anchor.x / cell).floor() as i32;
        let cz = (anchor.z / cell).floor() as i32;
        let mut chunk = 0;
        loop {
            let g = self
                .groups
                .entry((material.id(), cx, cz, chunk))
                .or_default();
            if g.positions.len() + pos.len() <= MERGE_VERTEX_CAP || g.positions.is_empty() {
                g.material.get_or_insert_with(|| material.clone());
                let base = g.positions.len() as u32;
                g.positions.extend(
                    pos.iter()
                        .map(|p| xf.transform_point((*p).into()).to_array()),
                );
                g.normals.extend(
                    nor.iter()
                        .map(|n| (xf.rotation * bevy::math::Vec3::from(*n)).to_array()),
                );
                g.uvs.extend_from_slice(uv);
                match m.indices() {
                    Some(Indices::U16(v)) => {
                        g.indices.extend(v.iter().map(|i| base + u32::from(*i)))
                    }
                    Some(Indices::U32(v)) => g.indices.extend(v.iter().map(|i| base + *i)),
                    None => {}
                }
                self.stats.parts_merged += 1;
                self.stats.vertices += pos.len();
                return true;
            }
            chunk += 1;
        }
    }

    /// Builds the merged meshes, spawns one entity per group and returns them (for the tile root).
    pub fn flush(&mut self, commands: &mut Commands, meshes: &mut Assets<Mesh>) -> Vec<Entity> {
        let mut out = Vec::new();
        let mut groups: Vec<_> = self.groups.drain().collect();
        groups.sort_by_key(|(k, _)| (k.1, k.2, k.3));
        for (_, g) in groups {
            let Some(material) = g.material else { continue };
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, g.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, g.normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, g.uvs);
            mesh.insert_indices(Indices::U32(g.indices));
            let e = commands
                .spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material),
                    Name::new("Merged"),
                ))
                .id();
            out.push(e);
            self.stats.entities += 1;
        }
        out
    }
}
