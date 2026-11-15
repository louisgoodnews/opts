//! Scanning the player vehicle for mirror panes and grouping them.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::glass::{surface_kind, SurfaceKind};
use crate::mirrors::*;

/// What a scan learned about one pane mesh.
pub(super) struct Found {
    pub(super) member: MirrorMember,
    pub(super) uv_min: Vec2,
    pub(super) uv_max: Vec2,
}

pub(super) type CandidateMeshQuery<'w, 's> = Query<
    'w,
    's,
    (&'static MeshMaterial3d<StandardMaterial>, &'static Mesh3d),
    Without<MirrorSurface>,
>;
pub(super) fn scan_vehicle_panes(
    vehicle: Entity,
    children: &Query<&Children>,
    candidates: &CandidateMeshQuery,
    meshes: &Assets<Mesh>,
) -> HashMap<GroupKey, (Handle<StandardMaterial>, Vec<Found>)> {
    let mut groups: HashMap<GroupKey, (Handle<StandardMaterial>, Vec<Found>)> = HashMap::new();
    for entity in children.iter_descendants(vehicle) {
        let Ok((material, mesh)) = candidates.get(entity) else {
            continue;
        };
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
        groups
            .entry(GroupKey::of(material.0.id()))
            .or_insert_with(|| (material.0.clone(), Vec::new()))
            .1
            .push(Found {
                member: MirrorMember {
                    entity,
                    centre,
                    normal,
                    height,
                    aspect,
                    original: material.0.clone(),
                },
                uv_min,
                uv_max,
            });
    }
    groups
}
