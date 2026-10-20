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

/// One key press of a start procedure.
#[derive(Debug, Clone, Copy)]
pub struct Step {
    /// The script trigger (`<trigger>_off` is raised on release).
    pub trigger: &'static str,
    /// The first degree at which the step is taken.
    pub from: f32,
    /// Seconds the key is held; for [`Self::until_engine`] the longest it is held.
    pub hold_s: f32,
    /// Hold until `engine_on` (the starter).
    pub until_engine: bool,
    /// Wait for the busbar voltage first.
    pub needs_busbar: bool,
}

const fn tap(trigger: &'static str, from: f32) -> Step {
    Step {
        trigger,
        from,
        hold_s: 0.3,
        until_engine: false,
        needs_busbar: false,
    }
}

const fn start(trigger: &'static str) -> Step {
    Step {
        trigger,
        from: ENGINE,
        hold_s: 8.0,
        until_engine: true,
        needs_busbar: true,
    }
}

const BATTERY: &str = "cp_batterietrennschalter_toggle";
const START: &str = "kw_m_enginestart";

/// How a family of vehicles is recognised.
#[derive(Debug, Clone, Copy)]
pub enum Detect {
    /// The scripts own this variable.
    Writes(&'static str),
    /// Everything else.
    Otherwise,
}

/// A start procedure.
#[derive(Debug)]
pub struct Recipe {
    /// Name for logs and the documentation.
    pub name: &'static str,
    /// How the family is recognised.
    pub detect: Detect,
    /// The key presses in order.
    pub steps: &'static [Step],
}

/// The procedures (`docs/src/quality/vehicle-start-procedures.md`), most specific first.
pub static RECIPES: &[Recipe] = &[
    // Citybus 530: the battery key steps the ignition key: electrics, drive, starter (held).
    Recipe {
        name: "stepped ignition key",
        detect: Detect::Writes("cp_keyboard_e_m"),
        steps: &[
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, IGNITION),
            start(BATTERY),
        ],
    },
    // BRT C2: the battery key inserts the key, turns it to the electrics and to drive; the start
    // key then starts the engine.
    Recipe {
        name: "inserted ignition key",
        detect: Detect::Writes("c2_cp_zuendschluessel_rot_pos"),
        steps: &[
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, IGNITION),
            start(START),
        ],
    },
    // MAN NL, SD200, NG272 and unknown vehicles: battery switch, start key held.
    Recipe {
        name: "battery switch and start key",
        detect: Detect::Otherwise,
        steps: &[tap(BATTERY, ELECTRICS), start(START)],
    },
];

/// The recipe of this vehicle.
#[must_use]
pub fn recipe_for(engine: &ScriptEngine) -> &'static Recipe {
    RECIPES
        .iter()
        .find(|r| match r.detect {
            Detect::Writes(var) => engine.writes(var),
            Detect::Otherwise => true,
        })
        .unwrap_or(&RECIPES[RECIPES.len() - 1])
}

/// Seconds between one step and the next.
const SETTLE_S: f32 = 1.0;
/// The busbar voltage from which the electrics count as on (the scripts' own limit).
const BUSBAR_MIN: f32 = 0.3;
/// The tank when the vehicle's constants name none (litres): a city bus.
const DEFAULT_TANK_L: f32 = 250.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Held,
}

/// The progress of one vehicle through its recipe.
#[derive(Debug, Clone, Default)]
pub struct SetupRun {
    step: usize,
    phase: Phase,
    since: f32,
    fuel_done: bool,
    lights_done: bool,
}

impl SetupRun {
    /// Takes the next step when it is due. `now` is the vehicle's seconds since it appeared.
    /// Returns `true` once there is nothing more to do at this degree.
    pub fn advance(&mut self, engine: &mut ScriptEngine, setup: &VehicleSetup, now: f32) -> bool {
        if !self.fuel_done && now > 0.2 {
            self.fuel_done = true;
            fill_tank(engine, setup.fuel);
        }
        let recipe = recipe_for(engine);
        let busbar = engine.get("elec_busbar_main");
        if self.phase == Phase::Held {
            let step = recipe.steps[self.step];
            let done = if step.until_engine {
                engine.get("engine_on") > 0.5 || now - self.since >= step.hold_s
            } else {
                now - self.since >= step.hold_s
            };
            if done {
                engine.trigger(&format!("{}_off", step.trigger));
                self.phase = Phase::Idle;
                self.step += 1;
                self.since = now;
            }
            return false;
        }
        // A vehicle whose scripts have no engine (a tram) has nothing to start: its start step
        // is skipped, and what waits for a busbar its scripts never raise does not wait.
        if let Some(step) = recipe.steps.get(self.step) {
            if step.until_engine && !engine.writes("engine_on") {
                self.step += 1;
                return false;
            }
        }
        let busbar = if engine.writes("elec_busbar_main") {
            busbar
        } else {
            1.0
        };
        if let Some(step) = recipe.steps.get(self.step) {
            if step.from <= setup.degree + 1e-4 {
                let ready =
                    now - self.since >= SETTLE_S && (!step.needs_busbar || busbar > BUSBAR_MIN);
                if ready && now > 0.5 {
                    engine.trigger(step.trigger);
                    self.phase = Phase::Held;
                    self.since = now;
                }
                return false;
            }
        }
        // Nothing left at this degree: the lights, once the electrics are on.
        if !self.lights_done && setup.degree >= ELECTRICS && busbar > BUSBAR_MIN {
            self.lights_done = true;
            match setup.lights {
                StartLights::Off => {}
                StartLights::Parking => {
                    tap_key(engine, "kw_standlicht_toggle");
                }
                StartLights::Dipped => {
                    tap_key(engine, "kw_scheinwerfer_toggle");
                }
            }
        }
        true
    }
}

/// Whether the vehicle's scripts own an engine (`engine_on`): a tram's do not.
#[must_use]
pub fn has_engine(engine: &ScriptEngine) -> bool {
    engine.writes("engine_on")
}

/// Raises a key's trigger and its `_off` (the lights are not held keys).
fn tap_key(engine: &mut ScriptEngine, trigger: &str) {
    engine.trigger(trigger);
    engine.trigger(&format!("{trigger}_off"));
}

/// Sets the tank to `share` of its capacity (the constant `engine_tank_capacity`, or
/// [`DEFAULT_TANK_L`]). The scripts' own start value is a random one; this is the player's.
fn fill_tank(engine: &mut ScriptEngine, share: f32) {
    let capacity = engine
        .consts_mut()
        .scalar("engine_tank_capacity")
        .filter(|c| *c > 1.0)
        .unwrap_or(DEFAULT_TANK_L);
    if engine.writes("engine_tank_content") {
        engine.set("engine_tank_content", capacity * share.clamp(0.0, 1.0));
    }
}

#[cfg(test)]
mod tests;
