//! The clamped variant of a material's bitmap (`[matl_texadress_clamp]`): the same picture, sampled
//! at its edges instead of repeated.

use bevy::asset::Handle;
use bevy::image::Image;

use super::super::cache::ContentLoader;

impl ContentLoader<'_> {
    /// The texture sampled at its edges instead of repeated (`[matl_texadress_clamp]`), made once
    /// per source image: the same picture serves a clamped and an unclamped material alike.
    pub(in crate::model) fn clamped_variant(&mut self, handle: &Handle<Image>) -> Handle<Image> {
        if let Some(h) = self.cache.clamped.get(&handle.id()) {
            return h.clone();
        }
        let out = match self.images.get(handle) {
            Some(img) => {
                let mut copy = img.clone();
                let mut d = match &copy.sampler {
                    bevy::image::ImageSampler::Descriptor(d) => d.clone(),
                    _ => crate::texture_filter::object_sampler_descriptor(
                        crate::texture_filter::texture_filtering(),
                    ),
                };
                use bevy::image::ImageAddressMode::ClampToEdge;
                d.address_mode_u = ClampToEdge;
                d.address_mode_v = ClampToEdge;
                d.address_mode_w = ClampToEdge;
                copy.sampler = bevy::image::ImageSampler::Descriptor(d);
                self.images.add(copy)
            }
            None => handle.clone(),
        };
        self.cache.clamped.insert(handle.id(), out.clone());
        out
    }
}
