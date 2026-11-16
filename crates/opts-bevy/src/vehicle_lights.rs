//! Script-driven vehicle lights (M-0045): `[interiorlight]`, `[spotlight]` and `[matl_lightmap]`.
//!
//! Rules (`docs/src/formats/text/vehicles/model_cfg.md`):
//!
//! * an `[interiorlight]` shines with its variable as brightness factor (0 off, 1 normal, 2
//!   double; values above ~3 look the same, so the factor is clamped to `0..=3`);
//! * of all `[spotlight]`s only the one whose 0-based index equals `Spot_Select` shines; `-1`
//!   switches all off (the SD200 `lights.osc` writes `0` for high beam, `1` for dipped beam and
//!   `-1` otherwise, matching the declaration order of its two spotlights);
//! * a `[matl_lightmap]` texture is emissive light scaled by its variable.
//!
//! Lights of vehicles without a script engine keep their spawn state.

use bevy::prelude::*;

use crate::ai_variables::{AiVariables, VariableSource};
use crate::vehicle_script::VehicleScripts;
mod alpha;
mod bindings;
mod lamps;
mod light_map;

pub use alpha::{scaled_alpha, vehicle_alpha_scale_system};
pub use bindings::{AlphaScaleBinding, LightMapBinding, ALPHA_VISIBLE_MIN};
pub use lamps::vehicle_light_system;
pub use light_map::vehicle_light_map_system;

/// Upper bound of a brightness variable; brighter values look the same in OMSI.
pub const MAX_BRIGHTNESS: f32 = 3.0;
/// Emission of a light map at factor 1. The emissive picture is the material's texture lit by the
/// light map (`crate::model::compose_light`), and emission is drawn without the camera's exposure
/// (`emissive_exposure_weight` 0), so 1 shows a lit display or window in its texture's own
/// colours, as scenery light maps do. (It was 100 while the light map itself was the emitted
/// colour: every lit area went white, the MB C2's display among them.)
pub const LIGHT_MAP_GAIN: f32 = 1.0;

/// What switches a vehicle light.
#[derive(Debug, Clone, PartialEq)]
pub enum LightSwitch {
    /// Brightness factor from a script variable (`[interiorlight]`).
    Variable(String),
    /// Spotlight number: on while `Spot_Select` equals it (`[spotlight]`).
    Spot(usize),
}

/// A point or spot light of a vehicle that follows the vehicle's script variables.
#[derive(Component, Debug, Clone)]
pub struct ScriptLight {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// What switches the light.
    pub switch: LightSwitch,
    /// Intensity at brightness factor 1.
    pub base_intensity: f32,
}

impl ScriptLight {
    /// Brightness factor for the current variable values (`0..=MAX_BRIGHTNESS`).
    #[must_use]
    pub fn factor(&self, value: impl Fn(&str) -> f32) -> f32 {
        match &self.switch {
            LightSwitch::Variable(name) => value(name).clamp(0.0, MAX_BRIGHTNESS),
            LightSwitch::Spot(i) => {
                let selected = value("spot_select").round();
                if selected >= 0.0 && selected as usize == *i {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Registers the light systems after the scripts ran.
#[derive(Default)]
pub struct VehicleLightPlugin;

impl Plugin for VehicleLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                vehicle_light_system,
                vehicle_light_map_system,
                vehicle_alpha_scale_system,
                crate::free_texture::free_texture_system,
            ),
        );
    }
}

#[cfg(test)]
mod tests;
