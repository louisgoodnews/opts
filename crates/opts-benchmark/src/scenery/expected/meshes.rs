//! Reading the mesh files of an object and the `[matl]` overrides of a mesh.

use super::*;

/// Parses a mesh file and resolves keyed models; unresolved keyed models are dropped.
pub(super) fn read_models(path: &Path, keyed: &mut KeyedSummary) -> Result<Vec<O3d>, ()> {
    let bytes = std::fs::read(path).map_err(|_| ())?;
    let mut models = opts_bevy::mesh::parse_mesh_file(path, &bytes).map_err(|_| ())?;
    let unreadable = |ms: &[O3d]| {
        ms.iter()
            .any(|m| matches!(m.vertex_layout(), VertexLayout::Unreadable))
    };
    if !unreadable(&models) {
        return Ok(models);
    }
    keyed.total += 1;
    for m in models.iter_mut() {
        if m.encryption_key.is_some() {
            opts_core::formats::o3d::exact::decode(m);
        }
    }
    if unreadable(&models) {
        keyed.unresolved += 1;
        models.retain(|m| !matches!(m.vertex_layout(), VertexLayout::Unreadable));
    } else {
        keyed.decoded += 1;
    }
    Ok(models)
}

/// The `[matl]` overrides of a mesh, as the tile loader builds them (text images left out).
pub(super) fn overrides(sco: &SceneryObject, mesh: &ScoMesh) -> Vec<(TextureOverride, bool)> {
    mesh.materials
        .iter()
        .map(|m| {
            let text = m
                .text_texture
                .is_some_and(|i| sco.text_textures.get(i).is_some());
            (
                TextureOverride {
                    slot: m.material_index,
                    texture: m.texture.clone(),
                    alpha: m.alpha_mode,
                    alpha_variable: m.alpha_scale.clone(),
                    trans_map: None,
                    image: None,
                    light_map: None,
                    free_texture: None,
                    night_map: None,
                },
                text,
            )
        })
        .collect()
}
