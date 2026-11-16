//! The mirror panes of every vehicle: the component each mirror mesh gets, and its registration
//! once the mesh appears.

use bevy::prelude::*;

use crate::mirrors::*;

/// A mirror pane of any vehicle (set once, when the mesh appears).
#[derive(Component, Debug, Clone)]
pub struct AmbientPane {
    /// Centre of the pane in the mesh's local space.
    pub centre: Vec3,
    /// Mean normal of the pane in the mesh's local space.
    pub normal: Vec3,
    /// Extent along the pane's up direction (local metres, before scale).
    pub height: f32,
    /// Width over height.
    pub aspect: f32,
    /// The pane's UV rectangle.
    pub uv_min: Vec2,
    /// The pane's UV rectangle.
    pub uv_max: Vec2,
    /// The pane's own static material.
    pub original: Handle<StandardMaterial>,
}

type NewAmbientPaneItem<'a> = (Entity, &'a MeshMaterial3d<StandardMaterial>, &'a Mesh3d);
type NewAmbientPaneFilter = (
    Added<MeshMaterial3d<StandardMaterial>>,
    Without<AmbientPane>,
);

/// Marks new mirror meshes.
pub fn register_ambient_panes(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    new: Query<NewAmbientPaneItem, NewAmbientPaneFilter>,
) {
    for (entity, material, mesh) in &new {
        if surface_kind(material.0.id()) != Some(SurfaceKind::Mirror) {
            continue;
        }
        let Some(mesh) = meshes.get(&mesh.0) else {
            continue;
        };
        let (Some(pos), Some(nor)) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3()),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                .and_then(|a| a.as_float3()),
        ) else {
            continue;
        };
        let Some((centre, normal, height, aspect)) = pane_geometry(pos, nor) else {
            continue;
        };
        let (uv_min, uv_max) = mesh
            .attribute(Mesh::ATTRIBUTE_UV_0)
            .and_then(|a| match a {
                bevy::mesh::VertexAttributeValues::Float32x2(v) => uv_bounds(v),
                _ => None,
            })
            .unwrap_or((Vec2::ZERO, Vec2::ONE));
        commands.entity(entity).insert(AmbientPane {
            centre,
            normal,
            height,
            aspect,
            uv_min,
            uv_max,
            original: material.0.clone(),
        });
    }
}

pub(super) type AmbientPaneItem<'a> = (
    Entity,
    &'a AmbientPane,
    &'a GlobalTransform,
    Option<&'a InheritedVisibility>,
);
