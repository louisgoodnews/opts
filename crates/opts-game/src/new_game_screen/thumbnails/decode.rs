//! Decoding: the RGBA image of a picture, its pixels and the average colour of a texture.

use super::*;

pub(super) fn to_image(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

pub(super) fn pixels(img: &image::DynamicImage) -> Thumb {
    let rgba = img.to_rgba8();
    Thumb::Pixels {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}

/// The average colour of an image file (8 × 8 samples).
pub fn average_colour(src: &Path) -> Option<[u8; 3]> {
    let img = image::open(src).ok()?.thumbnail(8, 8).to_rgb8();
    let n = u64::from(img.width() * img.height());
    if n == 0 {
        return None;
    }
    let mut sum = [0_u64; 3];
    for p in img.pixels() {
        for (s, c) in sum.iter_mut().zip(p.0) {
            *s += u64::from(c);
        }
    }
    Some([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])
}
