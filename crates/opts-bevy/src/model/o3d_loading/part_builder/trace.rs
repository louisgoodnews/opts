//! The scene-trace notes of a slot's material (for the load trace only).

use super::*;

impl ContentLoader<'_> {
    pub(super) fn trace_slot_material(
        sc: &mut TraceScratch,
        slot: usize,
        tex_name: Option<&str>,
        material: &StandardMaterial,
        textured: bool,
    ) {
        if sc.lap.active() {
            sc.materials.push(crate::scene_trace::MaterialNote {
                slot,
                texture: tex_name.unwrap_or_default().to_string(),
                alpha: match material.alpha_mode {
                    AlphaMode::Blend => "blend",
                    AlphaMode::Mask(_) => "mask",
                    _ => "opaque",
                }
                .into(),
                textured,
            });
            if tex_name.is_some() && !textured {
                sc.warnings.push(format!(
                    "texture not found: {}",
                    tex_name.unwrap_or_default()
                ));
            }
        }
    }
}
