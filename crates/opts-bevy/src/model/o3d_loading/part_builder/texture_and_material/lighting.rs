//! The lightmap and nightmap of a part: the emissive texture and the light variable it uses.

use super::*;

impl ContentLoader<'_> {
    pub(in super::super) fn apply_lightmap_nightmap(
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
                            (Some(b), Some(l)) => super::super::overrides::compose_light(b, l),
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
