//! The declared table of the variables the engine supplies to vehicle scripts, the feed generated
//! from it, and the table of the outputs the engine consumes
//! (`docs/src/architecture/osc-runtime.md`, "Variable interface"; names from
//! `docs/src/formats/text/vehicles/predefined-variables.md` and OMSI's `program/varlist_*.txt`).
//!
//! A host fills a [`HostInputs`] value from its own state and calls [`feed`]; the names, units
//! and sources live here so that the feed, the integration report and the documentation agree.
//! A name the scripts themselves store to (`Throttle` of an AI script, `Wheel_Rotation_*` of a
//! trailer script) is left to the scripts: the host writes only names the set does not write.

use super::ScriptEngine;

/// Where the engine takes an input from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The vehicle's physics (speeds, accelerations, wheels, odometer).
    Physics,
    /// The driver or the traffic planner (pedals, AI flags, target).
    Planner,
    /// The simulation clock (time of day, date, sun).
    Clock,
    /// Weather (temperature, humidity, precipitation).
    Weather,
    /// Environment of the vehicle (brightness, road condition, dirt, wear).
    Environment,
    /// The passenger system (counts, door requests).
    Passengers,
    /// Timetable state (schedule active).
    Timetable,
    /// Collision events.
    Collision,
    /// The mouse while a hot-spot is held (fed by the trigger system, not by [`feed`]).
    Mouse,
    /// Rail coupling state.
    Rail,
    /// Traffic rules (priority).
    Traffic,
    /// A fixed value no system produces yet.
    Constant,
}

/// Whether the name is a vehicle float variable (`(L.L.x)`) or a system value (`(L.S.x)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `(L.L.name)`.
    Float,
    /// `(L.S.name)`.
    System,
}

/// One input of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputRow {
    /// Lower-case name.
    pub name: &'static str,
    /// Unit or value range.
    pub unit: &'static str,
    /// Producer.
    pub source: Source,
    /// Variable kind.
    pub kind: Kind,
    /// Scripts may store to this name too (AI scripts write the pedals, trailers write wheel
    /// rotations): the host writes it only when the set does not.
    pub shared: bool,
    /// A real value reaches it today; otherwise the feed writes the accepted zero and the
    /// integration report lists it as *zero-fed*.
    pub live: bool,
}

mod table;
pub use table::INPUT_TABLE;

/// Per-wheel input families: `<family>_<axle>_<l|r>` for four axles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelFamily {
    /// `Wheel_RotationSpeed_n_s` (rpm).
    RotationSpeed,
    /// `Wheel_Rotation_n_s` (rad, growing while driving forward); trailer scripts write it.
    Rotation,
    /// `Axle_Steering_n_s` (rad).
    Steering,
    /// `Axle_Suspension_n_s` (m).
    Suspension,
}

/// The wheel families with their name stem.
pub const WHEEL_FAMILIES: &[(WheelFamily, &str, &str)] = &[
    (WheelFamily::RotationSpeed, "wheel_rotationspeed", "rpm"),
    (WheelFamily::Rotation, "wheel_rotation", "rad"),
    (WheelFamily::Steering, "axle_steering", "rad"),
    (WheelFamily::Suspension, "axle_suspension", "m"),
];

/// Axles the predefined names cover.
pub const AXLES: usize = 4;

/// Door request families: `PAX_Entry<n>_Req`, `PAX_Exit<n>_Req` for eight doors; scripts
/// clear them, so they are shared.
pub const DOOR_FAMILIES: &[&str] = &["pax_entry", "pax_exit"];

/// Doors the predefined names cover.
pub const DOORS: usize = 8;

/// One wheel of the host's vehicle.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelInput {
    /// Rotational speed (rpm).
    pub speed_rpm: f32,
    /// Accumulated rotation (rad).
    pub rotation_rad: f32,
    /// Steering angle (rad, positive turning right).
    pub steering_rad: f32,
    /// Suspension travel (m): the extension from the unloaded spring, negative at rest.
    pub suspension_m: f32,
}

/// One axle: left and right wheel.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AxleInput {
    /// Left wheel.
    pub left: WheelInput,
    /// Right wheel.
    pub right: WheelInput,
}

/// The lifespan factor of a vehicle that never breaks down (OPTS has no breakdown option yet).
pub const WEARLIFESPAN_NO_BREAKDOWNS: f32 = 1000.0;

/// Everything the host knows that the scripts may read, in the units of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct HostInputs {
    /// Vehicle speed (km/h).
    pub velocity_km_h: f32,
    /// Ground speed (km/h).
    pub velocity_ground_km_h: f32,
    /// Mean wheel speed (rpm).
    pub n_wheel_rpm: f32,
    /// Accelerations in vehicle axes (m/s²).
    pub a_trans: [f32; 3],
    /// Odometer (m).
    pub odometer_m: f64,
    /// Accelerator 0..1.
    pub throttle: f32,
    /// Service brake 0..1.
    pub brake: f32,
    /// Clutch 0..1.
    pub clutch: f32,
    /// The vehicle is AI-driven.
    pub ai: bool,
    /// AI engine request (−1 stop, 0 none, 1 start).
    pub ai_engine: f32,
    /// Hof terminus row the dispatcher set (−1 none).
    pub ai_target_index: f32,
    /// A course is set.
    pub schedule_active: bool,
    /// Passengers on board.
    pub humans_count: u32,
    /// Door requests by passengers, per door.
    pub pax_entry_req: [bool; DOORS],
    /// Exit requests, per door.
    pub pax_exit_req: [bool; DOORS],
    /// Ambient brightness 0..1.
    pub envir_brightness: f32,
    /// Road condition 0..1 (0 dry).
    pub street_cond: f32,
    /// Dirt accumulation rate.
    pub dirt_rate: f32,
    /// Lifespan factor of the vehicle: the scripts scale their failure limits with it
    /// (`random(Fehlertoleranz * WearLifespan)` is the distance to the next breakdown, a drive
    /// failure is `error distance > Fehlertoleranz * WearLifespan`). 0 would break the vehicle at
    /// the first metre, so the default is [`WEARLIFESPAN_NO_BREAKDOWNS`].
    pub wearlifespan: f32,
    /// Precipitation 0..1.
    pub precip_rate: f32,
    /// 0 none, 1 rain, 2 snow (the `.owt` coding ❓).
    pub precip_type: f32,
    /// Seconds since midnight.
    pub time_of_day_s: f32,
    /// Calendar day, month, year, day of year.
    pub day: u32,
    /// Month.
    pub month: u32,
    /// Year.
    pub year: i32,
    /// Day of the year.
    pub day_of_year: u32,
    /// Sun elevation (deg).
    pub sun_alt_deg: f32,
    /// The simulation is paused.
    pub pause: bool,
    /// Outside temperature (°C).
    pub temperature_c: f32,
    /// Absolute humidity (g/m³).
    pub abs_hum: f32,
    /// Automatic clutch setting.
    pub auto_clutch: bool,
    /// Last collision: energy (J) and position in vehicle axes.
    pub coll_energy: f32,
    /// Collision position.
    pub coll_pos: [f32; 3],
    /// Axles, front first.
    pub axles: Vec<AxleInput>,
}

impl Default for HostInputs {
    fn default() -> Self {
        Self {
            velocity_km_h: 0.0,
            velocity_ground_km_h: 0.0,
            n_wheel_rpm: 0.0,
            a_trans: [0.0; 3],
            odometer_m: 0.0,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            ai: false,
            ai_engine: 0.0,
            ai_target_index: -1.0,
            schedule_active: false,
            humans_count: 0,
            pax_entry_req: [false; DOORS],
            pax_exit_req: [false; DOORS],
            envir_brightness: 1.0,
            street_cond: 0.0,
            dirt_rate: 0.0,
            wearlifespan: WEARLIFESPAN_NO_BREAKDOWNS,
            precip_rate: 0.0,
            precip_type: 0.0,
            time_of_day_s: 12.0 * 3600.0,
            day: 29,
            month: 6,
            year: 2026,
            day_of_year: 180,
            sun_alt_deg: 60.0,
            pause: false,
            temperature_c: 15.0,
            abs_hum: 8.0,
            auto_clutch: true,
            coll_energy: 0.0,
            coll_pos: [0.0; 3],
            axles: Vec::new(),
        }
    }
}

impl HostInputs {
    /// The value of a scalar row.
    #[must_use]
    pub fn value(&self, row: &InputRow) -> f32 {
        let b = |v: bool| f32::from(u8::from(v));
        match (row.name, row.kind) {
            ("velocity", _) => self.velocity_km_h,
            ("velocity_ground", _) => self.velocity_ground_km_h,
            ("n_wheel", _) => self.n_wheel_rpm,
            ("a_trans_x", _) => self.a_trans[0],
            ("a_trans_y", _) => self.a_trans[1],
            ("a_trans_z", _) => self.a_trans[2],
            ("kmcounter_km", _) => (self.odometer_m / 1000.0).floor() as f32,
            ("kmcounter_m", _) => self.odometer_m.rem_euclid(1000.0) as f32,
            ("throttle", _) => self.throttle,
            ("brake", _) => self.brake,
            ("clutch", _) => self.clutch,
            ("ai", _) => b(self.ai),
            ("ai_engine", _) => self.ai_engine,
            ("ai_target_index", _) => self.ai_target_index,
            ("schedule_active", _) => b(self.schedule_active),
            ("humans_count", _) => self.humans_count as f32,
            ("envir_brightness", _) => self.envir_brightness,
            ("streetcond", _) => self.street_cond,
            ("dirtrate", _) => self.dirt_rate,
            ("wearlifespan", _) => self.wearlifespan,
            ("preciprate", _) => self.precip_rate,
            ("preciptype", _) => self.precip_type,
            ("time", _) => self.time_of_day_s,
            ("day", _) => self.day as f32,
            ("month", _) => self.month as f32,
            ("year", _) => self.year as f32,
            ("dayofyear", _) => self.day_of_year as f32,
            ("sunalt", _) => self.sun_alt_deg,
            ("pause", _) => b(self.pause),
            ("weather_temperature", _) => self.temperature_c,
            ("weather_abshum", _) => self.abs_hum,
            ("autoclutch", _) => b(self.auto_clutch),
            ("coll_energy", _) => self.coll_energy,
            ("coll_pos_x", _) => self.coll_pos[0],
            ("coll_pos_y", _) => self.coll_pos[1],
            ("coll_pos_z", _) => self.coll_pos[2],
            _ => 0.0,
        }
    }

    /// The value of a wheel family member (0 beyond the vehicle's axles).
    #[must_use]
    pub fn wheel(&self, family: WheelFamily, axle: usize, right: bool) -> f32 {
        let Some(a) = self.axles.get(axle) else {
            return 0.0;
        };
        let w = if right { &a.right } else { &a.left };
        match family {
            WheelFamily::RotationSpeed => w.speed_rpm,
            WheelFamily::Rotation => w.rotation_rad,
            WheelFamily::Steering => w.steering_rad,
            WheelFamily::Suspension => w.suspension_m,
        }
    }
}

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

#[cfg(test)]
mod tests;
