//! The electric charge of a battery vehicle whose scripts own it (ISS-042, M-0123).
//!
//! The electric bus scripts keep the state of charge in the float variable
//! [`ELECTRIC_CHARGE_VAR`] (in the units of the scalar constant [`ELECTRIC_CAPACITY_CONST`]) and
//! drain it themselves. The host mirrors that value into the fuel model, so the readout and the
//! low-level flag show the real charge, and the model does not drain it a second time.

use super::{FuelKind, FuelState};

/// Float variable of the scripts that carries an electric vehicle's state of charge.
pub const ELECTRIC_CHARGE_VAR: &str = "electric_battery";
/// Scalar constant of the scripts that gives the full charge of the battery.
pub const ELECTRIC_CAPACITY_CONST: &str = "electric_battery_capacity";

impl FuelState {
    /// Makes this model a battery the scripts own: the kind becomes electric and the capacity and
    /// content take the scripts' values (`charge` is clamped to the capacity). Returns the power
    /// draw in capacity units per second since the last call (0 on the first call, when the
    /// charge rises, or for a `dt` of 0 or less).
    pub fn follow_charge(&mut self, charge: f32, capacity: f32, dt: f32) -> f32 {
        let draw = if self.script_owned && dt > 0.0 {
            ((self.content - charge) / dt).max(0.0)
        } else {
            0.0
        };
        self.kind = FuelKind::Electric;
        self.script_owned = true;
        self.capacity = capacity.max(1.0);
        self.content = charge.clamp(0.0, self.capacity);
        draw
    }
}
