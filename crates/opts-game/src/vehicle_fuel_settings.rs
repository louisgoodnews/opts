//! The setting *Fuel and battery* (M-0123): how much of the consumption applies. `Off` keeps the
//! level and the torque as they are, `Realistic` burns at the model's rates and `Accelerated`
//! burns `ACCELERATED_FACTOR` (10) times as fast. The value feeds every
//! session vehicle's fuel model through [`sync_fuel_mode`].

use bevy::prelude::*;
use opts_bevy::{VehicleController, VehicleSetup};
use opts_sim::vehicle::fuel::{FuelKind, FuelMode, DEFAULT_CAPACITY};

use crate::settings::GameSettings;
use crate::settings_registry::descriptor::{
    opt, Category as C, SettingDescriptor as D, SettingKind as K,
};

/// `vehicle.fuel_mode`: `off`, `realistic` or `accelerated`.
pub const FUEL_MODE_KEY: &str = "vehicle.fuel_mode";

const FUEL_MODES: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-fuel-mode-off"),
    opt("realistic", "set-opt-fuel-mode-realistic"),
    opt("accelerated", "set-opt-fuel-mode-accelerated"),
];

/// The descriptor of the setting.
#[must_use]
pub fn descriptors() -> Vec<D> {
    vec![D::new(
        FUEL_MODE_KEY,
        K::Enum(FUEL_MODES),
        "realistic",
        C::Vehicle,
        40,
        "set-vehicle-fuel-mode",
        "set-vehicle-fuel-mode-desc",
    )
    .keywords(&["fuel", "battery", "consumption", "diesel", "range", "empty"])
    .since("0.4")]
}

/// The stored consumption choice (`realistic` for a missing or unknown value).
#[must_use]
pub fn fuel_mode_of(settings: &GameSettings) -> FuelMode {
    settings
        .raw(FUEL_MODE_KEY)
        .map_or(FuelMode::Realistic, |v| FuelMode::parse(&v))
}

/// Gives each session vehicle's fuel model the chosen mode. A vehicle without a model gets one
/// that starts at the player's fuel share ([`VehicleSetup::fuel`]).
pub fn sync_fuel_mode(
    settings: Res<GameSettings>,
    setup: Option<Res<VehicleSetup>>,
    mut vehicles: Query<&mut VehicleController>,
) {
    let mode = fuel_mode_of(&settings);
    let share = setup.map_or(VehicleSetup::default().fuel, |s| s.fuel);
    for mut controller in &mut vehicles {
        let sim = &mut controller.sim;
        if sim.fuel.as_ref().map(|f| f.mode) == Some(mode) {
            continue;
        }
        let fresh = sim.fuel.is_none();
        sim.ensure_fuel(FuelKind::Diesel, DEFAULT_CAPACITY, mode);
        if fresh {
            if let Some(fuel) = sim.fuel.as_mut() {
                fuel.set_level(share);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_value_is_realistic() {
        assert_eq!(fuel_mode_of(&GameSettings::default()), FuelMode::Realistic);
    }

    #[test]
    fn the_key_and_default_are_registered_in_the_vehicle_category() {
        let r = crate::settings_registry::SettingsRegistry::with_builtin();
        let d = r.get(FUEL_MODE_KEY).expect("registered");
        assert_eq!(d.category, C::Vehicle);
    }
}
