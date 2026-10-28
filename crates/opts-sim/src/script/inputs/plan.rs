//! Feed planning, per-frame input injection, and script output table definitions.

use super::*;

#[derive(Debug, Clone, Copy)]
enum Slot {
    Scalar(usize),
    Wheel(WheelFamily, usize, bool),
    Door(usize, usize),
}

#[derive(Debug, Clone)]
struct Planned {
    name: String,
    slot: Slot,
    kind: Kind,
    id: u32,
    /// The set stores to the name: the host leaves it alone.
    script_owned: bool,
    live: bool,
}

/// The feed of one engine, resolved once: variable ids and the names the scripts own.
#[derive(Debug, Clone)]
pub struct FeedPlan {
    rows: Vec<Planned>,
}

impl FeedPlan {
    /// Resolves every table name against `engine` (names are interned if missing, so the
    /// scripts' later reads find the fed value).
    pub fn new(engine: &mut ScriptEngine) -> Self {
        let mut rows = Vec::new();
        let mut push = |engine: &mut ScriptEngine, name: String, slot, kind, shared, live| {
            let (id, script_owned) = match kind {
                // The pedals are fed every frame, also when a script writes them: a script that
                // writes `Throttle` for a corner case (the HH109 1992's idle assist for one bus
                // number) used to take the pedal away from the player for good.
                Kind::Float => (
                    engine.variable_id(&name),
                    shared && engine.writes(&name) && name != "throttle" && name != "brake",
                ),
                Kind::System => (engine.system_id(&name), false),
            };
            rows.push(Planned {
                name,
                slot,
                kind,
                id,
                script_owned,
                live,
            });
        };
        for (i, r) in INPUT_TABLE.iter().enumerate() {
            push(
                engine,
                r.name.to_owned(),
                Slot::Scalar(i),
                r.kind,
                r.shared,
                r.live,
            );
        }
        for (family, stem, _) in WHEEL_FAMILIES {
            let shared = *family == WheelFamily::Rotation;
            for axle in 0..AXLES {
                for (side, right) in [("l", false), ("r", true)] {
                    push(
                        engine,
                        format!("{stem}_{axle}_{side}"),
                        Slot::Wheel(*family, axle, right),
                        Kind::Float,
                        shared,
                        *family != WheelFamily::Suspension,
                    );
                }
            }
        }
        for (f, stem) in DOOR_FAMILIES.iter().enumerate() {
            for door in 0..DOORS {
                push(
                    engine,
                    format!("{stem}{door}_req"),
                    Slot::Door(f, door),
                    Kind::Float,
                    true,
                    false,
                );
            }
        }
        Self { rows }
    }

    /// Names the host feeds (not owned by the scripts).
    #[must_use]
    pub fn fed_names(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| !r.script_owned)
            .map(|r| r.name.as_str())
            .collect()
    }

    /// Names of the table the scripts store to themselves (the host leaves them alone).
    #[must_use]
    pub fn script_owned(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| r.script_owned)
            .map(|r| r.name.as_str())
            .collect()
    }

    /// Fed names that carry the accepted zero because no system produces them yet.
    #[must_use]
    pub fn zero_fed(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| !r.script_owned && !r.live)
            .map(|r| r.name.as_str())
            .collect()
    }

    /// Whether `name` (any case) is a table name, and whether this plan feeds it.
    #[must_use]
    pub fn status(&self, name: &str) -> Option<bool> {
        let lower = name.to_ascii_lowercase();
        self.rows
            .iter()
            .find(|r| r.name == lower)
            .map(|r| !r.script_owned)
    }
}

/// Writes every input of the plan into the engine. Names the scripts own are skipped.
pub fn feed(engine: &mut ScriptEngine, plan: &FeedPlan, inputs: &HostInputs) {
    for r in &plan.rows {
        if r.script_owned {
            continue;
        }
        let value = match r.slot {
            Slot::Scalar(i) => inputs.value(&INPUT_TABLE[i]),
            Slot::Wheel(family, axle, right) => inputs.wheel(family, axle, right),
            Slot::Door(0, d) => f32::from(u8::from(inputs.pax_entry_req[d])),
            Slot::Door(_, d) => f32::from(u8::from(inputs.pax_exit_req[d])),
        };
        match r.kind {
            Kind::Float => engine.set_id(r.id, value),
            Kind::System => engine.set_system_id(r.id, value),
        }
    }
}

/// Who consumes an output of the scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consumer {
    /// Drive torque and brake forces (under script drive).
    Physics,
    /// Lamps and spotlights.
    Lights,
    /// Displays, route and timetable state.
    Display,
    /// The audio mixer.
    Audio,
    /// The passenger system.
    Passengers,
    /// Force feedback device.
    ForceFeedback,
    /// Developer diagnostics.
    Debug,
    /// Suspension, climate and other values nothing reads yet.
    None,
}

/// One output of the scripts the engine is expected to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputRow {
    /// Lower-case name (a `*` stands for the wheel or door index pattern).
    pub name: &'static str,
    /// Consumer.
    pub consumer: Consumer,
    /// OPTS reads it today.
    pub consumed: bool,
}

const fn out(name: &'static str, consumer: Consumer, consumed: bool) -> OutputRow {
    OutputRow {
        name,
        consumer,
        consumed,
    }
}

/// Outputs of the scripts. Family rows end in `_` and cover every index.
pub const OUTPUT_TABLE: &[OutputRow] = &[
    out("m_wheel", Consumer::Physics, true),
    out("brakeforce", Consumer::Physics, true),
    out("axle_brakeforce_", Consumer::Physics, true),
    out("axle_springfactor_", Consumer::Physics, false),
    out("throttle", Consumer::Physics, false),
    out("brake", Consumer::Physics, false),
    out("spot_select", Consumer::Lights, true),
    out("ai_light", Consumer::Lights, true),
    out("ai_blinker_l", Consumer::Lights, true),
    out("ai_blinker_r", Consumer::Lights, true),
    out("ai_interiorlight", Consumer::Lights, true),
    out("ai_brakelight", Consumer::Lights, false),
    out("ai_scheduled_atstation", Consumer::Passengers, false),
    out("target_index_int", Consumer::Display, false),
    out("refresh_strings", Consumer::Display, false),
    out("pax_entry", Consumer::Passengers, false),
    out("pax_exit", Consumer::Passengers, false),
    out("giventicket", Consumer::Passengers, false),
    out("snd_outsidevol", Consumer::Audio, false),
    out("snd_microphone", Consumer::Audio, false),
    out("snd_radio", Consumer::Audio, false),
    out("ff_vib_amp", Consumer::ForceFeedback, false),
    out("ff_vib_period", Consumer::ForceFeedback, false),
    out("cabinair_temp", Consumer::None, false),
    out("cabinair_abshum", Consumer::None, false),
    out("cabinair_relhum", Consumer::None, false),
    out("dirt_norm", Consumer::None, false),
    out("tank_percent", Consumer::None, false),
    out("debug_", Consumer::Debug, false),
];

/// The output row `name` (any case) belongs to, if any: exact names, `*_` wheel families and the
/// `pax_entry<n>_open` / `pax_exit<n>_open` door families.
#[must_use]
pub fn output_row(name: &str) -> Option<&'static OutputRow> {
    let lower = name.to_ascii_lowercase();
    OUTPUT_TABLE.iter().find(|r| {
        if r.name.ends_with('_') {
            lower.starts_with(r.name)
        } else if r.name == "pax_entry" || r.name == "pax_exit" {
            lower.starts_with(r.name) && lower.ends_with("_open")
        } else {
            r.name == lower
        }
    })
}
