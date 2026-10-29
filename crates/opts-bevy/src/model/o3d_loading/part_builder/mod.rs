//! Material slot building, alpha composition, and lightmap/nightmap handling for O3D parts.

use super::*;

mod texture_and_material;
use texture_and_material::ResolvedTextures;

/// Context for building parts from a single decoded [`O3d`] model.
pub(super) struct SlotBuildCtx<'a> {
    pub o3d_path: &'a Path,
    pub tex_dirs: &'a [PathBuf],
    pub overrides: &'a [TextureOverride],
    pub keep_skin: bool,
    pub slots: Option<&'a [(String, u16)]>,
}

impl ContentLoader<'_> {
    pub(super) fn build_model_parts(
        &mut self,
        model: &O3d,
        ctx: &SlotBuildCtx<'_>,
        sc: &mut TraceScratch,
        parts: &mut Vec<ModelPart>,
    ) {
        let t_mesh = opts_logging::stream_trace::span("o3d.meshes");
        let slot_meshes = crate::mesh::o3d_to_bevy_meshes_by_material_slots(model, ctx.slots);
        drop(t_mesh);
        for (slot, mesh) in slot_meshes {
            self.build_slot_part(slot, mesh, model, ctx, sc, parts);
        }
    }

    fn resolve_slot_textures(
        &mut self,
        base: &o3d::Material,
        over: Option<&TextureOverride>,
        ctx: &SlotBuildCtx<'_>,
    ) -> ResolvedTextures {
        let (tex_name, mut tex) = self.resolve_slot_texture_initial(base, over, ctx);
        // The missing-texture sentinel looks at the diffuse texture only, not the maps after it.
        let failures_after_diffuse = self.cache.stats.failures.len();
        let (mut glass_tex, blended_by_map) = self.compose_transmap_glass(&mut tex, over, ctx);
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

    fn configure_slot_material(
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

    fn build_glass_material(
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

    fn build_slot_part(
        &mut self,
        slot: usize,
        mut mesh: Mesh,
        model: &O3d,
        ctx: &SlotBuildCtx<'_>,
        sc: &mut TraceScratch,
        parts: &mut Vec<ModelPart>,
    ) {
        if !ctx.keep_skin {
            mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
            mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
        }
        let vertices = mesh.count_vertices();
        let base = model
            .materials
            .get(slot)
            .cloned()
            .unwrap_or_else(|| o3d::Material {
                diffuse: [1.0; 4],
                specular: [0.0; 3],
                emissive: [0.0; 3],
                power: 0.0,
                textures: Vec::new(),
            });
        let over = find_override(&model.materials, slot, ctx.overrides);
        if over.is_some_and(TextureOverride::is_variable_hidden) {
            self.cache.stats.hidden_parts += 1;
            sc.hidden += 1;
            return;
        }
        let failures_before = self.cache.stats.failures.len();
        let res = self.resolve_slot_textures(&base, over, ctx);
        let sentinel = crate::texture_sentinel::missing_texture_sentinel()
            && (res.failures_after_diffuse > failures_before
                || model.materials.get(slot).is_none());

        let textured = res.tex.is_some();
        let mut material = self.configure_slot_material(&base, over, res.tex, res.blended_by_map);
        let glass_material = Self::build_glass_material(&mut material, res.glass_tex);
        let (light_variable, light_image) = self.apply_lightmap_nightmap(&mut material, over, ctx);
        if sentinel {
            material = crate::texture_sentinel::sentinel_material();
        }
        tracing::debug!(path = %ctx.o3d_path.display(), slot, vertices, textured, alpha = ?material.alpha_mode, texture = ?res.tex_name, "model part");
        Self::trace_slot_material(sc, slot, res.tex_name.as_deref(), &material, textured);
        let pivot = model
            .transform
            .filter(|_| model.version != 0)
            .map(|m| Vec3::new(m[3][0], m[3][1], -m[3][2]));
        let material = self.cache.pool.material(self.materials, material);
        if let Some(m) = self.materials.get(&material) {
            crate::glass::note_surface(&material, res.tex_name.as_deref(), m, false);
            crate::reflections::note_material(&material, res.tex_name.as_deref(), m);
        }
        let part = ModelPart {
            mesh: self.cache.pool.mesh(self.meshes, mesh),
            material,
            vertices,
            light_variable,
            alpha_variable: over
                .and_then(|o| o.runtime_alpha_variable())
                .map(str::to_owned),
            image: over.and_then(|o| o.image.clone()),
            free_texture: over
                .and_then(|o| o.free_texture.clone())
                .map(|v| (v, light_image)),
            pivot,
        };
        self.push_assembled_part(part, glass_material, res.tex_name.as_deref(), parts);
    }

    fn push_assembled_part(
        &mut self,
        part: ModelPart,
        glass_material: Option<StandardMaterial>,
        tex_name: Option<&str>,
        parts: &mut Vec<ModelPart>,
    ) {
        if let Some(glass) = glass_material {
            self.cache.stats.transmap_splits += 1;
            let glass_part = ModelPart {
                material: self.cache.pool.material(self.materials, glass),
                ..part.clone()
            };
            if let Some(g) = self.materials.get(&glass_part.material) {
                crate::glass::note_surface(&glass_part.material, tex_name, g, true);
            }
            parts.push(part);
            parts.push(glass_part);
        } else {
            parts.push(part);
        }
    }

    fn trace_slot_material(
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
