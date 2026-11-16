//! Alpha composition: the alpha channel of a base texture taken from a transmap.

use super::*;

pub(in crate::model) fn compose_alpha(base: &Image, trans: &Image) -> Option<Image> {
    // DDS textures stay block-compressed on the GPU; the texel values are needed here.
    let base = crate::texture::decode_to_rgba8(base)?;
    let trans_decoded = crate::texture::decode_to_rgba8(trans);
    let trans = trans_decoded.as_ref().unwrap_or(trans);
    let base = &base;
    use bevy::render::render_resource::TextureFormat as F;
    let (bw, bh) = (base.width() as usize, base.height() as usize);
    let (tw, th) = (trans.width() as usize, trans.height() as usize);
    let base_data = base.data.as_deref()?;
    let trans_data = trans.data.as_deref()?;
    if !matches!(
        base.texture_descriptor.format,
        F::Rgba8UnormSrgb | F::Rgba8Unorm
    ) || base_data.len() < bw * bh * 4
        || tw == 0
        || th == 0
    {
        return None;
    }
    let sample: Box<dyn Fn(usize) -> u8> = match trans.texture_descriptor.format {
        F::R8Unorm if trans_data.len() >= tw * th => Box::new(|i| trans_data[i]),
        F::Rgba8UnormSrgb | F::Rgba8Unorm if trans_data.len() >= tw * th * 4 => {
            // Use the alpha channel if the map has one, else its luminance.
            let has_alpha = trans_data[..tw * th * 4]
                .chunks_exact(4)
                .any(|p| p[3] != 255);
            Box::new(move |i| {
                let p = &trans_data[i * 4..i * 4 + 4];
                if has_alpha {
                    p[3]
                } else {
                    p[0]
                }
            })
        }
        _ => return None,
    };
    let mut out = base.clone();
    out.texture_descriptor.mip_level_count = 1;
    let out_data = out.data.as_mut()?;
    out_data.truncate(bw * bh * 4);
    for y in 0..bh {
        for x in 0..bw {
            let (tx, ty) = (x * tw / bw, y * th / bh);
            out_data[(y * bw + x) * 4 + 3] = sample(ty * tw + tx);
        }
    }
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}
