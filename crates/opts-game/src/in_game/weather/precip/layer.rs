//! Writes the particle quads into the mesh and shows the layer with its material.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use opts_bevy::weather::PrecipPlan;
use opts_core::formats::weather::PrecipKind;

use super::geometry::{flake_corners, particle_position, rain_corners};
use super::state::{PrecipLayer, PrecipState};
use crate::in_game::types::InGameEntity;

/// Places the quads of the visible particles around the camera and updates the index range when
/// their number changed.
pub(super) fn write_geometry(
    state: &mut PrecipState,
    meshes: &mut Assets<Mesh>,
    mesh_handle: &Handle<Mesh>,
    plan: &PrecipPlan,
    (t, cam): (f64, &GlobalTransform),
) {
    let quads = (plan.quads as usize).min(state.allocated);
    let drawn_changed = state.drawn != quads;
    let (right, up, cam_pos) = (cam.right().as_vec3(), cam.up().as_vec3(), cam.translation());
    let seeds = std::mem::take(&mut state.seeds);
    let indices = std::mem::take(&mut state.indices);
    if let Some(mut mesh) = meshes.get_mut(mesh_handle) {
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for (q, seed) in seeds.iter().take(quads).enumerate() {
                let p = particle_position(*seed, t, plan, cam_pos);
                let corners = if plan.kind == PrecipKind::Rain {
                    rain_corners(p, plan, cam_pos, right)
                } else {
                    flake_corners(p, plan, right, up)
                };
                for (k, c) in corners.iter().enumerate() {
                    positions[q * 4 + k] = c.to_array();
                }
            }
        }
        if drawn_changed {
            mesh.insert_indices(Indices::U32(indices[..quads * 6].to_vec()));
        }
    }
    state.seeds = seeds;
    state.indices = indices;
    state.drawn = quads;
}

/// Spawns the precipitation layer when it does not exist, shows it and gives it the material of
/// the current kind of precipitation.
pub(super) fn sync_layer(
    commands: &mut Commands,
    layer: &mut Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<PrecipLayer>>,
    mesh_handle: &Handle<Mesh>,
    want_material: &Handle<StandardMaterial>,
) {
    if layer.is_empty() {
        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(want_material.clone()),
            Transform::IDENTITY,
            Visibility::Visible,
            NoFrustumCulling,
            NotShadowCaster,
            NotShadowReceiver,
            PrecipLayer,
            InGameEntity,
            Name::new("Precipitation"),
        ));
    }
    for (mut v, mut m) in layer {
        if *v != Visibility::Visible {
            *v = Visibility::Visible;
        }
        if m.0 != *want_material {
            m.0 = want_material.clone();
        }
    }
}
