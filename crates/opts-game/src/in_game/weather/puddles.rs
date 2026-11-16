//! Puddles on roads and pavements while it rains.
//!
//! The terrain has puddles in its shader (`ground_layers.wesl`); the splines (roads, pavements,
//! parking bays) are plain materials and only get darker and smoother when wet. This module adds
//! the puddles there: flat, irregular, nearly mirror-smooth patches of dark water laid on the
//! road surface around the camera, as many as the weather's puddle coverage and the wetness ask
//! for, so a wet street shows patches that reflect the sky and the sun's glint at grazing angles
//! (roughness 0.03, the sky cube of `opts_bevy::reflections` as the environment).
//!
//! * **Where.** Random points in a ring around the camera ([`MIN_DISTANCE_M`]..[`MAX_DISTANCE_M`]);
//!   a point counts when the [`GroundIndex`] says a road lies there and the road is level across
//!   the patch (within [`FLAT_TOLERANCE_M`] over its extent), so patches never float on a slope.
//! * **How many.** [`puddle_target`]: [`MAX_PUDDLES`] at full coverage and wetness, none when it
//!   is dry or snow covers the ground. At most [`SPAWN_PER_TICK`] appear per tick; puddles left
//!   behind farther than [`DROP_DISTANCE_M`] and those above the target are removed.
//! * **Intensity.** The rain intensity setting (`graphics.rain_intensity`) scales coverage and
//!   wetness, so more rain means more puddles.

mod spawn;
mod update;

pub use update::{update_puddles, PuddleInputs};

use bevy::prelude::*;
use opts_bevy::weather::SurfaceState;

/// Most puddles at once.
pub const MAX_PUDDLES: usize = 80;
/// Nearest a puddle is placed to the camera (m).
pub const MIN_DISTANCE_M: f32 = 4.0;
/// Farthest a puddle is placed from the camera (m).
pub const MAX_DISTANCE_M: f32 = 55.0;
/// A puddle farther than this from the camera is removed (m).
pub const DROP_DISTANCE_M: f32 = 75.0;
/// Seconds between two ticks.
const TICK_S: f32 = 0.25;
/// Most puddles added per tick.
pub const SPAWN_PER_TICK: usize = 3;
/// How much the road may rise or fall across a puddle (m).
pub const FLAT_TOLERANCE_M: f32 = 0.02;
/// How far above the road a puddle lies (m).
const LIFT_M: f32 = 0.012;
/// Brightness (nits) of the sky colour the water shows at a full-bright sky.
const SKY_NITS: f32 = 1.2;
/// Tries to find a place per tick.
const TRIES_PER_TICK: usize = 24;

/// Marks a puddle.
#[derive(Component, Debug, Clone, Copy)]
pub struct Puddle {
    /// Position in the ground index frame (east, north).
    pub at: Vec2,
}

/// The puddles of the drive.
#[derive(Resource)]
pub struct PuddleState {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
    timer: f32,
    rng: u64,
}

impl Default for PuddleState {
    fn default() -> Self {
        Self {
            mesh: None,
            material: None,
            timer: 0.0,
            rng: 0x5EED_1234_ABCD_9876,
        }
    }
}

impl PuddleState {
    fn next(&mut self) -> f32 {
        // xorshift64*
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        let v = self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (v >> 40) as f32 / 16_777_216.0
    }
}

/// How many puddles the surface state asks for: coverage (`0..=0.45`) over its full value times
/// the wetness, times [`MAX_PUDDLES`].
#[must_use]
pub fn puddle_target(state: &SurfaceState, wet: f32) -> usize {
    let coverage = (state.puddles / 0.45).clamp(0.0, 1.0);
    (coverage * wet.clamp(0.0, 1.0) * MAX_PUDDLES as f32).round() as usize
}

/// The opacity of a puddle's picture at `(u, v)` in `0..1`: an irregular blob, opaque in the
/// middle and soft at the edge.
#[must_use]
pub fn blob_alpha(u: f32, v: f32) -> f32 {
    let (x, y) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    let d = x.hypot(y);
    let a = y.atan2(x);
    // The edge of the blob wobbles with the angle.
    let edge = 0.78 + 0.1 * (3.0 * a + 0.7).sin() + 0.06 * (5.0 * a + 2.1).sin();
    let t = ((edge - d) / 0.28).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests;
