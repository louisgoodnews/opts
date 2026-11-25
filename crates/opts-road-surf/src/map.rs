//! The height maps of road textures: a decoded `.surf` bitmap, or a synthetic one from a surface
//! type.

use super::{roughness_of, SURF_AMPLITUDE_M};

/// Cells of a synthetic height map along each side.
const NOISE_CELLS: usize = 32;

/// A decoded `.surf` height map.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfMap {
    width: usize,
    height: usize,
    /// Heights in metres, row by row from the top of the image.
    pub(super) heights: Vec<f32>,
}

impl SurfMap {
    /// Decodes a `.surf` file (a BMP or another image format): the red channel, mid-grey level.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let img = image::load_from_memory(bytes).ok()?.to_rgb8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        if w == 0 || h == 0 {
            return None;
        }
        let heights = img
            .pixels()
            .map(|p| (f32::from(p.0[0]) - 127.5) / 127.5 * SURF_AMPLITUDE_M)
            .collect();
        Some(Self {
            width: w,
            height: h,
            heights,
        })
    }

    /// A synthetic height map of random bumps up to `amplitude_m` (deterministic for `seed`),
    /// sampled in metres when its strip has a [`TexStrip::period_m`].
    #[must_use]
    pub fn noise(seed: u64, amplitude_m: f32) -> Self {
        let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
        let heights = (0..NOISE_CELLS * NOISE_CELLS)
            .map(|_| {
                // xorshift64*
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                let r =
                    (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32;
                (r * 2.0 - 1.0) * amplitude_m
            })
            .collect();
        Self {
            width: NOISE_CELLS,
            height: NOISE_CELLS,
            heights,
        }
    }

    /// The synthetic map of a surface type ([`roughness_of`]) and the length it repeats over (m).
    #[must_use]
    pub fn synthetic(surface: u32) -> (Self, f32) {
        let (amplitude, bump) = roughness_of(surface);
        (
            Self::noise(u64::from(surface) + 1, amplitude),
            bump * NOISE_CELLS as f32,
        )
    }

    /// A map from heights (tests).
    #[must_use]
    pub fn from_heights(width: usize, height: usize, heights: Vec<f32>) -> Self {
        Self {
            width,
            height,
            heights,
        }
    }

    /// Height at texture coordinates `(u, v)` (repeating), bilinear between the pixel centres.
    #[must_use]
    pub fn height_at(&self, u: f32, v: f32) -> f32 {
        let fx = u.rem_euclid(1.0) * self.width as f32 - 0.5;
        let fy = v.rem_euclid(1.0) * self.height as f32 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let wrap = |i: f32, n: usize| (i as i64).rem_euclid(n as i64) as usize;
        let at =
            |x: f32, y: f32| self.heights[wrap(y, self.height) * self.width + wrap(x, self.width)];
        let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx;
        let bottom = at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}
