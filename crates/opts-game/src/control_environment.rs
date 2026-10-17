//! `sim.set_time`: sets the session environment's clock, and optionally its date, weather and
//! clock rate, through an [`EnvironmentChange`] (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::control::params::{f64_param, opt_f64, opt_str};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{CalendarDate, EnvironmentChange, MapDatePolicy, SessionEnvironment};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::new_game_screen::weather::PRESETS;

/// Highest clock rate `sim.set_time` accepts.
pub const MAX_RATE: f64 = 3600.0;

/// Registers the method.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("sim.set_time", set_time);
}

/// The preset a `weather` parameter names: its id, its slug or a word of it (`rain`, `snow`),
/// case-insensitively.
#[must_use]
pub fn weather_preset(name: &str) -> Option<&'static str> {
    let n = name.trim().to_lowercase();
    if n.is_empty() {
        return None;
    }
    PRESETS
        .iter()
        .find(|p| p.id.to_lowercase() == n || opts_gui::ui_id::slug(p.id) == n)
        .or_else(|| {
            PRESETS
                .iter()
                .find(|p| p.id.to_lowercase().split(' ').any(|w| w.starts_with(&n)))
        })
        .map(|p| p.id)
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

fn change_of(params: &Value) -> Result<EnvironmentChange, RpcError> {
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
        Some(w) => Some(weather_preset(w).ok_or_else(|| {
            let ids: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
            RpcError::invalid_params(format!("unknown `weather`; one of {}", ids.join(", ")))
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
        weather: weather.map(str::to_owned),
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
    let change = match change_of(params) {
        Ok(c) => c,
        Err(e) => return Outcome::err(e),
    };
    world.write_message(change);
    // The change applies in the next frame's environment systems; answer with the result.
    let mut frames = 0;
    Outcome::Wait(Box::new(move |world| {
        frames += 1;
        (frames >= 2).then(|| Ok(state(world.resource::<SessionEnvironment>())))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_names_resolve_to_presets() {
        assert_eq!(weather_preset("rain"), Some("Rainy Afternoon"));
        assert_eq!(weather_preset("Winter Snowfall"), Some("Winter Snowfall"));
        assert_eq!(
            weather_preset("autumn-morning-fog"),
            Some("Autumn Morning Fog")
        );
        assert_eq!(weather_preset("clear"), Some("Clear Summer Day"));
        assert_eq!(weather_preset("hail"), None);
    }

    #[test]
    fn parameters_are_checked() {
        let c = change_of(&json!({"seconds": 3600.0, "date": "1994-12-31", "rate": 2.0}))
            .expect("valid");
        assert_eq!(c.date, Some(CalendarDate::new(1994, 12, 31)));
        assert_eq!(c.map_date, MapDatePolicy::Follow);
        for bad in [
            json!({"seconds": -1.0}),
            json!({"seconds": 86_400.0}),
            json!({"seconds": 1.0, "date": "1994-02-30"}),
            json!({"seconds": 1.0, "weather": "hail"}),
            json!({"seconds": 1.0, "rate": 0.0}),
            json!({}),
        ] {
            assert!(change_of(&bad).is_err(), "{bad}");
        }
    }
}
