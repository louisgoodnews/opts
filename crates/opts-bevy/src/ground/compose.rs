//! Blending the ground layers of a tile into one RGBA texture ([`compose_tile_ground`]).

use super::*;


/// One ground layer: its texture and how often it repeats across the 300 m tile.
pub struct GroundLayer<'a> {
    /// Decoded layer texture.
    pub texture: &'a CpuImage,
    /// Repeats per tile edge (`[groundtex]` parameter 3; 60 = one repeat per 5 m cell).
    pub repeats: f32,
    /// Blend weight mask (alpha), `None` for the base layer.
    pub mask: Option<&'a CpuImage>,
}

/// Blends `layers` (first = base) into a `size × size` RGBA texture of one tile.
///
/// Row 0 is the **north** edge. Masks are sampled without wrap (clamped) and their alpha is the
/// blend weight of the layer above everything below it.
#[must_use]
pub fn compose_tile_ground(layers: &[GroundLayer], size: usize) -> Image {
    let mut data = vec![255u8; size * size * 4];
    let inv = 1.0 / size as f32;
    for y in 0..size {
        let v = (y as f32 + 0.5) * inv; // 0 = north
        for x in 0..size {
            let u = (x as f32 + 0.5) * inv;
            let mut acc = [0.0f32; 3];
            for (li, layer) in layers.iter().enumerate() {
                let t = layer
                    .texture
                    .sample_wrap(u * layer.repeats, v * layer.repeats);
                let w = match (li, layer.mask) {
                    (0, _) => 1.0,
                    (_, Some(m)) => {
                        let mx = ((u * m.width as f32) as usize).min(m.width - 1);
                        let my = ((v * m.height as f32) as usize).min(m.height - 1);
                        f32::from(m.rgba[(my * m.width + mx) * 4 + 3]) / 255.0
                    }
                    _ => 0.0,
                };
                if w > 0.0 {
                    for k in 0..3 {
                        acc[k] = acc[k] * (1.0 - w) + t[k] * w;
                    }
                }
            }
            let o = (y * size + x) * 4;
            data[o] = acc[0].round() as u8;
            data[o + 1] = acc[1].round() as u8;
            data[o + 2] = acc[2].round() as u8;
        }
    }
    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
