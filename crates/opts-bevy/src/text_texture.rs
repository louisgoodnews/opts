//! Text textures: rasterises `[texttexture]` strings with OMSI bitmap fonts (`Fonts/*.oft`).
//!
//! A font is a colour bitmap plus an alpha bitmap; the `.oft` file gives each character's cell.
//! The text is laid out at native pixel size on one line, aligned per `orientation`, and shrunk
//! uniformly when it does not fit. Layout rules and open questions: `docs/src/architecture/text-textures.md`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::asset::{Assets, Handle};
use bevy::image::Image;
#[allow(unused_imports)] // `OftChar` is for the tests of `script_text`
pub(crate) use opts_core::formats::oft::{OftChar, OftFile, OftFont};

use crate::install::OmsiInstall;

mod gpu;
pub use gpu::{led_image, rebind_redrawn_images, text_image, text_image_addressed, RedrawnImages};

/// What to draw: one `[texttexture]` slot resolved against one object instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextSpec {
    /// Font name from the `.sco`.
    pub font: String,
    /// The resolved string.
    pub text: String,
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// `true`: draw in `rgb`; `false`: keep the font bitmap's own colours.
    pub mono: bool,
    /// Text colour for mono rendering.
    pub rgb: [u8; 3],
    /// 0 centred, 1 left, 2 right.
    pub orientation: u32,
}

mod files;
mod fonts;
mod registry;
mod render;

pub use files::*;
pub use fonts::*;
pub use registry::*;
pub use render::*;

#[cfg(test)]
mod tests;
