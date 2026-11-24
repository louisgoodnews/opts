//! The engine exhaust plumes of the vehicles nearest the camera (M-0117): puffs from the back of
//! each vehicle with an [`ExhaustOutlet`] (and of the player's vehicle, always), at the rate of
//! [`opts_sim::exhaust::puff_rate_per_s`] (air temperature and throttle; AI vehicles at
//! [`AI_LOAD`]), scaled to zero by the distance to the camera ([`fade_of`]). The player's vehicle
//! and the [`NEAREST_N`] nearest others put out puffs; at most `cap` puffs are alive at once across
//! all of them, and nothing is emitted with `graphics.exhaust = off`.
//!
//! The anchor is the vehicle model's exhaust `[smoke]` emitter ([`ExhaustOutlet`]) when it defines
//! one; otherwise the fallback, 1 m above the ground and [`REAR_OFFSET_M`] behind the origin. Each
//! puff rises and widens on its own straight path and is removed after
//! [`opts_sim::exhaust::PUFF_LIFETIME_S`].

use std::collections::HashMap;

use bevy::prelude::*;

mod emit;
mod nearest;
mod outlet;
mod select;
pub use emit::{update_exhaust, ExhaustAssets, ExhaustInputs};
pub use nearest::{fade_of, nearest, AI_LOAD, FADE_END_M, FADE_START_M, NEAREST_N};
pub use outlet::{chain_outlet, fallback_outlet_of, outlet_of, OUTLET_HEIGHT_M, REAR_OFFSET_M};

/// Edge length (m) of a puff at birth; it grows to twice this over its life.
pub(crate) const PUFF_SIZE_M: f32 = 0.35;
/// Velocity (m/s) of a puff: up and back, slightly to the side of the fall of the air.
pub(crate) const DRIFT_M_PER_S: Vec3 = Vec3::new(0.0, 0.8, -0.6);

/// One visible puff: its age and the point it left the outlet from.
#[derive(Component, Debug, Clone, Copy)]
pub struct Puff {
    pub(crate) age: f32,
    pub(crate) origin: Vec3,
    pub(crate) drift: Vec3,
}

/// The carried fraction of each emitter (by entity) and the shared mesh and material of the puffs.
#[derive(Resource, Default)]
pub struct ExhaustState {
    pub(crate) carry: HashMap<Entity, f32>,
    pub(crate) assets: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
}

/// The exhaust settings of the session, copied in each frame by the game: whether the plume is
/// drawn and the cap on the live puffs (`graphics.exhaust`, `graphics.exhaust_particles`).
#[derive(Resource, Debug, Clone, Copy)]
pub struct ExhaustSettings {
    pub on: bool,
    pub cap: u32,
}

impl Default for ExhaustSettings {
    fn default() -> Self {
        Self { on: true, cap: 12 }
    }
}

/// Where a puff of age `age` is: it rises and drifts back with the wind direction `drift`.
#[must_use]
pub fn puff_position(puff: &Puff) -> Vec3 {
    puff.origin + puff.drift * puff.age
}

#[cfg(test)]
mod tests;
