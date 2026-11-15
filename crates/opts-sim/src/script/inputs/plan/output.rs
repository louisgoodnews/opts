//! The output table: who consumes each output the scripts write, and lookup by name.


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
