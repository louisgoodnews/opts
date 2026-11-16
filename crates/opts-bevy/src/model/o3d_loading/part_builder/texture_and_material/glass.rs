//! The body glass split: the glass layer of a body texture as its own image.

use super::*;

impl ContentLoader<'_> {
    pub(in super::super) fn resolve_body_glass_split(
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
                        .and_then(|decoded| super::super::overrides::transmap_glass(&decoded))
                        .map(|g| self.images.add(g));
                    self.cache.glass_split.insert(source.id(), made.clone());
                    made
                }
            })
        } else {
            None
        }
    }
}
