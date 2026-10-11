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

use crate::vehicle_script::VehicleScripts;

/// Upper bound of a brightness variable; brighter values look the same in OMSI.
pub const MAX_BRIGHTNESS: f32 = 3.0;

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

/// A mesh part whose material has a light map driven by a script variable.
///
/// The part's material may be shared with other vehicles of the same model; it is replaced by a
/// private copy the first time the owner runs scripts (parked vehicles never pay for the copy).
#[derive(Component, Debug, Clone)]
pub struct LightMapBinding {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// Brightness variable.
    pub variable: String,
    /// The material is this vehicle's own copy.
    pub private: bool,
    /// Last applied factor (`NaN` before the first update).
    pub applied: f32,
}

impl LightMapBinding {
    /// Binds `variable` for the vehicle `owner`.
    #[must_use]
    pub fn new(owner: Entity, variable: impl Into<String>) -> Self {
        Self {
            owner,
            variable: variable.into(),
            private: false,
            applied: f32::NAN,
        }
    }
}

/// Sets point and spot light intensities from the owners' script variables.
pub fn vehicle_light_system(
    mut lights: Query<(
        &ScriptLight,
        Option<&mut PointLight>,
        Option<&mut SpotLight>,
        &mut Visibility,
    )>,
    vehicles: Query<&VehicleScripts>,
) {
    for (light, point, spot, mut visibility) in &mut lights {
        let Ok(scripts) = vehicles.get(light.owner) else {
            continue;
        };
        let Ok(engine) = scripts.engine.lock() else {
            continue;
        };
        let f = light.factor(|n| engine.get(n));
        let intensity = light.base_intensity * f;
        if let Some(mut p) = point {
            if p.intensity != intensity {
                p.intensity = intensity;
            }
        }
        if let Some(mut s) = spot {
            if s.intensity != intensity {
                s.intensity = intensity;
            }
        }
        // A dark light is hidden so it costs nothing (shadow maps, clustering).
        let want = if f > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Scales light-map emission by the owners' script variables.
pub fn vehicle_light_map_system(
    mut parts: Query<(&mut LightMapBinding, &mut MeshMaterial3d<StandardMaterial>)>,
    vehicles: Query<&VehicleScripts>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut binding, mut material) in &mut parts {
        let Ok(scripts) = vehicles.get(binding.owner) else {
            continue;
        };
        let f = {
            let Ok(engine) = scripts.engine.lock() else {
                continue;
            };
            engine.get(&binding.variable).clamp(0.0, MAX_BRIGHTNESS)
        };
        if f == binding.applied {
            continue;
        }
        if !binding.private {
            let Some(copy) = materials.get(&material.0).cloned() else {
                continue;
            };
            material.0 = materials.add(copy);
            binding.private = true;
        }
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.emissive = LinearRgba::rgb(f, f, f);
        }
        binding.applied = f;
    }
}

/// Registers the light systems after the scripts ran.
#[derive(Default)]
pub struct VehicleLightPlugin;

impl Plugin for VehicleLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (vehicle_light_system, vehicle_light_map_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(switch: LightSwitch) -> ScriptLight {
        ScriptLight {
            owner: Entity::PLACEHOLDER,
            switch,
            base_intensity: 100.0,
        }
    }

    #[test]
    fn interior_light_follows_its_variable_clamped() {
        let l = light(LightSwitch::Variable("lights_cab".into()));
        assert_eq!(l.factor(|_| 0.0), 0.0);
        assert_eq!(l.factor(|_| 0.5), 0.5);
        assert_eq!(l.factor(|_| 7.0), MAX_BRIGHTNESS);
        assert_eq!(l.factor(|_| -1.0), 0.0);
    }

    #[test]
    fn only_the_selected_spotlight_shines() {
        let high = light(LightSwitch::Spot(0));
        let dipped = light(LightSwitch::Spot(1));
        let select = |v: f32| move |n: &str| if n == "spot_select" { v } else { 0.0 };
        assert_eq!(
            (high.factor(select(0.0)), dipped.factor(select(0.0))),
            (1.0, 0.0)
        );
        assert_eq!(
            (high.factor(select(1.0)), dipped.factor(select(1.0))),
            (0.0, 1.0)
        );
        assert_eq!(
            (high.factor(select(-1.0)), dipped.factor(select(-1.0))),
            (0.0, 0.0)
        );
    }
}
