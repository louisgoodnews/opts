//! The start procedures of the vehicle families ([`RECIPES`]) and how each family is recognised
//! ([`Detect`]).

use opts_sim::script::ScriptEngine;

use super::steps::{drag, press, start, start_again, start_drag, tap, BATTERY, START};
use super::Step;
use super::{ELECTRICS, IGNITION};

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
