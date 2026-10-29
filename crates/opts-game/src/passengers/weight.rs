//! The mass of the people on board (M-0124): the player's vehicle moves its curb mass plus the
//! passengers, `humans_inside × average mass`, unless the setting `gameplay.passenger_weight` is
//! off. The axle loads and the suspension keep the curb mass (their load distribution waits for
//! the per-axle input of M-0073).

use bevy::prelude::*;
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_sim::vehicle::dynamic_mass_kg;

use super::PassengerRig;
use crate::settings::GameSettings;

/// Key of the setting that turns the effect on and off.
pub const SETTING: &str = "gameplay.passenger_weight";
/// Key of the average mass of one passenger (kg).
pub const MASS_SETTING: &str = "gameplay.passenger_mass_kg";
/// Average passenger mass when the setting is missing (kg).
pub const DEFAULT_MASS_KG: f32 = 75.0;

/// A passenger count that stands in for the people inside while testing (`vehicle.passengers`
/// with `mass_count`); `None` uses the real passengers.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PassengerCountOverride(pub Option<usize>);

/// Starts a drive without a test count: the override belongs to the session that set it, and a
/// saved one comes back with the `passenger_load` save section.
pub fn clear_override(mut forced: ResMut<PassengerCountOverride>) {
    forced.0 = None;
}

/// What the settings say about the passengers' mass: whether it counts at all (on unless the
/// setting is `false`) and one person's mass in kg (the default when it is missing or unreadable).
pub(super) fn mass_settings(settings: Option<&GameSettings>) -> (bool, f32) {
    settings.map_or((true, DEFAULT_MASS_KG), |s| {
        (
            s.raw(SETTING)
                .is_none_or(|v| !v.trim().eq_ignore_ascii_case("false")),
            s.raw(MASS_SETTING)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .unwrap_or(DEFAULT_MASS_KG),
        )
    })
}

/// The mass (kg) the dynamics carry for `count` people: none while the setting is off.
fn carried_mass_kg(on: bool, average_kg: f32, count: usize) -> f32 {
    if on {
        dynamic_mass_kg(0.0, count, average_kg)
    } else {
        0.0
    }
}

/// Writes the passengers' mass into the player's vehicle simulation.
pub fn apply_passenger_mass(
    settings: Option<Res<GameSettings>>,
    forced: Option<Res<PassengerCountOverride>>,
    mut vehicles: Query<(&PassengerRig, &mut VehicleController), With<PlayerVehicle>>,
) {
    let (on, average) = mass_settings(settings.as_deref());
    for (rig, mut controller) in &mut vehicles {
        let count = forced
            .as_ref()
            .and_then(|f| f.0)
            .unwrap_or_else(|| rig.sim.humans_inside());
        let kg = carried_mass_kg(on, average, count);
        if controller.sim.passenger_mass_kg != kg {
            controller.sim.passenger_mass_kg = kg;
        }
    }
}

#[cfg(test)]
mod tests;
