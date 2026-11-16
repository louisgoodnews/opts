//! Light effects of vehicles (`[light_enh]`, `[light_enh_2]` in `model.cfg`) and a small pool of
//! real headlights.
//!
//! * **Glow sprites:** every light effect becomes a sprite under the vehicle's root (not under a
//!   LOD group, so it never pops with the meshes), drawn by the scenery light system
//!   ([`crate::scenery_runtime::scenery_light_system`]: size by brightness, fade by the cone).
//!   Brightness = variable × factor, clamped to `0..=1`, approached with the block's time
//!   constant. The variable comes from the vehicle's scripts or its AI variable table
//!   ([`crate::ai_variables::VariableSource`]), so player and AI vehicles share the code.
//! * **Headlight pool:** forward-facing white light effects mark a vehicle's headlights. Of the
//!   vehicles whose headlights shine, the nearest [`HEADLIGHT_POOL`] get one real, shadowless
//!   `SpotLight` each (at the middle of the pair), with hysteresis so the choice is stable under
//!   small camera moves. Everything else is only a glow (hundreds of spot lights would cost too
//!   much).
//!
//! Design: `docs/src/architecture/ai-traffic-kinematics.md`, §5.

use bevy::prelude::*;
use opts_core::formats::scenery::ScoLight;

mod spawn;
mod system;

pub use spawn::{detail_lights, spawn_vehicle_glows, spawn_vehicle_glows_with_cones};
pub use system::{attach_glow_assets, vehicle_glow_system, VehicleGlowAssets};

/// Number of real headlights (one spot light per vehicle).
pub const HEADLIGHT_POOL: usize = 6;
/// Headlights shine (for the pool) above this brightness.
const HEADLIGHT_ON: f32 = 0.3;

/// A light effect of a vehicle; the entity also carries the [`crate::scenery_runtime::SceneryLight`] that is drawn.
#[derive(Component, Debug, Clone)]
pub struct VehicleGlow {
    /// The vehicle root whose scripts or AI variables drive the light.
    pub owner: Entity,
    /// Brightness variable (empty: always on).
    pub variable: String,
    /// Multiplier on the variable.
    pub factor: f32,
    /// Seconds to reach 63 % of a new brightness (`0` = at once).
    pub time_const_s: f32,
    /// Colour, 0–255.
    pub color: [f32; 3],
    /// The light is a headlight (shines forward, white).
    pub headlight: bool,
}

impl VehicleGlow {
    /// The brightness after `dt` seconds towards `target` from `level`.
    #[must_use]
    pub fn approach(&self, level: f32, target: f32, dt: f32) -> f32 {
        if self.time_const_s <= 1e-3 {
            target
        } else {
            level + (target - level) * (1.0 - (-dt / self.time_const_s).exp())
        }
    }
}

/// Whether a light effect is a headlight: directed forward (OMSI +y) and white-ish.
#[must_use]
pub fn is_headlight(light: &ScoLight) -> bool {
    !light.omni && light.direction.y > 0.7 && light.color.iter().all(|c| *c >= 150.0)
}

mod headlight_pool;
pub use headlight_pool::{
    choose_headlights, headlight_pool_system, PooledHeadlight, HEADLIGHT_HYSTERESIS,
};

/// Registers the vehicle glow and headlight pool systems.
#[derive(Default)]
pub struct VehicleGlowPlugin;

impl Plugin for VehicleGlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleGlowAssets>().add_systems(
            Update,
            (
                attach_glow_assets,
                vehicle_glow_system.before(crate::scenery_runtime::SceneryRuntimeSet),
                headlight_pool_system,
            ),
        );
    }
}

#[cfg(test)]
mod tests;
