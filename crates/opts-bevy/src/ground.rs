//! Per-tile ground texture composition.
//!
//! OMSI paints terrain with up to ~10 ground layers: `global.cfg` lists them (`[groundtex]`,
//! layer 0 is the base) and each tile can carry an **alpha mask per layer**
//! (`texture/map/tile_<x>_<y>.map.<N>.dds`, A8, 64²…1024²) giving the blend weight of layer `N`.
//! This is how map authors paint road markings, pavements and paths straight into the terrain
//! instead of placing splines. [`compose_tile_ground`] blends the layers on the CPU into one
//! RGBA texture covering the whole 300 m tile.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// A decoded 8-bit RGBA image on the CPU (top row first).
#[derive(Debug, Clone)]
pub struct CpuImage {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// `width * height * 4` bytes, RGBA.
    pub rgba: Vec<u8>,
}

fn rgb565(v: u16) -> [u8; 3] {
    let (r, g, b) = ((v >> 11) & 31, (v >> 5) & 63, v & 31);
    [
        ((r * 255 + 15) / 31) as u8,
        ((g * 255 + 31) / 63) as u8,
        ((b * 255 + 15) / 31) as u8,
    ]
}

fn decode_bc_color(block: &[u8], opaque_black: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mix = |x: u8, y: u8, wx: u32, wy: u32, d: u32| {
        ((u32::from(x) * wx + u32::from(y) * wy) / d) as u8
    };
    let mut p = [[0u8; 4]; 4];
    p[0] = [a[0], a[1], a[2], 255];
    p[1] = [b[0], b[1], b[2], 255];
    if c0 > c1 || !opaque_black {
        p[2] = [
            mix(a[0], b[0], 2, 1, 3),
            mix(a[1], b[1], 2, 1, 3),
            mix(a[2], b[2], 2, 1, 3),
            255,
        ];
        p[3] = [
            mix(a[0], b[0], 1, 2, 3),
            mix(a[1], b[1], 1, 2, 3),
            mix(a[2], b[2], 1, 2, 3),
            255,
        ];
    } else {
        p[2] = [
            mix(a[0], b[0], 1, 1, 2),
            mix(a[1], b[1], 1, 1, 2),
            mix(a[2], b[2], 1, 1, 2),
            255,
        ];
        p[3] = [0, 0, 0, 0];
    }
    p
}

fn decode_bc(data: &[u8], w: usize, h: usize, kind: u8) -> Option<Vec<u8>> {
    let block_bytes = if kind == 1 { 8 } else { 16 };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    if data.len() < bw * bh * block_bytes {
        return None;
    }
    let mut out = vec![0u8; w * h * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let blk = &data[(by * bw + bx) * block_bytes..][..block_bytes];
            let (color_blk, alpha): (&[u8], [u8; 16]) = match kind {
                1 => (blk, [255; 16]),
                2 => {
                    let mut a = [0u8; 16];
                    for i in 0..16 {
                        let nib = (blk[i / 2] >> ((i % 2) * 4)) & 15;
                        a[i] = nib * 17;
                    }
                    (&blk[8..], a)
                }
                _ => {
                    let (a0, a1) = (u32::from(blk[0]), u32::from(blk[1]));
                    let mut table = [0u32; 8];
                    table[0] = a0;
                    table[1] = a1;
                    for i in 2..8u32 {
                        table[i as usize] = if a0 > a1 {
                            ((8 - i) * a0 + (i - 1) * a1) / 7
                        } else if i < 6 {
                            ((6 - i) * a0 + (i - 1) * a1) / 5
                        } else if i == 6 {
                            0
                        } else {
                            255
                        };
                    }
                    let bits =
                        u64::from_le_bytes([blk[2], blk[3], blk[4], blk[5], blk[6], blk[7], 0, 0]);
                    let mut a = [0u8; 16];
                    for (i, v) in a.iter_mut().enumerate() {
                        *v = table[((bits >> (3 * i)) & 7) as usize] as u8;
                    }
                    (&blk[8..], a)
                }
            };
            let pal = decode_bc_color(color_blk, kind == 1);
            let idx = u32::from_le_bytes([color_blk[4], color_blk[5], color_blk[6], color_blk[7]]);
            for py in 0..4 {
                for px in 0..4 {
                    let (x, y) = (bx * 4 + px, by * 4 + py);
                    if x >= w || y >= h {
                        continue;
                    }
                    let i = py * 4 + px;
                    let mut c = pal[((idx >> (2 * i)) & 3) as usize];
                    if kind != 1 {
                        c[3] = alpha[i];
                    }
                    out[(y * w + x) * 4..][..4].copy_from_slice(&c);
                }
            }
        }
    }
    Some(out)
}

impl CpuImage {
    /// Decodes mip level 0 of a Bevy [`Image`] (RGBA8, R8 masks, BC1/BC2/BC3).
    #[must_use]
    pub fn from_image(img: &Image) -> Option<Self> {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let data = img.data.as_deref()?;
        let rgba = match img.texture_descriptor.format {
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => {
                (data.len() >= w * h * 4).then(|| data[..w * h * 4].to_vec())?
            }
            TextureFormat::R8Unorm => {
                (data.len() >= w * h).then_some(())?;
                data[..w * h].iter().flat_map(|&a| [a, a, a, a]).collect()
            }
            TextureFormat::Bc1RgbaUnorm | TextureFormat::Bc1RgbaUnormSrgb => {
                decode_bc(data, w, h, 1)?
            }
            TextureFormat::Bc2RgbaUnorm | TextureFormat::Bc2RgbaUnormSrgb => {
                decode_bc(data, w, h, 2)?
            }
            TextureFormat::Bc3RgbaUnorm | TextureFormat::Bc3RgbaUnormSrgb => {
                decode_bc(data, w, h, 3)?
            }
            _ => return None,
        };
        Some(Self {
            width: w,
            height: h,
            rgba,
        })
    }

    /// Bilinear sample with wrap-around; `u`, `v` in texture units (1.0 = one repeat).
    pub(crate) fn sample_wrap(&self, u: f32, v: f32) -> [f32; 4] {
        let (x, y) = (
            u.rem_euclid(1.0) * self.width as f32 - 0.5,
            v.rem_euclid(1.0) * self.height as f32 - 0.5,
        );
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |xx: f32, yy: f32| {
            let xi = (xx as i64).rem_euclid(self.width as i64) as usize;
            let yi = (yy as i64).rem_euclid(self.height as i64) as usize;
            let p = &self.rgba[(yi * self.width + xi) * 4..][..4];
            [
                f32::from(p[0]),
                f32::from(p[1]),
                f32::from(p[2]),
                f32::from(p[3]),
            ]
        };
        let (a, b, c, d) = (
            at(x0, y0),
            at(x0 + 1.0, y0),
            at(x0, y0 + 1.0),
            at(x0 + 1.0, y0 + 1.0),
        );
        let mut out = [0.0; 4];
        for k in 0..4 {
            out[k] =
                (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy;
        }
        out
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(rgb: [u8; 3], a: u8) -> CpuImage {
        CpuImage {
            width: 2,
            height: 2,
            rgba: [rgb[0], rgb[1], rgb[2], a].repeat(4),
        }
    }

    #[test]
    fn mask_blends_layers_and_row_zero_is_north() {
        let base = solid([0, 0, 255], 255);
        let paint = solid([255, 0, 0], 255);
        // 2x2 mask: opaque only in the top row (north).
        let mask = CpuImage {
            width: 2,
            height: 2,
            rgba: vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let layers = [
            GroundLayer {
                texture: &base,
                repeats: 1.0,
                mask: None,
            },
            GroundLayer {
                texture: &paint,
                repeats: 1.0,
                mask: Some(&mask),
            },
        ];
        let img = compose_tile_ground(&layers, 4);
        let px = |x: usize, y: usize| &img.data[(y * 4 + x) * 4..][..3];
        assert_eq!(px(1, 0), [255, 0, 0], "north half is painted");
        assert_eq!(px(1, 3), [0, 0, 255], "south half keeps the base");
    }

    #[test]
    fn bc1_block_decodes_to_its_endpoint_colours() {
        // c0 = pure red (0xF800), c1 = pure blue (0x001F), indices all 0 -> red.
        let block = [0x00, 0xF8, 0x1F, 0x00, 0, 0, 0, 0];
        let out = decode_bc(&block, 4, 4, 1).expect("decodes");
        assert_eq!(&out[..4], [255, 0, 0, 255]);
    }
}
