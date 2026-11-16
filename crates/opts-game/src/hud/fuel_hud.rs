//! The fuel readout of the player's vehicle (M-0123): the level and the low-level flag. There is
//! no readout when the vehicle has no fuel model or the consumption setting is `Off`.

use opts_bevy::VehicleController;
use opts_sim::vehicle::fuel::{FuelMode, FuelState};

/// The level of a fuel model, as the widgets show it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuelHud {
    /// The level in percent (0 to 100).
    pub percent: f32,
    /// At or below the low-level threshold.
    pub low: bool,
}

/// The readout of a fuel model; `None` without a model or in mode `Off`.
#[must_use]
pub fn readout(fuel: Option<&FuelState>) -> Option<FuelHud> {
    let fuel = fuel.filter(|f| f.mode != FuelMode::Off)?;
    Some(FuelHud {
        percent: fuel.level() * 100.0,
        low: fuel.is_low(),
    })
}

/// The readout of a vehicle's fuel model.
#[must_use]
pub fn fuel_hud(controller: &VehicleController) -> Option<FuelHud> {
    readout(controller.sim.fuel.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_sim::vehicle::fuel::FuelKind;

    #[test]
    fn no_model_or_mode_off_gives_no_readout() {
        assert_eq!(readout(None), None);
        let mut off = FuelState::full(FuelKind::Diesel, 250.0);
        off.mode = FuelMode::Off;
        assert_eq!(readout(Some(&off)), None);
    }

    #[test]
    fn the_level_is_a_percentage_and_the_low_flag_follows_the_threshold() {
        let mut fuel = FuelState::full(FuelKind::Diesel, 200.0);
        fuel.set_level(0.5);
        let half = readout(Some(&fuel)).expect("shown");
        assert!((half.percent - 50.0).abs() < 1e-4);
        assert!(!half.low);
        fuel.set_level(0.1);
        assert!(readout(Some(&fuel)).expect("shown").low);
    }
}
