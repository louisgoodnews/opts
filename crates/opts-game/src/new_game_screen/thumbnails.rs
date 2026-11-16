//! Thumbnails and hero images: decoded off the main thread, cached as small PNGs under the user
//! cache directory (with a size cap) and put into the slots the lists and details provide.
//!
//! Maps use the `picture.jpg` of their folder. Vehicles use a preview image when the content has
//! one (`picture`/`preview`/`thumbnail` next to the `.bus` file); none of the shipped vehicles
//! does, so they keep the neutral placeholder glyph — nothing is invented. Livery swatches take
//! the average colour of the repaint's first readable texture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_gui::patterns::{RowThumb, ThumbGlyph};

use super::livery::{LiveryState, LiverySwatch};
use super::step_where::HeroImage;
use super::types::GameCatalogue;

/// The placeholder glyph inside a hero or thumbnail slot; removed when the image arrives.
#[derive(Component, Debug, Clone, Copy)]
pub struct HeroGlyph;

/// A slot that has been dealt with (filled, or nothing to show).
#[derive(Component, Debug, Clone, Copy)]
pub struct ThumbDone;

/// Default cache size cap.
pub const DEFAULT_CAP_BYTES: u64 = 64 * 1024 * 1024;
/// Size of a map hero image.
pub const HERO_SIZE: (u32, u32) = (320, 180);
/// Size of a square thumbnail.
pub const SQUARE_SIZE: (u32, u32) = (96, 96);

/// What a finished request produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Thumb {
    /// Decoded pixels.
    Pixels {
        /// Width.
        width: u32,
        /// Height.
        height: u32,
        /// RGBA8 data.
        rgba: Vec<u8>,
    },
    /// An average colour (livery swatch).
    Colour([u8; 3]),
}

mod cache;
mod decode;
mod service;
pub use cache::{load_thumb, prune};
pub use decode::average_colour;
use decode::{pixels, to_image};
pub use service::{Lookup, ThumbnailService};

mod config_slot;
mod slots;
pub use config_slot::{config_thumb_key, spawn_config_thumb, CONFIG_THUMB_PX};
pub use slots::{fill_thumbnails, probe_of, source_of, vehicle_preview, Probe};

#[cfg(test)]
mod tests;
