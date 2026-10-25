//! The O3D mesh path of the content loader.

use super::*;

impl ContentLoader<'_> {
    /// Loads and splits an `.o3d` file into one [`ModelPart`] per used material.
    ///
    /// `tex_dirs` are searched for textures before the global `Texture` folder.
    pub fn o3d_parts(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, false, None, None)
    }

    /// [`Self::o3d_parts`] bent over a `[crossing_heightdeformation]` base surface: every vertex
    /// is raised by the base height under it. `bend` is `(base file, surface)`; the parts are
    /// cached per base file.
    pub fn o3d_parts_bent(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        bend: Option<(&Path, &BaseSurface)>,
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, false, None, bend)
    }

    /// The base surface of a `[crossing_heightdeformation]` mesh file (cached; `None` when the
    /// file cannot be read or decoded).
    pub fn base_surface(&mut self, path: &Path) -> Option<std::sync::Arc<BaseSurface>> {
        if let Some(b) = self.cache.bases.get(path) {
            return b.clone();
        }
        let base = match load_decoded_meshes(path) {
            Ok(models) => Some(std::sync::Arc::new(BaseSurface::from_meshes(&models))),
            Err(e) => {
                self.fail(format!("crossing base mesh {}: {e}", path.display()));
                None
            }
        };
        self.cache.bases.insert(path.to_path_buf(), base.clone());
        base
    }

    /// Like [`Self::o3d_parts`], but keeps the bone-weight vertex attributes (for meshes that
    /// are spawned with a `SkinnedMesh`, i.e. humans).
    pub fn o3d_skinned_parts(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, true, None, None)
    }

    /// [`Self::o3d_skinned_parts`] for a vehicle mesh whose `[setbone]` entries give the joint
    /// numbers (`(bone name, joint)`); bones the model does not list use the joint after the
    /// highest one.
    pub fn o3d_skinned_parts_slots(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        slots: &[(String, u16)],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, true, Some(slots), None)
    }

    fn o3d_parts_impl(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        keep_skin: bool,
        slots: Option<&[(String, u16)]>,
        bend: Option<(&Path, &BaseSurface)>,
    ) -> Vec<ModelPart> {
        let mut scratch = TraceScratch::new();
        let parts = self.o3d_parts_core(
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin,
            slots,
            bend,
            &mut scratch,
        );
        if scratch.lap.active() {
            scratch.finish(self.meshes, o3d_path, &parts);
        }
        parts
    }

    #[allow(clippy::too_many_arguments)]
    fn o3d_parts_core(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        keep_skin: bool,
        slots: Option<&[(String, u16)]>,
        bend: Option<(&Path, &BaseSurface)>,
        sc: &mut TraceScratch,
    ) -> Vec<ModelPart> {
        let mut repaint: Vec<_> = self.cache.texture_replacements.iter().collect();
        repaint.sort();
        let key = parts_key(
            o3d_path,
            overrides,
            keep_skin,
            &repaint,
            slots,
            bend.map(|b| b.0),
        );
        if let Some(parts) = self.cache.parts.get(&key) {
            sc.stage = "parts-cache";
            if self.cache.protected.contains(o3d_path) {
                self.cache.stats.protected_meshes += 1;
            }
            return parts.clone();
        }

        let _t_miss = opts_logging::stream_trace::span("o3d.parts_miss");
        let t_parse = opts_logging::stream_trace::span("o3d.read_parse");
        let bytes = match std::fs::read(o3d_path) {
            Ok(b) => b,
            Err(e) => {
                sc.stage = "read-failed";
                sc.warn(|| format!("read failed: {e}"));
                self.fail(format!("o3d read failed {}: {e}", o3d_path.display()));
                return Vec::new();
            }
        };
        sc.read_ms = sc.lap.split();
        let mut models: Vec<O3d> = match crate::mesh::parse_mesh_file(o3d_path, &bytes) {
            Ok(m) => m,
            Err(e) => {
                sc.stage = "parse-failed";
                sc.warn(|| format!("parse failed: {e}"));
                self.fail(format!("mesh parse failed {}: {e}", o3d_path.display()));
                return Vec::new();
            }
        };
        sc.parse_ms = sc.lap.split();
        sc.stage = "plain";
        drop(t_parse);
        let is_o3d = o3d_path
            .extension()
            .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("o3d"));
        if is_o3d {
            // Keyed meshes: the exporter's generator is known (`o3d.md`, Addendum 14), so the
            // original positions, texture coordinates and normals come back exactly and the mesh
            // is read like a plain file from here on.
            for m in &mut models {
                if m.encryption_key.is_some() && o3d::exact::decode(m).is_some() {
                    self.cache.stats.exact_meshes += 1;
                    sc.stage = "exact";
                }
            }
            // A keyed file the generator does not cover would still read as `Unreadable`
            // (none is known); it is skipped with a warning rather than drawn scrambled.
            sc.decode_ms = sc.lap.split();
            if models
                .iter()
                .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable))
            {
                sc.stage = "rejected";
                self.cache.stats.protected_meshes += 1;
                self.cache.protected.insert(o3d_path.to_path_buf());
                tracing::warn!(
                    path = %o3d_path.display(),
                    "mesh vertex positions are scrambled and could not be decoded; skipped"
                );
                self.cache.parts.insert(key, Vec::new());
                return Vec::new();
            }
            // A mesh that is hundreds of metres across is not an object: positions are corrupt
            // in a way no plausibility check caught (stray far-away vertices).
            let extent = models
                .iter()
                .map(|m| {
                    let (mut lo, mut hi) = (
                        bevy::math::Vec3::splat(f32::MAX),
                        bevy::math::Vec3::splat(f32::MIN),
                    );
                    for t in &m.indices {
                        for &i in t {
                            if let Some(v) = m.vertices.get(i as usize) {
                                lo = lo.min(v.position);
                                hi = hi.max(v.position);
                            }
                        }
                    }
                    (hi - lo).max_element()
                })
                .fold(0.0_f32, f32::max);
            if extent > 60.0 {
                tracing::info!(path = %o3d_path.display(), extent, layout = ?models.first().map(O3d::vertex_layout), "large mesh");
            }
            let limit = MAX_PLAUSIBLE_EXTENT_M;
            if extent > limit {
                sc.stage = "extent";
                sc.warn(|| format!("extent {extent:.0} m exceeds {limit:.0} m"));
                self.cache.stats.protected_meshes += 1;
                self.cache.protected.insert(o3d_path.to_path_buf());
                tracing::warn!(path = %o3d_path.display(), extent, "mesh extent is implausible; skipped");
                self.cache.parts.insert(key, Vec::new());
                return Vec::new();
            }
            if models.iter().any(|m| {
                matches!(
                    m.vertex_layout(),
                    o3d::VertexLayout::AxisCycled | o3d::VertexLayout::RecomputedNormals
                )
            }) {
                self.cache.stats.axis_cycled_meshes += 1;
                tracing::debug!(path = %o3d_path.display(), "axis-cycled vertex layout; normals recomputed");
            }
        }
        if let Some((_, base)) = bend {
            for m in &mut models {
                base.bend_mesh(m);
            }
        }
        for model in &models {
            self.cache.stats.o3d_files += 1;
            self.cache.stats.o3d_vertices += model.vertices.len();
            tracing::debug!(
                path = %o3d_path.display(),
                vertices = model.vertices.len(),
                triangles = model.indices.len(),
                materials = model.materials.len(),
                "loaded mesh"
            );
        }

        let mut parts = Vec::new();
        for model in &models {
            let t_mesh = opts_logging::stream_trace::span("o3d.meshes");
            let slot_meshes = crate::mesh::o3d_to_bevy_meshes_by_material_slots(model, slots);
            drop(t_mesh);
            for (slot, mut mesh) in slot_meshes {
                // Static model parts carry no skeleton: joint attributes without a `SkinnedMesh`
                // component make the render pipeline layouts disagree (fatal wgpu validation error).
                if !keep_skin {
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
                let over = find_override(&model.materials, slot, overrides);
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
                        let replacement =
                            self.cache.texture_replacements.get(&base_name(&n)).cloned();
                        match replacement {
                            Some(path) => {
                                let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                                let file = path
                                    .file_name()
                                    .map(|f| f.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                self.texture(&file, &[dir])
                                    .or_else(|| self.texture(&n, tex_dirs))
                            }
                            None => self.texture(&n, tex_dirs),
                        }
                    }),
                };
                // A `[matl]` entry that names a file that does not exist (a typo in the scenery
                // object) falls back to the texture the mesh itself names.
                if tex.is_none() && over.is_some() {
                    if let Some(own) = base.primary_texture().map(str::to_owned) {
                        if tex_name.as_deref() != Some(own.as_str()) {
                            tex = self.texture(&own, tex_dirs);
                        }
                    }
                }
                // Benchmark sentinel (M-0054): the diffuse texture or the material slot is missing.
                let sentinel = crate::texture_sentinel::missing_texture_sentinel()
                    && (self.cache.stats.failures.len() > failures_before
                        || model.materials.get(slot).is_none());
                let mut blended_by_map = false;
                let mut glass_tex = None;
                if let (Some(base_h), Some(tm)) =
                    (tex.clone(), over.and_then(|o| o.trans_map.clone()))
                {
                    if let Some(trans_h) = self.texture(&tm, tex_dirs) {
                        let split = over.and_then(|o| o.alpha).is_none_or(|a| a == 2);
                        let key = (base_h.id(), trans_h.id(), split);
                        if let Some((done, glass)) = self.cache.composed.get(&key) {
                            tex = Some(done.clone());
                            glass_tex = glass.clone();
                            blended_by_map = true;
                        } else {
                            let composed =
                                match (self.images.get(&base_h), self.images.get(&trans_h)) {
                                    (Some(b), Some(t)) => {
                                        let _t =
                                            opts_logging::stream_trace::span("o3d.compose_alpha");
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
                if !crate::material::uses_texture_alpha(over.and_then(|o| o.alpha))
                    && !blended_by_map
                {
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
                    if let Some(h) = self.texture(&lm.texture, tex_dirs) {
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
                        if let Some(h) = self.texture(night, tex_dirs) {
                            material.emissive_texture = Some(h);
                            material.emissive = bevy::color::LinearRgba::BLACK;
                            light_variable =
                                Some(crate::scenery_runtime::NIGHT_VARIABLE.to_owned());
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
                tracing::debug!(path = %o3d_path.display(), slot, vertices, textured, alpha = ?material.alpha_mode, texture = ?tex_name, "model part");
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
                        crate::glass::note_surface(
                            &glass_part.material,
                            tex_name.as_deref(),
                            g,
                            true,
                        );
                    }
                    parts.push(part);
                    parts.push(glass_part);
                } else {
                    parts.push(part);
                }
            }
        }
        sc.build_ms = sc.lap.split();
        self.cache.parts.insert(key, parts.clone());
        parts
    }
}
