//! Layer (c), the merger of static parts: the groups of one tile and the merge rules.

use std::collections::HashMap;

use bevy::asset::{AssetId, Assets, Handle};
use bevy::mesh::{Mesh, PrimitiveTopology};
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

/// Vertices per merged mesh at most (keeps one entity's bounds and draw size reasonable).
pub(super) const MERGE_VERTEX_CAP: usize = 200_000;

#[derive(Default)]
pub(super) struct MergeGroup {
    pub(super) material: Option<Handle<StandardMaterial>>,
    pub(super) positions: Vec<[f32; 3]>,
    pub(super) normals: Vec<[f32; 3]>,
    pub(super) uvs: Vec<[f32; 2]>,
    pub(super) indices: Vec<u32>,
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
    pub(super) groups: HashMap<(AssetId<StandardMaterial>, i32, i32, usize), MergeGroup>,
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
}
