//! One script engine per placed scenery object and the host values it is fed every tick.

use std::collections::HashMap;
use std::sync::Mutex;

use bevy::prelude::*;
use opts_sim::script::{Diagnostic, ScriptEngine};

use super::clock::SceneryClock;

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

impl SceneryScripts {
    /// Wraps a (forked) engine: writes the placed object's string variables (`names[i]` gets
    /// `values[i]`) and keeps the load diagnostics. `{init}` runs on the first tick.
    #[must_use]
    pub fn new(
        mut engine: ScriptEngine,
        names: &[String],
        values: &[String],
        load_diagnostics: Vec<Diagnostic>,
    ) -> Self {
        for (name, value) in names.iter().zip(values) {
            engine.set_string(name, value);
        }
        let ids = HostIds::resolve(&mut engine);
        let mut diagnostics = load_diagnostics;
        diagnostics.truncate(MAX_KEPT_DIAGNOSTICS);
        Self {
            engine: Mutex::new(engine),
            ids,
            initialised: false,
            diagnostics,
            ticks: 0,
            raised: Vec::new(),
        }
    }

    /// Runs `f` on the engine (to read or write variables); `None` if the lock is poisoned.
    pub fn with_engine<R>(&self, f: impl FnOnce(&mut ScriptEngine) -> R) -> Option<R> {
        self.engine.lock().ok().map(|mut e| f(&mut e))
    }

    /// The engine for exclusive access inside a system.
    pub fn engine_mut(&mut self) -> Option<&mut ScriptEngine> {
        self.engine.get_mut().ok()
    }

    /// Whether the script raised events that nobody has taken yet.
    #[must_use]
    pub fn has_raised(&self) -> bool {
        !self.raised.is_empty()
    }

    /// Takes the events the script raised since the last call (lower case names, oldest first).
    pub fn take_raised(&mut self) -> Vec<String> {
        std::mem::take(&mut self.raised)
    }

    /// Load-time and run-time diagnostics kept for this object.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Feeds the host values, runs `{init}` on the first call and `{frame}` with step `dt_s`.
    /// `phase` is the object's `TrafficLightPhase`. Returns the number of new run-time
    /// diagnostics.
    pub fn tick(
        &mut self,
        host: &HostValues,
        phase: Option<f32>,
        overrides: &SceneryVariableOverrides,
        dt_s: f32,
    ) -> usize {
        let ids = self.ids;
        let first = !self.initialised;
        self.initialised = true;
        self.ticks += 1;
        let Ok(e) = self.engine.get_mut() else {
            return 0;
        };
        e.set_system_id(ids.time, host.time_s);
        e.set_system_id(ids.day, host.day);
        e.set_system_id(ids.month, host.month);
        e.set_system_id(ids.year, host.year);
        e.set_system_id(ids.day_of_year, host.day_of_year);
        e.set_system_id(ids.sun_alt, host.sun_alt_deg);
        e.set_system_id(ids.temperature, host.temperature_c);
        e.set_system_id(ids.abs_hum, host.abs_hum);
        e.set_system_id(ids.precip_type, host.precip_type);
        e.set_system_id(ids.precip_rate, host.precip_rate);
        e.set_id(ids.night, host.night);
        if let Some(p) = phase {
            e.set_id(ids.phase, p);
        }
        for (name, value) in &overrides.0 {
            e.set(name, *value);
        }
        if first {
            e.run_init();
        }
        e.run_frame(dt_s, false);
        // The events the script raised (`(T.L.name)`, lower case) wait for the sound layer
        // (`opts_game::scenery_sound`), which plays the `[trigger]`s of the object's sounds.
        for ev in e.drain_events() {
            if self.raised.len() < MAX_RAISED {
                self.raised.push(ev.name);
            }
        }
        e.drain_messages();
        let new = e.drain_diagnostics();
        let count = new.len();
        for d in new {
            tracing::debug!(file = %d.file, line = d.line, "scenery script: {}", d.message);
            if self.diagnostics.len() < MAX_KEPT_DIAGNOSTICS {
                self.diagnostics.push(d);
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_values_and_overrides_reach_the_script() {
        let (engine, _) = ScriptEngine::from_sources(&[(
            "t.osc",
            "{frame}\n (L.S.Time) (S.L.t)\n (L.L.TrafficLightPhase) (S.L.p)\n \
             (L.L.NightlightA) (S.L.n)\n (L.S.Month) (S.L.m)\n (L.L.knob) (S.L.k)\n{end}",
        )]);
        let mut s = SceneryScripts::new(engine, &[], &[], Vec::new());
        let clock = SceneryClock::at(2026, 12, 24, 22.0 * 3600.0);
        let mut overrides = SceneryVariableOverrides::default();
        overrides.0.insert("Knob".into(), 4.0);
        s.tick(&HostValues::of(&clock), Some(7.0), &overrides, 0.1);
        let v = s
            .with_engine(|e| [e.get("t"), e.get("p"), e.get("n"), e.get("m"), e.get("k")])
            .unwrap();
        assert_eq!(v, [22.0 * 3600.0, 7.0, 1.0, 12.0, 4.0]);
        assert!(s.initialised && s.ticks == 1);
    }

    #[test]
    fn string_variables_come_from_the_placed_object() {
        let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{init}\n{end}")]);
        let s = SceneryScripts::new(
            engine,
            &["Name".into(), "Line".into()],
            &["Hauptbahnhof".into()],
            Vec::new(),
        );
        let got = s
            .with_engine(|e| {
                (
                    e.get_string("name").to_owned(),
                    e.get_string("line").to_owned(),
                )
            })
            .unwrap();
        assert_eq!(got, ("Hauptbahnhof".to_string(), String::new()));
    }
}
