//! Texture overrides of a scenery object's `[mesh]`: textures, alpha, light maps and text textures.

use super::super::*;
use opts_core::formats::scenery::SceneryObject;

/// The texture overrides of one `[mesh]` of a scenery object: the textures, alpha and light maps
/// of its materials, and the rendered text of its text textures.
pub(super) fn texture_overrides(
    job: &mut WorldLoadJob,
    images: &mut Assets<Image>,
    (sco, mesh_def): (&SceneryObject, &opts_core::formats::scenery::ScoMesh),
    (sco_path, obj, key, string_names): (&Path, &MapObjectInstance, (i32, i32), &[String]),
) -> Vec<TextureOverride> {
    mesh_def
        .materials
        .iter()
        .map(|m| TextureOverride {
            clamp: false,
            slot: m.material_index,
            texture: m.texture.clone(),
            alpha: m.alpha_mode,
            alpha_variable: m.alpha_scale.clone(),
            trans_map: None,
            light_map: super::super::super::scenery_objects::light_map_of(m),
            night_map: m.night_map.clone().filter(|t| !t.trim().is_empty()),
            free_texture: None,
            image: m.text_texture.and_then(|i| {
                let tt = sco.text_textures.get(i)?;
                let Some(text) = resolve_text(
                    tt.string_index,
                    tt.string_var.as_deref(),
                    &obj.string_vars,
                    string_names,
                ) else {
                    tracing::error!(at = %opts_bevy::position_tag(),
                        sco = %sco_path.display(),
                        source = ?tt.string_var.as_deref().unwrap_or("by number"),
                        index = tt.string_index,
                        "text texture has no string on this object instance"
                    );
                    return None;
                };
                let spec = TextSpec {
                    font: tt.font_name.clone(),
                    text,
                    width: tt.width,
                    height: tt.height,
                    mono: tt.color_mode == 0,
                    rgb: tt.text_color,
                    orientation: tt.orientation,
                };
                tracing::debug!(
                    sco = %sco_path.display(),
                    text = %spec.text,
                    tile = ?key,
                    pos = ?obj.position,
                    "text texture"
                );
                job.fonts.render(&job.install, images, &spec)
            }),
        })
        .collect()
}
