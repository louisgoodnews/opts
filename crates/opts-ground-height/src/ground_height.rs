//! The ground index of the resident tiles: road samples, terrain and surface plates by cell. The
//! systems that fill and drop it stay in `opts-game`.

use opts_bevy::bevy::prelude::*;
use opts_bevy::scenery::spline_pose;
use opts_bevy::TileTerrain;
use opts_collision::{ColliderGrid, Obb};
use opts_core::formats::map::{MapSplineInstance, TileGrid};
use opts_core::formats::scenery::SplineProfile;
use std::collections::HashMap;

/// Spacing of the road centre-line samples (m).
const SAMPLE_STEP_M: f32 = 2.0;
/// Cell size of the sample index (m).
const CELL_M: f32 = 8.0;
/// How much closer than a road layer the terrain must be to the vehicle's height to win (m).
const TERRAIN_LAYER_MARGIN_M: f32 = 0.3;
/// A road layer this far above the vehicle is a deck over it, not the ground (m).
const DECK_GAP_M: f32 = 1.0;

mod index;
mod insert;
mod query;
mod sample;

use index::cell_of;
pub use index::GroundIndex;
use sample::RoadSample;

#[cfg(test)]
mod tests;
