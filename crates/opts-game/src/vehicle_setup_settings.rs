//! The *Vehicle* settings (M-0141): how far a vehicle is set up when a drive begins, how much
//! fuel it has and which lights are on. They feed [`opts_bevy::VehicleSetup`], which the start-up
//! of the vehicles' scripts follows (`opts_bevy::vehicle_setup`).

use bevy::prelude::*;
use opts_bevy::{StartLights, VehicleSetup};

use crate::settings::GameSettings;
use crate::settings_registry::descriptor::{
    opt, Category as C, SettingDescriptor as D, SettingKind as K,
};

/// `vehicle.setup`: 0 (cold) to 1 (ready to drive).
pub const SETUP_KEY: &str = "vehicle.setup";
/// `vehicle.fuel`: the share of the tank.
pub const FUEL_KEY: &str = "vehicle.fuel";
/// `vehicle.start_lights`: `off`, `parking` or `dipped`.
pub const LIGHTS_KEY: &str = "vehicle.start_lights";

const START_LIGHTS: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-start-lights-off"),
    opt("parking", "set-opt-start-lights-parking"),
    opt("dipped", "set-opt-start-lights-dipped"),
];

/// The descriptors of the category.
#[must_use]
pub fn descriptors() -> Vec<D> {
    vec![
        D::new(
            SETUP_KEY,
            K::Float {
                min: 0.0,
                max: 1.0,
                step: 0.25,
            },
            "0.75",
            C::Vehicle,
            10,
            "set-vehicle-setup",
            "set-vehicle-setup-desc",
        )
        .keywords(&[
            "start",
            "engine",
            "cold",
            "ready",
            "ignition",
            "battery",
            "parking brake",
        ])
        .since("0.4"),
        D::new(
            FUEL_KEY,
            K::Float {
                min: 0.0,
                max: 1.0,
                step: 0.05,
            },
            "0.6",
            C::Vehicle,
            20,
            "set-vehicle-fuel",
            "set-vehicle-fuel-desc",
        )
        .keywords(&["tank", "diesel", "fuelage", "refuel"])
        .since("0.4"),
        D::new(
            LIGHTS_KEY,
            K::Enum(START_LIGHTS),
            "off",
            C::Vehicle,
            30,
            "set-vehicle-lights",
            "set-vehicle-lights-desc",
        )
        .keywords(&["headlights", "dipped", "parking lights"])
        .since("0.4"),
    ]
}

/// The stored values of the settings as a [`VehicleSetup`] (defaults for what is missing or
/// unreadable).
#[must_use]
pub fn setup_of(settings: &GameSettings) -> VehicleSetup {
    let default = VehicleSetup::default();
    let share = |key: &str, fallback: f32| {
        settings
            .raw(key)
            .and_then(|v| v.parse::<f32>().ok())
            .map_or(fallback, |v| v.clamp(0.0, 1.0))
    };
    VehicleSetup {
        degree: share(SETUP_KEY, default.degree),
        fuel: share(FUEL_KEY, default.fuel),
        lights: settings
            .raw(LIGHTS_KEY)
            .map_or(StartLights::Off, |v| StartLights::parse(&v)),
    }
}

/// Keeps [`VehicleSetup`] equal to the settings.
pub fn sync_vehicle_setup(settings: Res<GameSettings>, setup: Option<ResMut<VehicleSetup>>) {
    let Some(mut setup) = setup else { return };
    if !(settings.is_changed() || setup.is_added()) {
        return;
    }
    let want = setup_of(&settings);
    if *setup != want {
        *setup = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_resource_default() {
        assert_eq!(setup_of(&GameSettings::default()), VehicleSetup::default());
    }

    #[test]
    fn the_keys_and_defaults_are_in_the_registry() {
        let r = crate::settings_registry::SettingsRegistry::with_builtin();
        for key in [SETUP_KEY, FUEL_KEY, LIGHTS_KEY] {
            let d = r.get(key).unwrap_or_else(|| panic!("{key} registered"));
            assert_eq!(d.category, C::Vehicle);
        }
    }
}
