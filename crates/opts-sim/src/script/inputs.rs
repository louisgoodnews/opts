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

mod plan;
mod table;
pub use plan::{feed, output_row, Consumer, FeedPlan, OutputRow, OUTPUT_TABLE};
pub use table::INPUT_TABLE;

mod defaults;
mod families;
mod host;
mod schema;

pub use families::{
    AxleInput, WheelFamily, WheelInput, AXLES, DOORS, DOOR_FAMILIES, WEARLIFESPAN_NO_BREAKDOWNS,
    WHEEL_FAMILIES,
};
pub use schema::{InputRow, Kind, Source};

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

#[cfg(test)]
mod tests;
