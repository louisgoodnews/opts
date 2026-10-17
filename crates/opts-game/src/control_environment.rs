//! `sim.set_time`: sets the session environment's clock, and optionally its date, weather and
//! clock rate, through an [`EnvironmentChange`] (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::control::params::{f64_param, opt_f64, opt_str};
use opts_bevy::control::state::{ControlRegistry, Outcome};
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

    fn installed() -> WeatherCatalogue {
        use opts_core::formats::weather::{PresetFile, WeatherFile, WeatherFolder};
        let preset = |id: &str| PresetFile {
            id: id.into(),
            weather: WeatherFile::default(),
            texts: Default::default(),
        };
        WeatherCatalogue::from_folder(
            "/omsi".into(),
            WeatherFolder {
                presets: vec![preset("#CAVOK"), preset("Sturm"), preset("Schmuddelwetter")],
                ..default()
            },
        )
    }

    #[test]
    fn weather_names_resolve_to_presets() {
        let b = WeatherCatalogue::default();
        let w = |name| weather_preset(&b, name);
        assert_eq!(w("rain").as_deref(), Some("Rainy Afternoon"));
        assert_eq!(w("Winter Snowfall").as_deref(), Some("Winter Snowfall"));
        assert_eq!(
            w("autumn-morning-fog").as_deref(),
            Some("Autumn Morning Fog")
        );
        assert_eq!(w("clear").as_deref(), Some("Clear Summer Day"));
        assert_eq!(w("hail"), None);
    }

    #[test]
    fn installed_presets_and_old_ids_are_accepted() {
        let cat = installed();
        let w = |name| weather_preset(&cat, name);
        assert_eq!(w("sturm").as_deref(), Some("Sturm"));
        assert_eq!(w("#CAVOK").as_deref(), Some("#CAVOK"));
        assert_eq!(w("schmuddelwetter").as_deref(), Some("Schmuddelwetter"));
        // The old ids stand for the closest installed preset.
        assert_eq!(w("Rainy Afternoon").as_deref(), Some("Schmuddelwetter"));
        assert_eq!(w("clear").as_deref(), Some("#CAVOK"));
        let err = change_of(&json!({"seconds": 1.0, "weather": "hail"}), &cat).unwrap_err();
        assert!(
            err.message.contains("Sturm") && err.message.contains("#CAVOK"),
            "{err:?}"
        );
        let c = change_of(&json!({"seconds": 1.0, "weather": "Sturm"}), &cat).unwrap();
        assert_eq!(c.weather.as_deref(), Some("Sturm"));
    }

    #[test]
    fn parameters_are_checked() {
        let b = WeatherCatalogue::default();
        let c = change_of(
            &json!({"seconds": 3600.0, "date": "1994-12-31", "rate": 2.0}),
            &b,
        )
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
            assert!(change_of(&bad, &b).is_err(), "{bad}");
        }
    }
}
