//! `sim.set_time`: sets the session environment's clock, and optionally its date, weather and
//! clock rate, through an [`EnvironmentChange`] (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::control::params::{f64_param, opt_f64, opt_str};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::reflections::Wetness;
use opts_bevy::{
    CalendarDate, EnvironmentChange, MapDatePolicy, SessionEnvironment, WeatherCatalogue,
};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::app_state::AppState;

/// Highest clock rate `sim.set_time` accepts.
pub const MAX_RATE: f64 = 3600.0;

/// Registers the method.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("sim.set_time", set_time);
}

/// The preset a `weather` parameter names: an id, its slug, a localised name, a word of those
/// (`rain`, `snow`) or an old built-in id, case-insensitively.
#[must_use]
pub fn weather_preset(catalogue: &WeatherCatalogue, name: &str) -> Option<String> {
    catalogue.find(name)
}

/// The `wetness` parameter (M-0121), clamped to `0..=1`; `None` when absent.
pub fn wetness_param(params: &Value) -> Result<Option<f32>, RpcError> {
    Ok(opt_f64(params, "wetness")?.map(|w| w.clamp(0.0, 1.0) as f32))
}

fn state(env: &SessionEnvironment) -> Value {
    json!({
        "clock": env.clock_text(),
        "seconds_of_day": env.seconds_of_day(),
        "date": env.date().to_string(),
        "map_date": env.map_date().to_string(),
        "weather": env.weather(),
        "rate": env.rate(),
        "revision": env.revision(),
    })
}

/// The answer after the change: the environment and the global wetness (`null` without one).
fn report(world: &World) -> Value {
    let mut answer = state(world.resource::<SessionEnvironment>());
    answer["wetness"] = json!(world.get_resource::<Wetness>().map(|w| w.0));
    answer
}

fn change_of(params: &Value, catalogue: &WeatherCatalogue) -> Result<EnvironmentChange, RpcError> {
    let seconds = f64_param(params, "seconds")?;
    if !(0.0..86_400.0).contains(&seconds) {
        return Err(RpcError::invalid_params(
            "`seconds` must be 0 or more and less than 86400 (seconds since midnight)",
        ));
    }
    let date =
        match opt_str(params, "date")? {
            Some(d) => Some(CalendarDate::parse(d).ok_or_else(|| {
                RpcError::invalid_params("`date` must be a real date as YYYY-MM-DD")
            })?),
            None => None,
        };
    let weather = match opt_str(params, "weather")? {
        Some(w) => Some(weather_preset(catalogue, w).ok_or_else(|| {
            RpcError::invalid_params(format!(
                "unknown `weather`; one of {}",
                catalogue.ids().join(", ")
            ))
        })?),
        None => None,
    };
    let rate = opt_f64(params, "rate")?;
    if rate.is_some_and(|r| !(r > 0.0 && r <= MAX_RATE)) {
        return Err(RpcError::invalid_params(format!(
            "`rate` must be greater than 0 and at most {MAX_RATE}"
        )));
    }
    Ok(EnvironmentChange {
        date,
        seconds_of_day: Some(seconds),
        weather,
        rate,
        map_date: MapDatePolicy::Follow,
    })
}

fn set_time(world: &mut World, params: &Value) -> Outcome {
    let in_game = world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::InGame);
    if !in_game || !world.contains_resource::<SessionEnvironment>() {
        return Outcome::err(RpcError::new(code::NOT_READY, "no drive is running"));
    }
    let catalogue = world
        .get_resource::<WeatherCatalogue>()
        .cloned()
        .unwrap_or_default();
    let change = match change_of(params, &catalogue) {
        Ok(c) => c,
        Err(e) => return Outcome::err(e),
    };
    let wetness = match wetness_param(params) {
        Ok(w) => w,
        Err(e) => return Outcome::err(e),
    };
    if let Some(w) = wetness {
        let Some(mut global) = world.get_resource_mut::<Wetness>() else {
            return Outcome::err(RpcError::new(
                code::NOT_READY,
                "no wetness state (start a drive first)",
            ));
        };
        global.0 = w;
    }
    world.write_message(change);
    // The change applies in the next frame's environment systems; answer with the result.
    let mut frames = 0;
    Outcome::Wait(Box::new(move |world| {
        frames += 1;
        (frames >= 2).then(|| Ok(report(world)))
    }))
}

#[cfg(test)]
mod tests;
