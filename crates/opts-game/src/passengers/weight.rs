//! The mass of the people on board (M-0124): the player's vehicle moves its curb mass plus the
//! passengers, `humans_inside × average mass`, unless the setting `gameplay.passenger_weight` is
//! off. The axle loads and the suspension keep the curb mass (their load distribution waits for
//! the per-axle input of M-0073).

use bevy::prelude::*;
use opts_bevy::{PlayerVehicle, VehicleController};

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

/// People each unit of a chain carries for its dynamics, head first. The real passengers count
/// by the unit they are in; a test count (`override_count`) is shared over the units in
/// proportion to their curb mass, so one unit takes all of it. Fractions are fine: only the mass
/// (count x average) is used.
pub(super) fn unit_counts(
    override_count: Option<usize>,
    real: &[usize],
    curb_kg: &[f32],
) -> Vec<f32> {
    let Some(total) = override_count else {
        return real.iter().map(|&c| c as f32).collect();
    };
    let sum: f32 = curb_kg.iter().sum();
    if curb_kg.len() <= 1 || sum <= 0.0 {
        return vec![total as f32];
    }
    curb_kg.iter().map(|m| total as f32 * m / sum).collect()
}

/// Writes the passengers' mass into the player's vehicle simulation, unit by unit (the head and
/// each following unit carry their own people).
pub fn apply_passenger_mass(
    settings: Option<Res<GameSettings>>,
    forced: Option<Res<PassengerCountOverride>>,
    mut vehicles: Query<(&PassengerRig, &mut VehicleController), With<PlayerVehicle>>,
) {
    let (on, average) = mass_settings(settings.as_deref());
    for (rig, mut controller) in &mut vehicles {
        let sim = &controller.sim;
        let units = 1 + sim.trailer_loads.len();
        let curb: Vec<f32> = std::iter::once(sim.head_mass_kg())
            .chain(sim.trailer_loads.iter().map(|u| u.mass_kg))
            .collect();
        let real = rig.sim.humans_inside_by_unit(units);
        let counts = unit_counts(forced.as_ref().and_then(|f| f.0), &real, &curb);
        let masses: Vec<f32> = counts
            .iter()
            .map(|&c| carried_mass_kg(on, average, c))
            .collect();
        let current: Vec<f32> = std::iter::once(sim.passenger_mass_kg)
            .chain((1..units).map(|k| {
                sim.unit_passenger_mass_kg
                    .get(k - 1)
                    .copied()
                    .unwrap_or(0.0)
            }))
            .collect();
        if masses.iter().zip(&current).any(|(a, b)| a != b) || masses.len() != current.len() {
            controller.sim.set_unit_passenger_masses(&masses);
        }
    }
}

/// The mass (kg) the dynamics carry for `count` people (a share may be fractional): none while
/// the setting is off.
fn carried_mass_kg(on: bool, average_kg: f32, count: f32) -> f32 {
    if on {
        count * average_kg.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
