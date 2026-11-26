//! Assembling one slot's model part: its mesh, material and the glass part split off it.

use super::*;

impl ContentLoader<'_> {
    pub(super) fn build_slot_part(
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
        tracing::debug!(path = %ctx.o3d_path.display(), slot, texture = ?base.primary_texture(), has_override = over.is_some(), clamp = over.is_some_and(|o| o.clamp), text_image = over.is_some_and(|o| o.image.is_some()), "slot override chosen");
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
        if over.is_some_and(|o| o.trans_map.is_some()) {
            tracing::debug!(
                path = %ctx.o3d_path.display(),
                slot,
                texture = ?res.tex_name,
                body_alpha = ?material.alpha_mode,
                glass_pass = glass_material.is_some(),
                glass_alpha = ?glass_material.as_ref().map(|g| g.alpha_mode),
                sentinel,
                "transmap slot alpha at spawn"
            );
        }
        tracing::debug!(path = %ctx.o3d_path.display(), slot, vertices, textured, diffuse_alpha = base.diffuse[3], alpha = ?material.alpha_mode, texture = ?res.tex_name, "model part");
        Self::trace_slot_material(sc, slot, res.tex_name.as_deref(), &material, textured);
        let pivot = model
            .transform
            .filter(|_| model.version != 0)
            .map(crate::vehicle_anim::MeshPivot::from_o3d_transform);
        let material = self.cache.pool.material(self.materials, material);
        self.note_part_material(&material, res.tex_name.as_deref());
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

    /// Notes a slot's material for the glass, wetness and tyre passes (`note_surface` and
    /// `note_material`). The tyre test applies only to a vehicle part.
    fn note_part_material(
        &self,
        material: &bevy::prelude::Handle<StandardMaterial>,
        tex_name: Option<&str>,
    ) {
        if let Some(m) = self.materials.get(material) {
            crate::glass::note_surface(material, tex_name, m, false);
            let moisture = self.texture_takes_wetness(tex_name);
            let tyre = self.cache.vehicle_part
                && tex_name.is_some_and(crate::reflections::is_tyre_texture);
            crate::reflections::note_material(material, tex_name, (moisture, tyre), m);
        }
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
}
