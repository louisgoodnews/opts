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

/// Writes the passengers' mass into the player's vehicle simulation.
pub fn apply_passenger_mass(
    settings: Option<Res<GameSettings>>,
    forced: Option<Res<PassengerCountOverride>>,
    mut vehicles: Query<(&PassengerRig, &mut VehicleController), With<PlayerVehicle>>,
) {
    let (on, average) = settings.as_ref().map_or((true, DEFAULT_MASS_KG), |s| {
        (
            s.raw(SETTING)
                .is_none_or(|v| !v.trim().eq_ignore_ascii_case("false")),
            s.raw(MASS_SETTING)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .unwrap_or(DEFAULT_MASS_KG),
        )
    });
    for (rig, mut controller) in &mut vehicles {
        let kg = if on {
            let count = forced
                .as_ref()
                .and_then(|f| f.0)
                .unwrap_or_else(|| rig.sim.humans_inside());
            dynamic_mass_kg(0.0, count, average)
        } else {
            0.0
        };
        if controller.sim.passenger_mass_kg != kg {
            controller.sim.passenger_mass_kg = kg;
        }
    }
}
