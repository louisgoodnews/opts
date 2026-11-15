//! Texture format sniffing, DirectDraw Surface (`.dds`) decoding, and fallback image loaders.
//!
//! OMSI assets use `.dds`, `.bmp`, `.tga`, `.png`, and `.jpg` image files.
//! Notably, over 1,170 corpus files have file extensions that do not match their
//! underlying content (such as BMP files named with `.dds` extensions).
//! This module provides magic-byte content sniffing and decoder routing into native
//! Bevy [`Image`] assets.

#![deny(unsafe_code)]

mod bc_mips;
pub mod chain_cache;
mod dds_chain;
mod decode;
mod dds_image;
mod error;
mod format;
mod mips;
mod pixel;
pub use bc_mips::{
    bc_chain, bc_chain_stats, prepare_block_compressed, BcChainStats, GENERATOR_VERSION,
};
pub use decode::*;
pub use dds_image::{dds_to_bevy_image, dropped_dds_chains};
pub use error::TextureError;
pub use format::{sniff_image_format, ImageFormat};
pub use mips::{
    full_chain_levels, generate_mip_chain, level_size, looks_like_cutout, mip_stats,
    rgba8_srgb_chain, MipOptions, MipStats,
};

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[cfg(test)]
use self::pixel::convert_rgb16_to_rgba;
#[cfg(test)]
mod tests;
