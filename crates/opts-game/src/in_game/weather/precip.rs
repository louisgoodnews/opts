//! Rain and snow: one dynamic mesh of camera-facing quads around the camera, rebuilt every
//! frame, at most as many quads as the quality setting allows (`graphics.weather_precipitation`).
//!
//! A particle has no state: its place is a pure function of its fixed random seed and the time,
//! wrapped into a box around the camera, so particles stay where they are in the world while the
//! camera moves through them, and nothing has to be spawned or removed. Rain is drawn as streaks
//! along the fall direction (the wind slants it), snow as slowly falling flakes that flutter. The
//! effects use real time, so a preview on the paused *Environment* page keeps falling.

mod assets;
mod geometry;
mod layer;
mod state;
mod system;

pub(super) use assets::texture;
pub use geometry::{flake_corners, particle_position, rain_corners, seed_of};
pub use state::{PrecipLayer, PrecipState};
pub use system::update_precipitation;

/// Edge of the box (metres) the particles fill horizontally, centred on the camera.
pub const BOX_XZ: f32 = 44.0;
/// Height of the box; it reaches 8 m below and 12 m above the camera.
pub const BOX_Y: f32 = 20.0;

#[cfg(test)]
use assets::quad_mesh;
#[cfg(test)]
use bevy::prelude::*;
#[cfg(test)]
use geometry::BELOW_CAMERA;
#[cfg(test)]
use opts_bevy::weather::{precip_plan, PrecipPlan};
#[cfg(test)]
use opts_core::formats::weather::PrecipKind;

#[cfg(test)]
mod tests;
