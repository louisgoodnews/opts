//! Transmap split tests of the model loader.

use super::{transmap_glass_rgba, TRANSMAP_OPAQUE};
use bevy::image::Image;

fn image(alphas: &[u8]) -> Vec<u8> {
    alphas.iter().flat_map(|&a| [200, 10, 10, a]).collect()
}

/// A car body: 6 opaque texels, 2 window texels (transmap 0.6-0.8).
#[test]
fn mostly_opaque_composition_splits_into_body_and_glass() {
    let rgba = image(&[255, 255, 250, 243, 255, 255, 160, 200]);
    let glass = transmap_glass_rgba(&rgba).expect("split");
    let alpha: Vec<u8> = glass.chunks_exact(4).map(|p| p[3]).collect();
    assert_eq!(alpha, vec![0, 0, 0, 0, 0, 0, 160, 200]);
    // Colours are kept; the body pass (mask at 0.95) keeps exactly the zeroed texels.
    assert!(glass.chunks_exact(4).all(|p| p[..3] == [200, 10, 10]));
    let body: Vec<bool> = rgba
        .chunks_exact(4)
        .map(|p| f32::from(p[3]) / 255.0 >= TRANSMAP_OPAQUE)
        .collect();
    assert_eq!(body, alpha.iter().map(|&a| a == 0).collect::<Vec<_>>());
}

/// A shadow quad or smoke: mostly translucent, stays one blended pass.
#[test]
fn mostly_translucent_composition_is_not_split() {
    assert!(transmap_glass_rgba(&image(&[0, 40, 80, 120, 255, 30])).is_none());
    assert!(transmap_glass_rgba(&[]).is_none());
}

/// The composed image comes with a mip chain; the glass must carry exactly the data its
/// descriptor announces (a base level under a full-chain descriptor crashed the upload).
#[test]
fn glass_data_matches_its_mip_levels() {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let n = 64usize;
    let mut alpha = vec![255u8; n * n];
    alpha[..n * n / 8].fill(180);
    let mut composed = Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        image(&alpha),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    crate::texture_filter::prepare_object_texture(&mut composed);
    let glass = super::transmap_glass(&composed).expect("split");
    let levels = glass.texture_descriptor.mip_level_count;
    let expected: usize = (0..levels)
        .map(|l| {
            let (w, h) = crate::texture::level_size(n as u32, n as u32, l);
            w as usize * h as usize * 4
        })
        .sum();
    assert_eq!(glass.data.as_ref().map(Vec::len), Some(expected));
}

/// 242 / 255 = 0.949 is glass, 243 / 255 = 0.953 is body.
#[test]
fn threshold_matches_the_mask_cutoff() {
    let glass = transmap_glass_rgba(&image(&[242, 243, 255])).expect("split");
    assert_eq!(glass[3], 242);
    assert_eq!(glass[7], 0);
}

#[test]
fn a_light_map_lights_the_texture_instead_of_replacing_it() {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let rgba = |w: u32, h: u32, px: &[[u8; 4]]| {
        let mut img = Image::new(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            px.iter().flatten().copied().collect(),
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        img.texture_descriptor.mip_level_count = 1;
        img
    };
    // A dark display with one bright pixel, under a white light map (the MB C2 display) and a
    // half-size light map that lights only the left half.
    let base = rgba(2, 1, &[[255, 255, 255, 255], [20, 20, 20, 255]]);
    let white = rgba(1, 1, &[[255, 255, 255, 255]]);
    let lit = super::overrides::compose_light(&base, &white).expect("composed");
    let d = lit.data.as_deref().unwrap();
    assert_eq!(&d[0..3], &[255, 255, 255], "white light keeps the picture");
    assert_eq!(
        &d[4..7],
        &[20, 20, 20],
        "a dark pixel stays dark, not white"
    );
    let left_only = rgba(2, 1, &[[255, 128, 0, 255], [0, 0, 0, 255]]);
    let lit = super::overrides::compose_light(&base, &left_only).expect("composed");
    let d = lit.data.as_deref().unwrap();
    assert_eq!(
        &d[0..3],
        &[255, 128, 0],
        "the light's colour tints the lit area"
    );
    assert_eq!(&d[4..7], &[0, 0, 0], "the unlit area emits nothing");
}
