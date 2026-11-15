//! CPU mip-chain generation for uncompressed colour textures (M-0058 phase B).
//!
//! About 83 % of the corpus textures ship without a mipmap chain, and a texture without one
//! aliases as soon as it covers fewer screen pixels than texels. [`generate_mip_chain`] appends
//! the missing levels to an `Rgba8UnormSrgb` [`Image`]:
//!
//! - **sRGB-correct**: texels are averaged in linear light (a black/white checker becomes 188,
//!   not 128);
//! - **any size**: level `n` has `max(1, size >> n)` texels per axis (the layout wgpu expects), and
//!   every texel averages the exact (fractional) source area it covers, so non-power-of-two
//!   textures need no padding;
//! - **alpha** is averaged linearly and left alone, unless a coverage cutoff is given
//!   ([`MipOptions::alpha_cutoff`]): then colours are weighted by alpha and each level's alpha is
//!   scaled so that the share of texels at or above the cutoff stays that of the base level
//!   (foliage and fences do not thin out with distance; used by M-0059).
//!
//! Block-compressed textures and 8-bit masks are not touched.

use std::sync::OnceLock;

use bevy::image::Image;
use bevy::render::render_resource::TextureFormat;

/// Options of [`generate_mip_chain`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MipOptions {
    /// Alpha-coverage mode: the alpha cutoff (`0..=1`) of the material the texture is used with.
    /// `None` leaves alpha as the plain average.
    pub alpha_cutoff: Option<f32>,
    /// Weights colours by alpha when averaging (always on in coverage mode), so transparent texels
    /// with black RGB do not darken the colour of the visible ones. For glyph and sign textures.
    pub weight_colour_by_alpha: bool,
}

/// Number of levels of a full chain for a texture of `width` x `height` texels.
#[must_use]
pub fn full_chain_levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// Size of mip level `level` (never below 1).
#[must_use]
pub fn level_size(width: u32, height: u32, level: u32) -> (u32, u32) {
    ((width >> level).max(1), (height >> level).max(1))
}

mod srgb;
use srgb::*;

mod downsample;
use downsample::*;

mod general;
use general::*;

mod cutout;
pub use cutout::*;

mod stats;
pub use stats::*;


#[cfg(test)]
mod tests;
