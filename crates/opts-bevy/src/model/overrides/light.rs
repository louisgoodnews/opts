//! Light maps: the emissive picture of a light-mapped material, multiplied in linear light.

use super::*;

/// The emissive picture of a light-mapped material: its texture multiplied by the light map, in
/// linear light. OMSI's light maps are masks of the light's colour over the lit areas (the MAN
/// GN92 panel's is one warm patch over all gauges, the MB C2's display a white rectangle): used
/// as the emitted colour itself they paint those areas over; multiplied they light the dials and
/// the display's picture. The light map is sampled at the texture's resolution.
pub fn compose_light(base: &Image, light: &Image) -> Option<Image> {
    let base = crate::texture::decode_to_rgba8(base)?;
    let light = crate::texture::decode_to_rgba8(light)?;
    let (bw, bh) = (base.width() as usize, base.height() as usize);
    let (lw, lh) = (light.width() as usize, light.height() as usize);
    let base_data = base.data.as_deref()?;
    let light_data = light.data.as_deref()?;
    if bw == 0
        || bh == 0
        || lw == 0
        || lh == 0
        || base_data.len() < bw * bh * 4
        || light_data.len() < lw * lh * 4
    {
        return None;
    }
    let to_linear: Vec<f32> = (0..256)
        .map(|v| {
            let c = v as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect();
    let to_srgb = |l: f32| -> u8 {
        let c = if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        };
        (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8
    };
    let mut out = base.clone();
    out.texture_descriptor.mip_level_count = 1;
    let data = out.data.as_mut()?;
    data.truncate(bw * bh * 4);
    for y in 0..bh {
        let ly = y * lh / bh;
        for x in 0..bw {
            let lx = x * lw / bw;
            let o = (y * bw + x) * 4;
            let l = (ly * lw + lx) * 4;
            for c in 0..3 {
                let lin =
                    to_linear[usize::from(data[o + c])] * to_linear[usize::from(light_data[l + c])];
                data[o + c] = to_srgb(lin);
            }
            data[o + 3] = 255;
        }
    }
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}
