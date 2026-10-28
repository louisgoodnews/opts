//! Material slot building, alpha composition, and lightmap/nightmap handling for O3D parts.

use super::*;

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
        for (slot, mut mesh) in slot_meshes {
            // Static model parts carry no skeleton: joint attributes without a `SkinnedMesh`
            // component make the render pipeline layouts disagree (fatal wgpu validation error).
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
                continue;
            }
            let failures_before = self.cache.stats.failures.len();
            // An entry without a texture (a `[matl_change]` or `[matl_item]` that only sets
            // switches) keeps the material's own texture.
            let tex_name = over
                .map(|o| o.texture.clone())
                .filter(|t| !t.trim().is_empty())
                .or_else(|| base.primary_texture().map(str::to_owned));
            let mut tex = match over.and_then(|o| o.image.clone()) {
                Some(h) => Some(h),
                None => tex_name.clone().and_then(|n| {
                    let replacement = self.cache.texture_replacements.get(&base_name(&n)).cloned();
                    match replacement {
                        Some(path) => {
                            let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                            let file = path
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            self.texture(&file, &[dir])
                                .or_else(|| self.texture(&n, ctx.tex_dirs))
                        }
                        None => self.texture(&n, ctx.tex_dirs),
                    }
                }),
            };
            // A `[matl]` entry that names a file that does not exist (a typo in the scenery
            // object) falls back to the texture the mesh itself names.
            if tex.is_none() && over.is_some() {
                if let Some(own) = base.primary_texture().map(str::to_owned) {
                    if tex_name.as_deref() != Some(own.as_str()) {
                        tex = self.texture(&own, ctx.tex_dirs);
                    }
                }
            }
            // Benchmark sentinel (M-0054): the diffuse texture or the material slot is missing.
            let sentinel = crate::texture_sentinel::missing_texture_sentinel()
                && (self.cache.stats.failures.len() > failures_before
                    || model.materials.get(slot).is_none());
            let mut blended_by_map = false;
            let mut glass_tex = None;
            if let (Some(base_h), Some(tm)) = (tex.clone(), over.and_then(|o| o.trans_map.clone()))
            {
                if let Some(trans_h) = self.texture(&tm, ctx.tex_dirs) {
                    let split = over.and_then(|o| o.alpha).is_none_or(|a| a == 2);
                    let key = (base_h.id(), trans_h.id(), split);
                    if let Some((done, glass)) = self.cache.composed.get(&key) {
                        tex = Some(done.clone());
                        glass_tex = glass.clone();
                        blended_by_map = true;
                    } else {
                        let composed = match (self.images.get(&base_h), self.images.get(&trans_h)) {
                            (Some(b), Some(t)) => {
                                let _t = opts_logging::stream_trace::span("o3d.compose_alpha");
                                compose_alpha(b, t)
                            }
                            _ => None,
                        };
                        if let Some(img) = composed {
                            // A mostly opaque composition (a car body with its windows) is
                            // drawn as an opaque cut-out body plus a blended glass pass;
                            // blending the whole body lets its far side and interior show
                            // through.
                            let glass = split
                                .then(|| transmap_glass(&img))
                                .flatten()
                                .map(|g| self.images.add(g));
                            let done = self.images.add(img);
                            self.cache
                                .composed
                                .insert(key, (done.clone(), glass.clone()));
                            tex = Some(done);
                            glass_tex = glass;
                            blended_by_map = true;
                        }
                    }
                }
            }
            // A blended texture that is mostly opaque (a car body with its windows in the
            // alpha channel) is drawn like the transmap composition: an opaque cut-out body
            // plus a blended glass pass. Blending the whole body sorts it against the
            // interior behind it, which then shows through in patches (Bremen parked cars).
            // Not for a texture chosen at run time (`[matl_freetex]`): its default is a
            // placeholder (the Lion's City's VDV layers start with an opaque grey one), and
            // split into an opaque body it would cover the display with grey whatever the
            // scripts show.
            if glass_tex.is_none()
                && !blended_by_map
                && over.is_some_and(|o| {
                    o.alpha == Some(2) && o.trans_map.is_none() && o.free_texture.is_none()
                })
            {
                let _t = opts_logging::stream_trace::span("o3d.body_glass_split");
                if let Some(source) = tex.clone() {
                    glass_tex = if let Some(done) = self.cache.glass_split.get(&source.id()) {
                        done.clone()
                    } else {
                        let made = self
                            .images
                            .get(&source)
                            .and_then(crate::texture::decode_to_rgba8)
                            .and_then(|decoded| super::overrides::transmap_glass(&decoded))
                            .map(|g| self.images.add(g));
                        self.cache.glass_split.insert(source.id(), made.clone());
                        made
                    };
                }
            }
            if !crate::material::uses_texture_alpha(over.and_then(|o| o.alpha)) && !blended_by_map {
                let _t = opts_logging::stream_trace::span("o3d.opaque_variant");
                tex = tex.map(|h| self.opaque_variant(&h));
            }
            let textured = tex.is_some();
            let mut material = o3d_material_to_standard_material(&base, tex);
            if textured {
                // A textured surface is lit by its texture; the .o3d diffuse tint is a
                // fixed-function multiplier that is white in practice.
                material.base_color = bevy::color::Color::srgba(
                    1.0,
                    1.0,
                    1.0,
                    crate::material::effective_diffuse_alpha(base.diffuse[3]),
                );
            }
            // A script canvas that is the transparency map (`\S:n`, the LED matrices) is a
            // light source: it shows its colour whatever the daylight.
            if over
                .and_then(|o| o.trans_map.as_deref())
                .is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("\\s:"))
            {
                material.unlit = true;
                // Drawn after the cover glass in front of it, which does not write depth:
                // a light is seen through the glass at its full brightness.
                material.depth_bias = 10.0;
                // The `.o3d` diffuse alpha (a fraction here) would dim the lit pixels of the
                // mask a second time: the canvas alone says how bright an LED is.
                material.base_color = bevy::color::Color::WHITE;
            }
            if let Some(mode) = over
                .and_then(|o| o.alpha)
                .filter(|m| crate::material::uses_texture_alpha(Some(*m)))
            {
                // Mode 2 blends (glass, smoke); anything else is a hard cutout (foliage, fences).
                material.alpha_mode = if mode == 2 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Mask(0.5)
                };
            }
            if blended_by_map && over.and_then(|o| o.alpha).is_none() {
                material.alpha_mode = AlphaMode::Blend;
            }
            let glass_material = glass_tex.map(|g| {
                let mut glass = material.clone();
                // The self-illumination follows the picture (`material::emission_texture`).
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
            });
            // OMSI renders with back-face culling; keeping it also lets a camera that ends up
            // inside a building see out through the (culled) inner faces.
            material.cull_mode = Some(bevy::render::render_resource::Face::Back);
            material.double_sided = false;
            // A light map is emissive light switched by a script variable; it starts dark.
            // Maps without a variable have no switch and stay off ❓.
            let mut light_variable = None;
            let mut light_image = None;
            if let Some(lm) = over
                .and_then(|o| o.light_map.as_ref())
                .filter(|lm| !lm.variable.trim().is_empty())
            {
                if let Some(h) = self.texture(&lm.texture, ctx.tex_dirs) {
                    // The light map lights the material's own texture (texture × map).
                    let lit = material.base_color_texture.clone().and_then(|base_h| {
                        let key = (base_h.id(), h.id());
                        if let Some(done) = self.cache.lit.get(&key) {
                            return Some(done.clone());
                        }
                        let img = {
                            let _t = opts_logging::stream_trace::span("o3d.compose_light");
                            match (self.images.get(&base_h), self.images.get(&h)) {
                                (Some(b), Some(l)) => super::overrides::compose_light(b, l),
                                _ => None,
                            }
                        }?;
                        let done = self.images.add(img);
                        self.cache.lit.insert(key, done.clone());
                        Some(done)
                    });
                    light_image = Some(h.clone());
                    material.emissive_texture = Some(lit.unwrap_or(h));
                    material.emissive = bevy::color::LinearRgba::BLACK;
                    light_variable = Some(lm.variable.trim().to_owned());
                }
            }
            // A night map (no light map): the lit picture itself, switched by the night.
            if light_variable.is_none() {
                if let Some(night) = over.and_then(|o| o.night_map.as_ref()) {
                    if let Some(h) = self.texture(night, ctx.tex_dirs) {
                        material.emissive_texture = Some(h);
                        material.emissive = bevy::color::LinearRgba::BLACK;
                        light_variable = Some(crate::scenery_runtime::NIGHT_VARIABLE.to_owned());
                    }
                }
            }
            if sentinel {
                material = crate::texture_sentinel::sentinel_material();
            }
            if sc.lap.active() {
                sc.materials.push(crate::scene_trace::MaterialNote {
                    slot,
                    texture: tex_name.clone().unwrap_or_default(),
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
                        tex_name.clone().unwrap_or_default()
                    ));
                }
            }
            tracing::debug!(path = %ctx.o3d_path.display(), slot, vertices, textured, alpha = ?material.alpha_mode, texture = ?tex_name, "model part");
            let pivot = model
                .transform
                .filter(|_| model.version != 0)
                .map(|m| Vec3::new(m[3][0], m[3][1], -m[3][2]));
            let material = self.cache.pool.material(self.materials, material);
            if let Some(m) = self.materials.get(&material) {
                // Glass and mirror surfaces for the reflections (`glass`, `mirrors`), and the
                // research prototype hook (M-0085; free unless `OPTS_REFLECT` is set).
                crate::glass::note_surface(&material, tex_name.as_deref(), m, false);
                crate::reflections::note_material(&material, tex_name.as_deref(), m);
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
                    .map(|v| (v, light_image.clone())),
                pivot,
            };
            if let Some(glass) = glass_material {
                self.cache.stats.transmap_splits += 1;
                let glass_part = ModelPart {
                    material: self.cache.pool.material(self.materials, glass),
                    ..part.clone()
                };
                if let Some(g) = self.materials.get(&glass_part.material) {
                    // The glass pass of a transmap split is glass whatever its texture is called.
                    crate::glass::note_surface(&glass_part.material, tex_name.as_deref(), g, true);
                }
                parts.push(part);
                parts.push(glass_part);
            } else {
                parts.push(part);
            }
        }
    }
}
