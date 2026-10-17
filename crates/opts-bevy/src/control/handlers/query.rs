//! `state`, `camera.get`, `sim.get`, `log.tail`, `log.filter`.

use bevy::ecs::world::World;
use bevy::prelude::*;
use bevy::time::{Real, Time, Virtual};
use opts_cli::protocol::RpcError;
use opts_logging::{LogQuery, LogRecord};
use serde_json::{json, Map, Value};

use crate::camera::CameraState;
use crate::control::params::{opt_str, opt_u64};
use crate::control::state::{ControlRegistry, ControlState, Outcome};

/// Registers the query methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("state", state);
    r.register("camera.get", camera_get);
    r.register("sim.get", sim_get);
    r.register("log.tail", log_tail);
    r.register("log.filter", log_filter);
}

/// Facts every application has; the game adds app state, map and vehicle on top.
#[must_use]
pub fn base_state(world: &World) -> Map<String, Value> {
    let control = world.resource::<ControlState>();
    let mut m = Map::new();
    m.insert("frame".into(), json!(control.frame));
    m.insert(
        "fps".into(),
        json!((f64::from(control.fps) * 10.0).round() / 10.0),
    );
    let worst = control
        .frame_log
        .iter()
        .map(|(_, ms)| *ms)
        .fold(0.0, f32::max);
    m.insert(
        "frame_ms_max".into(),
        json!((f64::from(worst) * 10.0).round() / 10.0),
    );
    if let Some(cam) = world.get_resource::<CameraState>() {
        m.insert("camera_mode".into(), json!(cam.mode.label()));
    }
    m
}

fn state(world: &mut World, _: &Value) -> Outcome {
    Outcome::ok(Value::Object(base_state(world)))
}

fn camera_get(world: &mut World, _: &Value) -> Outcome {
    let mut out = Map::new();
    if let Some(cam) = world.get_resource::<CameraState>() {
        out.insert("mode".into(), json!(cam.mode.label()));
        out.insert("sub_index".into(), json!(cam.sub_camera_index));
    }
    let mut query =
        world.query_filtered::<(&GlobalTransform, Option<&Projection>), With<crate::MainCamera>>();
    if let Some((tf, projection)) = query.iter(world).next() {
        let (t, r) = (tf.translation(), tf.to_scale_rotation_translation().1);
        out.insert(
            "transform".into(),
            json!({ "translation": [t.x, t.y, t.z], "rotation": [r.x, r.y, r.z, r.w] }),
        );
        if let Some(Projection::Perspective(p)) = projection {
            out.insert("fov".into(), json!(p.fov.to_degrees()));
        }
    }
    Outcome::ok(Value::Object(out))
}

fn sim_get(world: &mut World, _: &Value) -> Outcome {
    let virt = world.resource::<Time<Virtual>>();
    let tick = world.resource::<ControlState>().frame;
    let _ = world.resource::<Time<Real>>();
    let mut out = json!({
        "paused": virt.is_paused(),
        "sim_time": virt.elapsed_secs_f64(),
        "time_scale": virt.relative_speed(),
        "tick": tick,
    });
    // The session clock (sky, traffic, scenery), when the app has one.
    if let Some(env) = world.get_resource::<crate::environment::SessionEnvironment>() {
        out["clock"] = json!(env.clock_text());
        out["seconds_of_day"] = json!(env.seconds_of_day());
        out["date"] = json!(env.date().to_string());
        out["map_date"] = json!(env.map_date().to_string());
        out["weather"] = json!(env.weather());
        out["clock_rate"] = json!(env.rate());
        out["weather_conditions"] = weather_json(env);
        if let Some(cat) = world.get_resource::<crate::weather::WeatherCatalogue>() {
            out["weather_canonical"] = json!(cat.canonical_id(env.weather()));
            out["weather_presets"] = json!(cat.ids());
        }
    }
    Outcome::ok(out)
}

/// The resolved weather as the effects see it (what `sim.get` reports as `weather_conditions`).
fn weather_json(env: &crate::environment::SessionEnvironment) -> Value {
    let c = env.conditions();
    json!({
        "fog_visibility_m": c.fog_visibility_m,
        "wind_from_deg": c.wind_from_deg,
        "wind_speed": c.wind_speed,
        "temperature_c": c.temperature_c,
        "dew_point_c": c.dew_point_c,
        "pressure_hpa": c.pressure_hpa,
        "clouds": c.cloud.as_ref().map(|cl| json!({"type": cl.name, "base_m": cl.base_m})),
        "precip_type": c.precip.code(),
        "precip_rate": c.precip_rate,
        "wetness": crate::weather::surface_state(c).wet,
        "snow": c.snow,
        "sun_scale": env.attenuation(),
    })
}

fn records(list: Vec<LogRecord>) -> Value {
    let lines: Vec<Value> = list
        .into_iter()
        .map(|r| json!({ "ts": r.ts, "level": r.level, "target": r.target, "message": r.message }))
        .collect();
    json!({ "lines": lines })
}

fn log_tail(_: &mut World, params: &Value) -> Outcome {
    match opt_u64(params, "n") {
        Ok(n) => Outcome::ok(records(opts_logging::recent_logs(
            n.unwrap_or(100) as usize,
            &LogQuery::default(),
        ))),
        Err(e) => Outcome::err(e),
    }
}

fn parse_level(text: &str) -> Result<tracing::Level, RpcError> {
    text.parse::<tracing::Level>().map_err(|_| {
        RpcError::invalid_params("`level` must be one of trace, debug, info, warn, error")
    })
}

fn log_filter(_: &mut World, params: &Value) -> Outcome {
    let query = (|| -> Result<(usize, LogQuery), RpcError> {
        let level = opt_str(params, "level")?.map(parse_level).transpose()?;
        Ok((
            opt_u64(params, "n")?.unwrap_or(100) as usize,
            LogQuery {
                target: opt_str(params, "target")?.map(str::to_string),
                min_level: level,
                contains: opt_str(params, "contains")?.map(str::to_string),
            },
        ))
    })();
    match query {
        Ok((n, q)) => Outcome::ok(records(opts_logging::recent_logs(n, &q))),
        Err(e) => Outcome::err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{CalendarDate, EnvironmentChange, SessionEnvironment};

    #[test]
    fn sim_get_reports_the_resolved_weather_and_the_available_presets() {
        let mut world = World::new();
        world.insert_resource(Time::<Virtual>::default());
        world.insert_resource(Time::<Real>::default());
        world.insert_resource(ControlState::default());
        world.insert_resource(crate::weather::WeatherCatalogue::default());
        let mut env = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 43_200.0, "");
        env.apply(&EnvironmentChange {
            weather: Some("Winter Snowfall".into()),
            ..Default::default()
        });
        world.insert_resource(env);
        let out = sim_get(&mut world, &Value::Null);
        let Outcome::Done(Ok(v)) = out else {
            panic!("sim.get did not answer at once");
        };
        assert_eq!(v["weather"], "Winter Snowfall");
        assert_eq!(v["weather_canonical"], "Winter Snowfall");
        assert_eq!(v["weather_presets"].as_array().map(Vec::len), Some(5));
        let c = &v["weather_conditions"];
        assert_eq!(c["precip_type"], 2);
        assert_eq!(c["snow"], true);
        assert_eq!(c["temperature_c"], -3.0);
        assert!(c["sun_scale"].as_f64().unwrap() < 0.6);
    }
}
