//! How far a vehicle is set up when a drive begins (M-0141): from cold and dead to ready to drive.
//!
//! The player chooses a **degree** from 0 to 1 (settings category *Vehicle*). The host brings the
//! vehicle to that degree by pressing the same keys a driver would, in the order the vehicle's own
//! scripts want (a *recipe*; the family is recognised by a variable its scripts own, see
//! [`RECIPES`]); nothing is written into the scripts' variables except the fuel content, once.
//!
//! | Degree | State |
//! | ---: | :--- |
//! | below 0.25 | cold: nothing is started |
//! | from 0.25 | electrics on (battery switch, ignition key inserted and turned) |
//! | from 0.5 | ignition in the drive position |
//! | from 0.75 | engine running (parking brake still set) |
//! | 1 | ready to drive: parking brake released, the gear selector follows the driver |
//!
//! Vehicles whose procedure has fewer steps reach a state earlier (the MAN family has no extra
//! ignition position: 0.5 is the same as 0.25).

use bevy::prelude::*;
use opts_sim::script::ScriptEngine;

mod recipes;
mod runner;
mod steps;
pub use recipes::{Detect, Recipe, RECIPES};
pub use runner::*;
pub use steps::{Step, DRAG_S};

/// Electrics on.
pub const ELECTRICS: f32 = 0.25;
/// Ignition in the drive position.
pub const IGNITION: f32 = 0.5;
/// Engine running.
pub const ENGINE: f32 = 0.75;
/// Ready to drive.
pub const READY: f32 = 1.0;

/// The lights a drive begins with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartLights {
    /// All off.
    #[default]
    Off,
    /// Parking lights.
    Parking,
    /// Dipped beam.
    Dipped,
}

impl StartLights {
    /// Parses the stored text (`off`, `parking`, `dipped`); anything else is `Off`.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        match text {
            "parking" => Self::Parking,
            "dipped" => Self::Dipped,
            _ => Self::Off,
        }
    }
}

/// The player's choices for the start of a drive (the settings of the *Vehicle* category).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct VehicleSetup {
    /// 0 (cold) to 1 (ready to drive), see the module's table.
    pub degree: f32,
    /// Fuel as a share of the tank, 0 to 1.
    pub fuel: f32,
    /// The lights switched on once the electrics are.
    pub lights: StartLights,
}

impl Default for VehicleSetup {
    fn default() -> Self {
        Self {
            degree: ENGINE,
            fuel: 0.6,
            lights: StartLights::Off,
        }
    }
}

/// The recipe of this vehicle.
#[must_use]
pub fn recipe_for(engine: &ScriptEngine) -> &'static Recipe {
    RECIPES
        .iter()
        .find(|r| r.detect.matches(engine))
        .unwrap_or(&RECIPES[RECIPES.len() - 1])
}

/// Whether the vehicle is ready to drive: its engine runs, or the variable its recipe names for
/// readiness is set (a serial hybrid, whose diesel stays off at standstill).
#[must_use]
pub fn ready(engine: &ScriptEngine) -> bool {
    match recipe_for(engine).ready {
        Some(var) => engine.get(var) > 0.5,
        None => engine.get("engine_on") > 0.5,
    }
}

#[cfg(test)]
mod tests;
