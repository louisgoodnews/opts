//! Layer (c), baking: the parts of one tile are added and flushed into one mesh per group.

use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::MeshMaterial3d;
use bevy::pbr::StandardMaterial;
use bevy::transform::components::Transform;

use super::merge::*;
use super::merge_cell_m;

impl StaticMerger {
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
