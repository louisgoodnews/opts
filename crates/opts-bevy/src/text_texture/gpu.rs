//! The images text textures and script canvases are drawn into, and keeping their materials
//! bound to them when they are redrawn.

use std::collections::HashSet;

use bevy::asset::{AssetId, Assets, Handle, RenderAssetUsages};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::ResMut;
use bevy::image::Image;
use bevy::pbr::StandardMaterial;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

/// An RGBA8 image holding rendered text. Mesh UVs may lie outside 0..1 (negative V is common):
/// the sampler wraps like every OMSI texture.
#[must_use]
pub fn text_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    text_image_addressed(width, height, data, false)
}

/// Like [`text_image`]; with `clamp` the sampler stops at the edge instead of repeating, as a
/// material with `[matl_texadress_clamp]` asks (its text would otherwise tile).
#[must_use]
pub fn text_image_addressed(width: u32, height: u32, data: Vec<u8>, clamp: bool) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // Mesh UVs may lie outside 0..1 (negative V is common): wrap like every OMSI texture.
    crate::texture_filter::prepare_text_texture(&mut image);
    if clamp {
        clamp_sampler(&mut image);
    }
    // Bevy copies a redrawn image of the same size into the texture already on the GPU, and
    // that copy fills only the largest mip level: the smaller ones keep the first (blank)
    // picture, so text seen small (the HH109's EFAD printer: a 1024 × 64 line on a few
    // centimetres of LCD) never appeared. Without `COPY_DST` a redrawn image gets a new texture
    // with its whole chain; its materials must then be bound again ([`RedrawnImages`]).
    if image.texture_descriptor.mip_level_count > 1 {
        image
            .texture_descriptor
            .usage
            .remove(TextureUsages::COPY_DST);
    }
    image
}

/// Stops the image's sampler at its edges on every axis (`[matl_texadress_clamp]`), keeping its
/// filtering.
fn clamp_sampler(image: &mut bevy::image::Image) {
    use bevy::image::{ImageAddressMode, ImageSampler};
    if let ImageSampler::Descriptor(d) = &mut image.sampler {
        d.address_mode_u = ImageAddressMode::ClampToEdge;
        d.address_mode_v = ImageAddressMode::ClampToEdge;
        d.address_mode_w = ImageAddressMode::ClampToEdge;
    }
}

/// Whether the image's sampler stops at its edges (`[matl_texadress_clamp]`).
fn is_clamped(image: &bevy::image::Image) -> bool {
    use bevy::image::{ImageAddressMode, ImageSampler};
    matches!(
        &image.sampler,
        ImageSampler::Descriptor(d) if d.address_mode_u == ImageAddressMode::ClampToEdge
    )
}

/// `next`, the picture that replaces `old` in place, with `old`'s addressing. A redraw swaps the
/// image for one whose sampler wraps, which undid `[matl_texadress_clamp]` on the first redraw:
/// the fleet number of the MAN SD200 tiled up its mesh.
#[must_use]
pub fn redrawn_as(old: &bevy::image::Image, mut next: bevy::image::Image) -> bevy::image::Image {
    let clamped = is_clamped(old);
    tracing::debug!(clamped, "redrawn image keeps its addressing");
    if clamped {
        clamp_sampler(&mut next);
    }
    next
}

/// Images replaced in place this frame (text textures, script canvases) whose materials must be
/// bound to the new GPU texture.
#[derive(Resource, Default)]
pub struct RedrawnImages(pub HashSet<AssetId<Image>>);

/// Marks the materials showing a [`RedrawnImages`] image as modified. A material's bind group
/// keeps the texture it was made with: when a redrawn image gets a new GPU texture (see
/// [`text_image`]), the material went on showing the old one until it was prepared again.
pub fn rebind_redrawn_images(
    mut redrawn: ResMut<RedrawnImages>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if redrawn.0.is_empty() {
        return;
    }
    let uses = |t: &Option<Handle<Image>>| t.as_ref().is_some_and(|h| redrawn.0.contains(&h.id()));
    let ids: Vec<_> = materials
        .iter()
        .filter(|(_, m)| uses(&m.base_color_texture) || uses(&m.emissive_texture))
        .map(|(id, _)| id)
        .collect();
    for id in ids {
        // `into_inner` marks the asset modified (a plain `get_mut` does not).
        if let Some(material) = materials.get_mut(id) {
            material.into_inner();
        }
    }
    redrawn.0.clear();
}

/// An RGBA8 image for a canvas of small bright dots (LED matrices): no mip chain, linear
/// sampling, wrapping like every OMSI texture.
#[must_use]
pub fn led_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    crate::texture_filter::prepare_led_texture(&mut image);
    image
}

#[cfg(test)]
mod tests;
