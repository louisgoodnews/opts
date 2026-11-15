//! The overlay of the full-size map: root, texture node, caption, scale bar and keys.

use super::draw::FullMapImage;
use super::viewport::scale_bar_m;
use super::*;

/// The root of the overlay.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapRoot;

/// The node that shows the texture (a square that covers the screen).
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapSurface;

/// The map name and the zoom.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapCaption;

/// The bar of the scale.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapScaleBar;

/// The length of the scale bar as text.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapScaleText;
mod panels;
mod spawn;
mod system;
pub use system::overlay;
use panels::*;
use spawn::*;

