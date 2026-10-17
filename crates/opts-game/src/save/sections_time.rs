//! The `time` save section: the start of the session, its play time and the session environment
//! (date, time, map date, weather, clock rate) when saved (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::{CalendarDate, EnvironmentChange, MapDatePolicy, SessionEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::sections::{parse, to_value};
use crate::in_game::types::InGameSessionInfo;
use crate::new_game_screen::types::NewGameSetup;

/// Data of the `time` section.
///
/// `year`..`sim_elapsed_s` are the start of the session (what the plan said) and its play time;
/// the fields after them are the session environment when it was saved. Saves written before
/// the environment existed lack them: the clock is then start + play time and the weather the
/// `weather` section's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeData {
    /// Calendar year of the session's start.
    pub year: i32,
    /// Month 1-12.
    pub month: u32,
    /// Day 1-31.
    pub day: u32,
    /// Hour of the clock when the session started.
    pub start_hour: u32,
    /// Minute of the clock when the session started.
    pub start_minute: u32,
    /// Simulated seconds since then (the play time).
    pub sim_elapsed_s: f64,
    /// The date of the clock when saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<CalendarDate>,
    /// Seconds since midnight of the clock when saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_of_day: Option<f64>,
    /// The date the map content was built for (Chrono).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_date: Option<CalendarDate>,
    /// The weather preset when saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather: Option<String>,
    /// The clock rate when saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
}

impl TimeData {
    /// The clock to restore: the saved one, or start + play time for an old save.
    #[must_use]
    pub fn clock(&self) -> (CalendarDate, f64) {
        if let (Some(date), Some(s)) = (self.date, self.seconds_of_day) {
            return (date, s);
        }
        let start = CalendarDate::clamped(self.year, self.month, self.day);
        let total = f64::from(self.start_hour * 3600 + self.start_minute * 60)
            + self.sim_elapsed_s.max(0.0);
        let days = (total / opts_bevy::environment::DAY_S).floor();
        (
            start.add_days(days as i64),
            total - days * opts_bevy::environment::DAY_S,
        )
    }
}

pub(super) fn time_capture(world: &mut World) -> Result<Option<Value>, String> {
    let setup = world.resource::<NewGameSetup>();
    let info = world.resource::<InGameSessionInfo>();
    let env = world.get_resource::<SessionEnvironment>();
    to_value(&TimeData {
        year: setup.year,
        month: setup.month,
        day: setup.day,
        start_hour: info.start_hour,
        start_minute: info.start_minute,
        sim_elapsed_s: f64::from(info.sim_elapsed_seconds),
        date: env.map(SessionEnvironment::date),
        seconds_of_day: env.map(SessionEnvironment::seconds_of_day),
        map_date: env.map(SessionEnvironment::map_date),
        weather: env.map(|e| e.weather().to_owned()),
        rate: env.map(SessionEnvironment::rate),
    })
}

pub(super) fn time_prepare(world: &mut World, data: &Value) -> Result<(), String> {
    let d: TimeData = parse(data, "time")?;
    let mut setup = world.resource_mut::<NewGameSetup>();
    // The session loads the map for the date it was built for when saved.
    let date = d
        .map_date
        .unwrap_or_else(|| CalendarDate::new(d.year, d.month, d.day));
    setup.year = date.year;
    setup.month = date.month;
    setup.day = date.day;
    setup.hour = d.start_hour;
    setup.minute = d.start_minute;
    if let Some(w) = &d.weather {
        w.clone_into(&mut setup.weather_preset);
    }
    Ok(())
}

pub(super) fn time_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let d: TimeData = parse(data, "time")?;
    world
        .resource_mut::<InGameSessionInfo>()
        .sim_elapsed_seconds = d.sim_elapsed_s as f32;
    let (date, seconds) = d.clock();
    world.write_message(EnvironmentChange {
        date: Some(date),
        seconds_of_day: Some(seconds),
        weather: d.weather.clone(),
        rate: d.rate,
        map_date: MapDatePolicy::Keep,
    });
    Ok(())
}
