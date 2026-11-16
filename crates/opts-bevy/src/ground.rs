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
mod bc;
mod compose;

pub use compose::*;
use bc::*;

#[derive(Debug, Clone)]
pub struct CpuImage {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// `width * height * 4` bytes, RGBA.
    pub rgba: Vec<u8>,
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

#[cfg(test)]
mod tests;
