//! The electric state of charge (ISS-042, M-0123): the scripts of a battery vehicle keep the charge,
//! and after each script step the vehicle's fuel model follows it, so the HUD readout shows it.

use super::*;
use opts_sim::vehicle::fuel::{FuelKind, DEFAULT_CAPACITY};

/// Where a vehicle's scripts keep the state of charge: the variable and the full capacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElectricCharge {
    /// Index of the charge variable in the engine (`electric_battery`).
    pub var: u32,
    /// Full charge (the `electric_battery_capacity` constant, or the model's default when missing).
    pub capacity: f32,
}

impl ElectricCharge {
    /// The charge of an engine whose scripts store the charge variable; `None` for a diesel vehicle.
    pub fn resolve(engine: &mut ScriptEngine) -> Option<Self> {
        if !engine.writes(opts_sim::vehicle::fuel::ELECTRIC_CHARGE_VAR) {
            return None;
        }
        Some(Self {
            var: engine.variable_id(opts_sim::vehicle::fuel::ELECTRIC_CHARGE_VAR),
            capacity: engine
                .const_scalar(opts_sim::vehicle::fuel::ELECTRIC_CAPACITY_CONST)
                .unwrap_or(DEFAULT_CAPACITY),
        })
    }
}

/// Fixed-step system: after the scripts ran, each electric vehicle's fuel model takes the scripts'
/// state of charge. A diesel vehicle (no charge in its scripts) is left alone.
pub fn electric_charge_system(
    time: Res<Time>,
    mut query: Query<(&mut VehicleController, &mut VehicleScripts)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut controller, mut scripts) in &mut query {
        let Some(charge) = scripts.charge else {
            continue;
        };
        let Ok(engine) = scripts.engine.get_mut() else {
            continue;
        };
        let soc = engine.get_id(charge.var);
        let Some(fuel) = controller.sim.fuel.as_mut() else {
            continue;
        };
        let (was_low, wired) = (fuel.is_low(), fuel.kind == FuelKind::Electric);
        let draw = fuel.follow_charge(soc, charge.capacity, dt);
        if !wired {
            tracing::debug!(
                state_of_charge = fuel.level(),
                capacity = fuel.capacity,
                "electric state of charge wired to the fuel model"
            );
        }
        if fuel.is_low() != was_low {
            tracing::debug!(
                state_of_charge = fuel.level(),
                power_draw = draw,
                low = fuel.is_low(),
                "electric state of charge crossed the low level"
            );
        }
    }
}
