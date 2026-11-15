//! Premultiplying glass colour textures: the RGBA8 conversion (in linear light for sRGB) and the
//! copy of an image with a fresh mip chain.

use super::*;

/// Multiplies the colour of RGBA8 texels by their alpha, in linear light (`srgb`: the bytes are
/// sRGB encoded). Texels with alpha 255 are left alone, with alpha 0 they become black.
pub fn premultiply_rgba8(data: &mut [u8], srgb: bool) {
    let to_linear = |v: u8| -> f32 {
        let c = f32::from(v) / 255.0;
        if !srgb {
            c
        } else if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let from_linear = |l: f32| -> u8 {
        let c = if !srgb {
            l
        } else if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        };
        (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    // Cache of the last texel: window textures are flat in large areas.
    let mut last: Option<([u8; 4], [u8; 3])> = None;
    for px in data.chunks_exact_mut(4) {
        let a = px[3];
        if a == 255 {
            continue;
        }
        let key = [px[0], px[1], px[2], a];
        if let Some((k, out)) = last {
            if k == key {
                px[..3].copy_from_slice(&out);
                continue;
            }
        }
        let f = f32::from(a) / 255.0;
        let out = [
            from_linear(to_linear(px[0]) * f),
            from_linear(to_linear(px[1]) * f),
            from_linear(to_linear(px[2]) * f),
        ];
        px[..3].copy_from_slice(&out);
        last = Some((key, out));
    }
}

/// A copy of `source` whose colour is multiplied by its alpha, with a fresh mip chain; `None`
/// when the texels cannot be read on the CPU.
pub(super) fn premultiplied_copy(source: &Image) -> Option<Image> {
    use bevy::render::render_resource::TextureFormat as F;
    let mut out = crate::texture::decode_to_rgba8(source)?;
    let srgb = match out.texture_descriptor.format {
        F::Rgba8UnormSrgb => true,
        F::Rgba8Unorm => false,
        _ => return None,
    };
    let texels = out.width() as usize * out.height() as usize * 4;
    let data = out.data.as_mut()?;
    data.truncate(texels);
    premultiply_rgba8(data, srgb);
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}
