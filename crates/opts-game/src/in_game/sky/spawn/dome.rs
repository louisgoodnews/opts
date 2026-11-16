//! The sky dome: the UV-sphere with the sky texture on it, and the directional sunlight.

use bevy::prelude::*;

use crate::in_game::types::InGameEntity;

use super::super::{SkyDome, SunLight, DOME_RADIUS};

/// Builds the dome: the UV-sphere's `u` is kept, `v` is rewritten to run 0 (zenith) → 1 (horizon).
pub(super) fn dome_mesh() -> Mesh {
    let mut mesh = Sphere::new(DOME_RADIUS).mesh().uv(48, 24);
    let uvs: Option<Vec<[f32; 2]>> = {
        use bevy::mesh::VertexAttributeValues as V;
        match (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_UV_0),
        ) {
            (Some(V::Float32x3(p)), Some(V::Float32x2(uv))) => Some(
                p.iter()
                    .zip(uv)
                    .map(|(p, uv)| {
                        let el = (p[1] / DOME_RADIUS).clamp(-1.0, 1.0).asin();
                        [
                            uv[0],
                            (1.0 - el / std::f32::consts::FRAC_PI_2).clamp(0.0, 1.0),
                        ]
                    })
                    .collect(),
            ),
            _ => None,
        }
    };
    if let Some(uvs) = uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    mesh
}

/// The dome with the sky texture and the directional sunlight.
pub(super) fn spawn_dome_and_sun(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Handle<Image>,
) {
    commands.spawn((
        (
            Mesh3d(meshes.add(dome_mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(image),
                unlit: true,
                cull_mode: Some(bevy::render::render_resource::Face::Front),
                fog_enabled: false,
                ..default()
            })),
        ),
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        SkyDome,
        InGameEntity,
        Name::new("Sky_Dome"),
    ));
    commands.spawn((
        (DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        }),
        SunLight,
        opts_bevy::sun_rays::LightRole::Sun,
        InGameEntity,
        Name::new("Sun_Light"),
    ));
}
