//! One script engine per placed scenery object and the host values it is fed every tick.

use std::collections::HashMap;
use std::sync::Mutex;

use bevy::prelude::*;
use opts_sim::script::{Diagnostic, ScriptEngine};

use super::clock::SceneryClock;

mod methods;
mod run;
#[cfg(test)]
mod tests;

/// Most diagnostics kept per object (the harness reads them; the log gets a summary).
const MAX_KEPT_DIAGNOSTICS: usize = 32;
/// Events kept between two reads of [`SceneryScripts::take_raised`].
const MAX_RAISED: usize = 64;

/// Values every scripted object receives before `{frame}`, from the [`SceneryClock`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostValues {
    /// `Time`.
    pub time_s: f32,
    /// `Day`.
    pub day: f32,
    /// `Month`.
    pub month: f32,
    /// `Year`.
    pub year: f32,
    /// `DayOfYear`.
    pub day_of_year: f32,
    /// `SunAlt`.
    pub sun_alt_deg: f32,
    /// `Weather_Temperature`.
    pub temperature_c: f32,
    /// `Weather_AbsHum`.
    pub abs_hum: f32,
    /// `PrecipType`.
    pub precip_type: f32,
    /// `PrecipRate`.
    pub precip_rate: f32,
    /// `NightlightA` (0 or 1).
    pub night: f32,
    /// `snow_visible` (0 or 1): 1 while the session's weather is snow (`PrecipType` 2).
    pub snow_visible: f32,
    /// Clock for light programs (seconds, not wrapped).
    pub program_clock_s: f64,
}

impl HostValues {
    /// The values of `clock`.
    #[must_use]
    pub fn of(clock: &SceneryClock) -> Self {
        Self {
            time_s: clock.time_of_day_s(),
            day: clock.day as f32,
            month: clock.month as f32,
            year: clock.year as f32,
            day_of_year: clock.day_of_year() as f32,
            sun_alt_deg: clock.sun_alt_deg,
            temperature_c: clock.temperature_c,
            abs_hum: clock.weather.abs_hum,
            precip_type: clock.weather.precip_type,
            precip_rate: clock.weather.precip_rate,
            night: f32::from(u8::from(clock.night)),
            snow_visible: f32::from(u8::from((clock.weather.precip_type - 2.0).abs() < 0.5)),
            program_clock_s: clock.seconds_of_day,
        }
    }
}

/// Variables written into every scripted object after the host values (benchmark stimuli such
/// as a forced `TrafficLightPhase`). Names are case-insensitive.
#[derive(Resource, Debug, Clone, Default)]
pub struct SceneryVariableOverrides(pub HashMap<String, f32>);

/// Engine indices of the host values, resolved once per object.
#[derive(Debug, Clone, Copy)]
struct HostIds {
    time: u32,
    day: u32,
    month: u32,
    year: u32,
    day_of_year: u32,
    sun_alt: u32,
    temperature: u32,
    abs_hum: u32,
    precip_type: u32,
    precip_rate: u32,
    phase: u32,
    night: u32,
}

impl HostIds {
    fn resolve(e: &mut ScriptEngine) -> Self {
        Self {
            time: e.system_id("Time"),
            day: e.system_id("Day"),
            month: e.system_id("Month"),
            year: e.system_id("Year"),
            day_of_year: e.system_id("DayOfYear"),
            sun_alt: e.system_id("SunAlt"),
            temperature: e.system_id("Weather_Temperature"),
            abs_hum: e.system_id("Weather_AbsHum"),
            precip_type: e.system_id("PrecipType"),
            precip_rate: e.system_id("PrecipRate"),
            phase: e.variable_id("TrafficLightPhase"),
            night: e.variable_id("NightlightA"),
        }
    }
}

/// The script engine of one placed scenery object (on the object's root entity).
#[derive(Component)]
pub struct SceneryScripts {
    /// The engine (a mutex because Bevy components must be `Sync`; systems use `get_mut`).
    engine: Mutex<ScriptEngine>,
    ids: HostIds,
    /// `{init}` has run.
    pub initialised: bool,
    /// Load-time and run-time diagnostics (the first few).
    diagnostics: Vec<Diagnostic>,
    /// Ticks run so far.
    pub ticks: u64,
    /// Events the script raised since the last [`Self::take_raised`] (lower case).
    raised: Vec<String>,
}
