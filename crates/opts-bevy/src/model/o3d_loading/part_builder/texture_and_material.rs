//! Texture resolution, alpha composition, and lighting maps for O3D parts.

use super::*;

mod glass;
mod lighting;
mod transmap_log;

use transmap_log::{log_transmap_pass, split_requested};

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
        // A bitmap of a clamped material (`[matl_texadress_clamp]`) is sampled at its edges; a
        // ready-made image (rendered text) was already made that way.
        if let (Some(o), Some(h)) = (over, tex.clone()) {
            if o.clamp && o.image.is_none() {
                tex = Some(self.clamped_variant(&h));
            }
        }
        (tex_name, tex)
    }

    pub(super) fn compose_transmap_glass(
        &mut self,
        tex: &mut Option<Handle<Image>>,
        tex_name: Option<&str>,
        over: Option<&TextureOverride>,
        ctx: &SlotBuildCtx<'_>,
    ) -> (Option<Handle<Image>>, bool) {
        let mut blended_by_map = false;
        let mut glass_tex = None;
        let matl_alpha = over.and_then(|o| o.alpha);
        if let (Some(base_h), Some(tm)) = (tex.clone(), over.and_then(|o| o.trans_map.clone())) {
            if let Some(trans_h) = self.texture(&tm, ctx.tex_dirs) {
                let split = split_requested(matl_alpha);
                let key = (base_h.id(), trans_h.id(), split);
                if let Some((done, glass)) = self.cache.composed.get(&key) {
                    *tex = Some(done.clone());
                    glass_tex = glass.clone();
                    blended_by_map = true;
                    log_transmap_pass(ctx, tex_name, &tm, matl_alpha, None, glass.is_some(), true);
                } else {
                    let composed = match (self.images.get(&base_h), self.images.get(&trans_h)) {
                        (Some(b), Some(t)) => {
                            let _t = opts_logging::stream_trace::span("o3d.compose_alpha");
                            compose_alpha(b, t)
                        }
                        _ => None,
                    };
                    if let Some(img) = composed {
                        let share = composed_opaque_share(&img);
                        let glass = split
                            .then(|| transmap_glass(&img))
                            .flatten()
                            .map(|g| self.images.add(g));
                        log_transmap_pass(
                            ctx,
                            tex_name,
                            &tm,
                            matl_alpha,
                            share,
                            glass.is_some(),
                            false,
                        );
                        let done = self.images.add(img);
                        self.cache
                            .composed
                            .insert(key, (done.clone(), glass.clone()));
                        *tex = Some(done);
                        glass_tex = glass;
                        blended_by_map = true;
                    } else {
                        tracing::debug!(
                            mesh = %ctx.o3d_path.display(),
                            texture = ?tex_name,
                            trans_map = %tm,
                            "transmap compose failed, texture keeps its own alpha"
                        );
                    }
                }
            } else {
                tracing::debug!(
                    mesh = %ctx.o3d_path.display(),
                    texture = ?tex_name,
                    trans_map = %tm,
                    "transmap texture not found, texture keeps its own alpha"
                );
            }
        }
        (glass_tex, blended_by_map)
    }
}
