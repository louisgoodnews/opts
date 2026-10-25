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

/// The engine speed from which an engine that fired runs on its own: the starter is held until
/// then (released at the first firing, the TH O550 stalls at about 350 rpm).
const SELF_SUSTAINING_RPM: f32 = 400.0;
/// The longest the starter is held after the engine fired, should it never reach that speed.
const AFTER_FIRING_S: f32 = 1.5;

/// Script interlocks a second starter press waits for, where the vehicle's scripts own them: the
/// ABS self-test that the ignition starts (about 6 s on the Studio Polygon Lite) and a
/// wait-to-start lamp. The first press does not wait, so a vehicle that starts at once is never
/// delayed.
const START_INTERLOCKS: [&str; 2] = ["bremse_abs_selftest", "waittostartactive"];
/// The longest a starter step waits for those interlocks before it is pressed anyway.
const INTERLOCK_WAIT_S: f32 = 10.0;

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
    /// `now` of the previous call (the frame time of a drag).
    last: f32,
    /// When the engine first fired during the current starter step.
    fired_at: Option<f32>,
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
        let dt = (now - self.last).max(0.0);
        self.last = now;
        if self.phase == Phase::Held {
            let step = recipe.steps[self.step];
            if step.drag_y != 0.0 {
                let moving = now - self.since <= DRAG_S;
                engine.set_system("mouse_x", 0.0);
                engine.set_system(
                    "mouse_y",
                    if moving {
                        step.drag_y * dt / DRAG_S
                    } else {
                        0.0
                    },
                );
                if moving {
                    engine.trigger(&format!("{}_drag", step.trigger));
                }
            }
            let done = if step.until_engine {
                if engine.get("engine_on") > 0.5 && self.fired_at.is_none() {
                    self.fired_at = Some(now);
                }
                let running = self.fired_at.is_some_and(|t| {
                    engine.get("engine_n") >= SELF_SUSTAINING_RPM || now - t >= AFTER_FIRING_S
                });
                running || now - self.since >= step.hold_s
            } else {
                now - self.since >= step.hold_s
            };
            if done {
                if step.drag_y != 0.0 {
                    engine.set_system("mouse_y", 0.0);
                }
                engine.trigger(&format!("{}_off", step.trigger));
                self.fired_at = None;
                self.phase = Phase::Idle;
                self.step += 1;
                self.since = now;
            }
            return false;
        }
        // A vehicle whose scripts have no engine (a tram) has nothing to start: its start step
        // is skipped, and what waits for a busbar its scripts never raise does not wait.
        if let Some(step) = recipe.steps.get(self.step) {
            if step.only_if_off && engine.get("engine_on") > 0.5 {
                self.step += 1;
                return false;
            }
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
                let interlocks_clear = !step.only_if_off
                    || now - self.since >= SETTLE_S + INTERLOCK_WAIT_S
                    || START_INTERLOCKS
                        .iter()
                        .all(|v| !engine.writes(v) || engine.get(v) < 0.5);
                let ready = now - self.since >= SETTLE_S
                    && (!step.needs_busbar || busbar > BUSBAR_MIN)
                    && interlocks_clear;
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
