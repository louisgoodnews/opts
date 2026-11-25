//! The material of a slot: its textures, the glass split and the material's own settings.

use super::*;

impl ContentLoader<'_> {
    pub(super) fn resolve_slot_textures(
        &mut self,
        base: &o3d::Material,
        over: Option<&TextureOverride>,
        ctx: &SlotBuildCtx<'_>,
    ) -> ResolvedTextures {
        let (tex_name, mut tex) = self.resolve_slot_texture_initial(base, over, ctx);
        // The missing-texture sentinel looks at the diffuse texture only, not the maps after it.
        let failures_after_diffuse = self.cache.stats.failures.len();
        let (mut glass_tex, blended_by_map) =
            self.compose_transmap_glass(&mut tex, tex_name.as_deref(), over, ctx);
        if glass_tex.is_none() {
            glass_tex = self.resolve_body_glass_split(&tex, over, blended_by_map);
        }
        if !crate::material::uses_texture_alpha(over.and_then(|o| o.alpha)) && !blended_by_map {
            let _t = opts_logging::stream_trace::span("o3d.opaque_variant");
            tex = tex.map(|h| self.opaque_variant(&h));
        }
        ResolvedTextures {
            tex_name,
            tex,
            glass_tex,
            blended_by_map,
            failures_after_diffuse,
        }
    }

    pub(super) fn configure_slot_material(
        &mut self,
        base: &o3d::Material,
        over: Option<&TextureOverride>,
        tex: Option<Handle<Image>>,
        blended_by_map: bool,
    ) -> StandardMaterial {
        let textured = tex.is_some();
        let mut material = o3d_material_to_standard_material(base, tex);
        if textured {
            material.base_color = bevy::color::Color::srgba(
                1.0,
                1.0,
                1.0,
                crate::material::effective_diffuse_alpha(base.diffuse[3]),
            );
        }
        if over
            .and_then(|o| o.trans_map.as_deref())
            .is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("\\s:"))
        {
            material.unlit = true;
            material.depth_bias = 10.0;
            material.base_color = bevy::color::Color::WHITE;
        }
        if let Some(mode) = over
            .and_then(|o| o.alpha)
            .filter(|m| crate::material::uses_texture_alpha(Some(*m)))
        {
            material.alpha_mode = if mode == 2 {
                AlphaMode::Blend
            } else {
                AlphaMode::Mask(0.5)
            };
        }
        if blended_by_map && over.and_then(|o| o.alpha).is_none() {
            material.alpha_mode = AlphaMode::Blend;
        }
        material.cull_mode = Some(bevy::render::render_resource::Face::Back);
        material.double_sided = false;
        material
    }

    pub(super) fn build_glass_material(
        material: &mut StandardMaterial,
        glass_tex: Option<Handle<Image>>,
    ) -> Option<StandardMaterial> {
        glass_tex.map(|g| {
            let mut glass = material.clone();
            if glass.emissive_texture.is_some()
                && glass.emissive_texture == glass.base_color_texture
            {
                glass.emissive_texture = Some(g.clone());
            }
            glass.base_color_texture = Some(g);
            glass.alpha_mode = AlphaMode::Blend;
            glass.cull_mode = Some(bevy::render::render_resource::Face::Back);
            glass.double_sided = false;
            material.alpha_mode = AlphaMode::Mask(TRANSMAP_OPAQUE);
            glass
        })
    }
}
