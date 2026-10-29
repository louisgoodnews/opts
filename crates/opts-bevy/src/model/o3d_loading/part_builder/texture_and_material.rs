//! Texture resolution, alpha composition, and lighting maps for O3D parts.

use super::*;

pub(super) struct ResolvedTextures {
    pub tex_name: Option<String>,
    pub tex: Option<Handle<Image>>,
    pub glass_tex: Option<Handle<Image>>,
    pub blended_by_map: bool,
    /// Texture failures recorded once the diffuse texture was looked up.
    pub failures_after_diffuse: usize,
}

impl ContentLoader<'_> {
    pub(super) fn resolve_slot_texture_initial(
        &mut self,
        base: &o3d::Material,
        over: Option<&TextureOverride>,
        ctx: &SlotBuildCtx<'_>,
    ) -> (Option<String>, Option<Handle<Image>>) {
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
        if tex.is_none() && over.is_some() {
            if let Some(own) = base.primary_texture().map(str::to_owned) {
                if tex_name.as_deref() != Some(own.as_str()) {
                    tex = self.texture(&own, ctx.tex_dirs);
                }
            }
        }
        (tex_name, tex)
    }

    pub(super) fn compose_transmap_glass(
        &mut self,
        tex: &mut Option<Handle<Image>>,
        over: Option<&TextureOverride>,
        ctx: &SlotBuildCtx<'_>,
    ) -> (Option<Handle<Image>>, bool) {
        let mut blended_by_map = false;
        let mut glass_tex = None;
        if let (Some(base_h), Some(tm)) = (tex.clone(), over.and_then(|o| o.trans_map.clone())) {
            if let Some(trans_h) = self.texture(&tm, ctx.tex_dirs) {
                let split = over.and_then(|o| o.alpha).is_none_or(|a| a == 2);
                let key = (base_h.id(), trans_h.id(), split);
                if let Some((done, glass)) = self.cache.composed.get(&key) {
                    *tex = Some(done.clone());
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
                        let glass = split
                            .then(|| transmap_glass(&img))
                            .flatten()
                            .map(|g| self.images.add(g));
                        let done = self.images.add(img);
                        self.cache
                            .composed
                            .insert(key, (done.clone(), glass.clone()));
                        *tex = Some(done);
                        glass_tex = glass;
                        blended_by_map = true;
                    }
                }
            }
        }
        (glass_tex, blended_by_map)
    }

    pub(super) fn resolve_body_glass_split(
        &mut self,
        tex: &Option<Handle<Image>>,
        over: Option<&TextureOverride>,
        blended_by_map: bool,
    ) -> Option<Handle<Image>> {
        if !blended_by_map
            && over.is_some_and(|o| {
                o.alpha == Some(2) && o.trans_map.is_none() && o.free_texture.is_none()
            })
        {
            let _t = opts_logging::stream_trace::span("o3d.body_glass_split");
            tex.clone().and_then(|source| {
                if let Some(done) = self.cache.glass_split.get(&source.id()) {
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
                }
            })
        } else {
            None
        }
    }

    pub(super) fn apply_lightmap_nightmap(
        &mut self,
        material: &mut StandardMaterial,
        over: Option<&TextureOverride>,
        ctx: &SlotBuildCtx<'_>,
    ) -> (Option<String>, Option<Handle<Image>>) {
        let mut light_variable = None;
        let mut light_image = None;
        if let Some(lm) = over
            .and_then(|o| o.light_map.as_ref())
            .filter(|lm| !lm.variable.trim().is_empty())
        {
            if let Some(h) = self.texture(&lm.texture, ctx.tex_dirs) {
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
        if light_variable.is_none() {
            if let Some(night) = over.and_then(|o| o.night_map.as_ref()) {
                if let Some(h) = self.texture(night, ctx.tex_dirs) {
                    material.emissive_texture = Some(h);
                    material.emissive = bevy::color::LinearRgba::BLACK;
                    light_variable = Some(crate::scenery_runtime::NIGHT_VARIABLE.to_owned());
                }
            }
        }
        (light_variable, light_image)
    }
}
