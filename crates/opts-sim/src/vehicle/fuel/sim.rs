//! The fuel model of a [`VehicleSimulation`]: the per-step depletion, the refuel action and the
//! level read by the host (M-0123).

use super::{FuelKind, FuelMode, FuelState};

impl crate::vehicle::VehicleSimulation {
    /// Gives the vehicle a fuel model of `kind` and `capacity` when it has none, and sets its
    /// consumption `mode`. A model that exists keeps its content (saves restore it).
    pub fn ensure_fuel(&mut self, kind: FuelKind, capacity: f32, mode: FuelMode) {
        self.fuel
            .get_or_insert_with(|| FuelState::full(kind, capacity))
            .mode = mode;
    }

    /// The refuel action: fills the store. Returns `false` for a vehicle without a model.
    pub fn refuel(&mut self) -> bool {
        match self.fuel.as_mut() {
            Some(fuel) => {
                fuel.refill();
                true
            }
            None => false,
        }
    }

    /// The level of the store (0 to 1), `None` for a vehicle without a model.
    #[must_use]
    pub fn fuel_level(&self) -> Option<f32> {
        self.fuel.as_ref().map(FuelState::level)
    }

    /// Depletes the fuel store for one step and returns the torque factor to apply to this
    /// step's drive (1 without a fuel model).
    pub(in crate::vehicle) fn fuel_step(&mut self, dt: f32) -> f32 {
        let throttle = self.inputs.throttle();
        match self.fuel.as_mut() {
            Some(fuel) => {
                fuel.step(throttle, dt);
                fuel.torque_factor()
            }
            None => 1.0,
        }
    }
}
