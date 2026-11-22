//! Tests of the snow refresh of loaded scenery textures (`season_refresh`).
use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// A 1x1 image whose red channel is `v`.
fn solid(v: u8) -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[v, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn red(images: &Assets<Image>, id: AssetId<Image>) -> u8 {
    images
        .get(id)
        .and_then(|i| i.data.as_ref())
        .map_or(0, |d| d[0])
}

fn decode(path: &Path) -> Option<Image> {
    Some(solid(if path.ends_with("snow.png") { 200 } else { 10 }))
}

#[test]
fn a_texture_loaded_before_the_cover_changes_shows_the_snow_file_after_the_change() {
    let mut images = Assets::<Image>::default();
    let handle = images.add(solid(10));
    let mut textures = SeasonTextures::default();
    textures.record(
        handle.id(),
        false,
        Some(PathBuf::from("snow.png")),
        Some(PathBuf::from("dry.png")),
    );
    assert_eq!(textures.refresh(&mut images, false, decode), 0);
    assert_eq!(red(&images, handle.id()), 10);
    assert_eq!(textures.refresh(&mut images, true, decode), 1);
    assert_eq!(red(&images, handle.id()), 200);
    assert_eq!(textures.refresh(&mut images, true, decode), 0);
    assert_eq!(textures.refresh(&mut images, false, decode), 1);
    assert_eq!(red(&images, handle.id()), 10);
}

#[test]
fn a_missing_variant_keeps_the_image_and_a_gone_asset_is_dropped() {
    let mut images = Assets::<Image>::default();
    let handle = images.add(solid(10));
    let mut textures = SeasonTextures::default();
    textures.record(handle.id(), false, None, Some(PathBuf::from("dry.png")));
    assert_eq!(textures.refresh(&mut images, true, decode), 0);
    assert_eq!(red(&images, handle.id()), 10);
    let id = handle.id();
    images.remove(id);
    assert_eq!(textures.refresh(&mut images, true, decode), 0);
    assert!(!textures.contains(id));
}
