//! The player vehicle's equipment options on the world load job (M-0078): the repaints the
//! vehicle was discovered with, the chosen repaint and the user's changes, for the catalogue the
//! control protocol (`vehicle.setvars`) shows and changes.

use crate::new_game_screen::repaints::VehicleRepaints;

use super::WorldLoadJob;

impl WorldLoadJob {
    /// The repaints, declared variables and family folder of the player's vehicle, once loaded.
    #[must_use]
    pub fn vehicle_repaints(&self) -> Option<&VehicleRepaints> {
        self.repaint.as_ref()
    }

    /// The repaint the vehicle was built with (empty: the vehicle's default).
    #[must_use]
    pub fn vehicle_livery(&self) -> &str {
        &self.config.livery
    }

    /// The user's equipment changes of the session, in the order they were made.
    #[must_use]
    pub fn vehicle_setvars(&self) -> &[(String, i32)] {
        &self.config.setvars
    }

    /// Records a change of an equipment option, so a respawn of the vehicle keeps it.
    pub fn set_vehicle_setvar(&mut self, name: &str, value: i32) {
        match self
            .config
            .setvars
            .iter_mut()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            Some(entry) => entry.1 = value,
            None => self.config.setvars.push((name.to_string(), value)),
        }
    }
}
