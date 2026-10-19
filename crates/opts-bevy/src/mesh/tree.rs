//! Tree billboards: the quad mesh and its material.

use super::*;

/// Builds a crossed-quad billboard for a `[tree]` object: two perpendicular vertical quads,
/// `width` metres wide and `height` metres tall, standing on the origin.
///
/// Normals are crown normals: per vertex `normalize(outward·0.6 + up·0.8)`, where "outward" is
/// the horizontal direction from the trunk to the vertex. Both quads of the cross therefore shade
/// the same from every side (no flat-card halves), and the light falls off gently towards the
/// side facing away from the sun like a round crown. Pair it with [`tree_billboard_material`].
#[must_use]
pub fn tree_billboard_mesh(height: f32, width: f32) -> Mesh {
    let hw = width * 0.5;
    let positions = vec![
        [-hw, 0.0, 0.0],
        [hw, 0.0, 0.0],
        [hw, height, 0.0],
        [-hw, height, 0.0],
        [0.0, 0.0, -hw],
        [0.0, 0.0, hw],
        [0.0, height, hw],
        [0.0, height, -hw],
    ];
    let uvs = vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let normals: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| {
            let outward = Vec3::new(p[0], 0.0, p[2]).normalize_or_zero();
            (outward * 0.6 + Vec3::Y * 0.8).normalize().to_array()
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]));
    mesh
}

/// Material for a [`tree_billboard_mesh`]: alpha-tested foliage drawn from both sides.
///
/// `cull_mode: None` draws the back faces, but `double_sided` stays `false` so Bevy does **not**
/// flip the normal on them: the crown normal is the same seen from either side, so a quad is
/// not dark from one side and pale (grazing retro-reflection) from the other. `reflectance` 0
/// removes the Fresnel specular sheen that made foliage look like a translucent sheet.
#[must_use]
pub fn tree_billboard_material(
    texture: bevy::asset::Handle<bevy::image::Image>,
) -> bevy::pbr::StandardMaterial {
    use bevy::prelude::AlphaMode;
    bevy::pbr::StandardMaterial {
        base_color_texture: Some(texture),
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: false,
        cull_mode: None,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..Default::default()
    }
}
