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

mod runner;
pub use runner::*;

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
    /// For a cockpit control turned with the mouse: the vertical mouse movement in pixels, spread
    /// over the first [`DRAG_S`] of the hold (`<trigger>_drag` each frame, like a real drag);
    /// 0 for a key press.
    pub drag_y: f32,
    /// Taken only while the engine is still off (a second try of a key that steps).
    pub only_if_off: bool,
}

/// Seconds over which a drag step moves the mouse.
pub const DRAG_S: f32 = 0.5;

const fn tap(trigger: &'static str, from: f32) -> Step {
    Step {
        trigger,
        from,
        hold_s: 0.3,
        until_engine: false,
        needs_busbar: false,
        drag_y: 0.0,
        only_if_off: false,
    }
}

const fn drag(trigger: &'static str, from: f32, drag_y: f32) -> Step {
    Step {
        trigger,
        from,
        hold_s: DRAG_S + 0.2,
        until_engine: false,
        needs_busbar: false,
        drag_y,
        only_if_off: false,
    }
}

/// The start key once more, held, if the first press left the engine off: on many vehicles the
/// start key itself steps (the first press switches the ignition on, the second runs the
/// starter: the TH Überlandbus S315/O550).
const fn start_again(trigger: &'static str) -> Step {
    Step {
        only_if_off: true,
        ..start(trigger)
    }
}

const fn start_drag(trigger: &'static str, drag_y: f32) -> Step {
    Step {
        drag_y,
        ..start(trigger)
    }
}

/// A button pressed once at the engine degree (the electrics on), not held for an engine that
/// fires.
const fn press(trigger: &'static str) -> Step {
    Step {
        hold_s: 0.5,
        until_engine: false,
        ..start(trigger)
    }
}

const fn start(trigger: &'static str) -> Step {
    Step {
        trigger,
        from: ENGINE,
        hold_s: 8.0,
        until_engine: true,
        needs_busbar: true,
        drag_y: 0.0,
        only_if_off: false,
    }
}

const BATTERY: &str = "cp_batterietrennschalter_toggle";
const START: &str = "kw_m_enginestart";

/// How a family of vehicles is recognised.
#[derive(Debug, Clone, Copy)]
pub enum Detect {
    /// The scripts own this variable.
    Writes(&'static str),
    /// The scripts answer this trigger.
    Trigger(&'static str),
    /// The constant files set this constant to this value.
    Const(&'static str, f32),
    /// Every one of these holds.
    All(&'static [Detect]),
    /// Everything else.
    Otherwise,
}

impl Detect {
    /// Whether this vehicle's scripts match.
    #[must_use]
    pub fn matches(&self, engine: &ScriptEngine) -> bool {
        match *self {
            Self::Writes(var) => engine.writes(var),
            Self::Trigger(name) => engine.has_trigger(name),
            Self::Const(name, value) => engine.consts().scalar(name) == Some(value),
            Self::All(all) => all.iter().all(|d| d.matches(engine)),
            Self::Otherwise => true,
        }
    }
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
    /// The variable that says the vehicle is ready to drive when that is not a running engine
    /// (a serial hybrid's drive readiness); `None`: the engine runs.
    pub ready: Option<&'static str>,
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
        ready: None,
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
        ready: None,
    },
    // Luzern VBL Citybus (`cp_keymode`) and Valiant 7000 family (`cp_start_key_rot_drag`): the battery
    // key inserts the key (Valiant: presses the battery button), then turns it to the electrics;
    // the third press, held, is the starter, and the release springs back to the drive position.
    Recipe {
        name: "three-step key",
        detect: Detect::Writes("cp_keymode"),
        steps: &[
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, ELECTRICS),
            start(BATTERY),
        ],
        ready: None,
    },
    Recipe {
        name: "three-step key (battery button)",
        detect: Detect::Trigger("cp_start_key_rot_drag"),
        steps: &[
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, ELECTRICS),
            start(BATTERY),
        ],
        ready: None,
    },
    // Citybus S31x: the battery key inserts the key, turns it to the electrics, to the ignition
    // and, held, to the starter. Recognised by the three together (each alone is shared: the
    // Actia dashboard and its drag trigger by the Heuliez/Iveco families, `key_mode` by the
    // Citelis and MAN Lion's City, which start the generic way).
    Recipe {
        name: "four-step key",
        detect: Detect::All(&[
            Detect::Writes("key_mode"),
            Detect::Writes("actia_dash_on"),
            Detect::Trigger("actia_move_drag"),
        ]),
        steps: &[
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, IGNITION),
            start(BATTERY),
        ],
        ready: None,
    },
    // Studio Polygon Lite (`engineStartSwitch`): the battery key steps `StartMode`: master
    // switch, ignition, starter (held).
    Recipe {
        name: "three-step key (master switch)",
        detect: Detect::Trigger("enginestartswitch"),
        steps: &[
            tap(BATTERY, ELECTRICS),
            tap(BATTERY, IGNITION),
            start(BATTERY),
            start_again(BATTERY),
        ],
        ready: None,
    },
    // MB C2 family (constant `engine_start_legacy` 0): two scripts answer the battery key (one
    // steps the key, one toggles the electrics), so the key is turned with the mouse like a
    // driver does: to the electrics (`cp_key_rot` 0.2 to 0.8; it springs back to a third on
    // release), then past 0.9, which starts the engine.
    Recipe {
        name: "ignition key turned by mouse",
        detect: Detect::Const("engine_start_legacy", 0.0),
        steps: &[
            drag("cp_key_rot", ELECTRICS, -15.0),
            start_drag("cp_key_rot", -25.0),
        ],
        ready: None,
    },
    // MB O530 G BlueTEC Hybrid (`Hybrid_Serial.osc`): the start key toggles drive readiness
    // (`startup_mode`). The diesel generator starts by itself while driving or when the traction
    // battery runs low, so at standstill it stays off, and a second press would switch the bus
    // off again.
    Recipe {
        name: "serial hybrid start button",
        detect: Detect::All(&[
            Detect::Writes("startup_mode"),
            Detect::Writes("generator_trigger"),
        ]),
        steps: &[tap(BATTERY, ELECTRICS), press(START)],
        ready: Some("startup_mode"),
    },
    // MAN NL, SD200, NG272 and unknown vehicles: battery switch, start key held (and once
    // more if the engine is still off).
    Recipe {
        name: "battery switch and start key",
        detect: Detect::Otherwise,
        steps: &[tap(BATTERY, ELECTRICS), start(START), start_again(START)],
        ready: None,
    },
];

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
