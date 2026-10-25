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
mod tests {
    use super::*;

    /// A redrawn image marks the materials showing it (and only those) modified, so they are
    /// bound to its new GPU texture.
    #[test]
    fn redrawn_image_rebinds_its_materials() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .init_resource::<RedrawnImages>()
            .add_systems(Update, rebind_redrawn_images);
        let (text, other) = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            (
                images.add(text_image(4, 4, vec![0; 64])),
                images.add(text_image(4, 4, vec![0; 64])),
            )
        };
        let (shows, unrelated) = {
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
            (
                materials.add(StandardMaterial::from(text.clone())),
                materials.add(StandardMaterial::from(other)),
            )
        };
        app.update();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<StandardMaterial>>>()
            .clear();
        app.world_mut()
            .resource_mut::<RedrawnImages>()
            .0
            .insert(text.id());
        app.update();
        app.update();
        let messages = app
            .world()
            .resource::<Messages<AssetEvent<StandardMaterial>>>();
        let modified: Vec<_> = messages
            .iter_current_update_messages()
            .chain(messages.get_cursor().read(messages))
            .filter_map(|e| match e {
                AssetEvent::Modified { id } => Some(*id),
                _ => None,
            })
            .collect();
        assert!(modified.contains(&shows.id()));
        assert!(!modified.contains(&unrelated.id()));
        assert!(app.world().resource::<RedrawnImages>().0.is_empty());
    }

    /// A text image with a mip chain is not copied into its old GPU texture (which would fill
    /// only the largest level).
    #[test]
    fn mipmapped_text_image_gets_a_new_texture() {
        let image = text_image(64, 8, vec![0; 64 * 8 * 4]);
        if image.texture_descriptor.mip_level_count > 1 {
            assert!(!image
                .texture_descriptor
                .usage
                .contains(TextureUsages::COPY_DST));
        }
    }
}
