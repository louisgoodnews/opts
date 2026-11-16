//! Converts an [`O3d`] into one Bevy [`Mesh`] per material slot.

use std::collections::{BTreeMap, HashMap};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::{skin_table_slots, smooth_normal_mesh, O3d, APPLY_O3D_MATRIX};
mod attrs;
mod builder;

use builder::SubmeshBuilder;

fn collect_material_groups(o3d: &O3d) -> BTreeMap<usize, Vec<[u32; 3]>> {
    let mut groups: BTreeMap<usize, Vec<[u32; 3]>> = BTreeMap::new();
    for (i, tri) in o3d.indices.iter().enumerate() {
        if tri.iter().any(|&vi| {
            o3d.vertices
                .get(vi as usize)
                .is_some_and(|v| !v.position.is_finite() || !v.uv.is_finite())
        }) {
            continue;
        }
        let slot = o3d.face_materials.get(i).copied().unwrap_or(0) as usize;
        groups.entry(slot).or_default().push(*tri);
    }
    groups
}

/// [`o3d_to_bevy_meshes_by_material`](super::o3d_to_bevy_meshes_by_material) with `[setbone]`
/// joint numbers for the skin (see [`skin_table_slots`]).
#[must_use]
pub fn o3d_to_bevy_meshes_by_material_slots(
    o3d: &O3d,
    slots: Option<&[(String, u16)]>,
) -> Vec<(usize, Mesh)> {
    let groups = collect_material_groups(o3d);
    let m = if APPLY_O3D_MATRIX || o3d.version == 0 {
        o3d.transform
    } else {
        None
    }
    .unwrap_or([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    let xf_pos = Box::new(move |p: bevy::math::Vec3| {
        bevy::math::Vec3::new(
            p.x * m[0][0] + p.y * m[1][0] + p.z * m[2][0] + m[3][0],
            p.x * m[0][1] + p.y * m[1][1] + p.z * m[2][1] + m[3][1],
            p.x * m[0][2] + p.y * m[1][2] + p.z * m[2][2] + m[3][2],
        )
    });
    let xf_dir = Box::new(move |n: bevy::math::Vec3| {
        bevy::math::Vec3::new(
            n.x * m[0][0] + n.y * m[1][0] + n.z * m[2][0],
            n.x * m[0][1] + n.y * m[1][1] + n.z * m[2][1],
            n.x * m[0][2] + n.y * m[1][2] + n.z * m[2][2],
        )
        .normalize_or_zero()
    });
    let det = bevy::math::Mat3::from_cols(
        bevy::math::Vec3::new(m[0][0], m[0][1], m[0][2]),
        bevy::math::Vec3::new(m[1][0], m[1][1], m[1][2]),
        bevy::math::Vec3::new(m[2][0], m[2][1], m[2][2]),
    )
    .determinant();

    let layout = o3d.vertex_layout();
    let cycled = matches!(layout, opts_core::formats::o3d::VertexLayout::AxisCycled);
    let recompute = cycled
        || matches!(
            layout,
            opts_core::formats::o3d::VertexLayout::RecomputedNormals
                | opts_core::formats::o3d::VertexLayout::Unreadable
        );
    let skin = skin_table_slots(o3d, slots);
    let builder = SubmeshBuilder {
        o3d,
        xf_pos,
        xf_dir,
        mirrored: det < 0.0,
        cycled,
        recompute,
        skin: &skin,
    };

    let mut out = Vec::with_capacity(groups.len());
    for (slot, tris) in groups {
        if let Some(mesh) = builder.build_submesh(slot, &tris) {
            out.push((slot, mesh));
        }
    }
    out
}
