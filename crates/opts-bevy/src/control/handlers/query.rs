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
    if let Some(q) = world.get_resource::<crate::light_quality::LightQuality>() {
        out["light_quality"] = light_quality_json(q);
        if let Some(g) = world.get_resource::<crate::light_quality::GovernorState>() {
            out["light_quality"]["target_ms"] = json!(g.target_ms);
            out["light_quality"]["late_share"] = json!(g.late_share);
            out["light_quality"]["up_wait_s"] = json!(g.up_wait_s);
        }
        if let Some(b) = world.get_resource::<crate::light_quality::GovernorBudget>() {
            out["light_quality"]["vsync"] = json!(b.vsync);
        }
    }
    out["reflections"] = reflections_json(world);
    if let Some(c) = world.get_resource::<crate::light_cones::ConeStats>() {
        out["light_cones"] = json!({
            "total": c.total,
            "eligible": c.eligible,
            "active": c.active,
            "hidden_by_budget": c.over_budget,
            "select_us": c.select_us,
        });
    }
    Outcome::ok(out)
}

/// The reflection features as they run (what `sim.get` reports as `reflections`): the shared sky
/// cube, the glass look and the mirror cameras.
fn reflections_json(world: &World) -> Value {
    let mut out = json!({});
    if let Some(c) = world.get_resource::<crate::sky_cube::SkyCubeState>() {
        out["sky_cube"] = json!({
            "on": c.handle.is_some(),
            "builds": c.builds,
            "intensity": c.intensity,
        });
    }
    if let Some(g) = world.get_resource::<crate::glass::GlassStats>() {
        out["glass"] = json!({"registered": g.registered, "reflective": g.reflective});
    }
    if let Some(m) = world.get_resource::<crate::mirrors::MirrorStats>() {
        out["mirrors"] = json!({
            "cameras": m.cameras,
            "selected": m.selected,
            "rendering": m.rendering,
        });
    }
    out
}

/// The lighting quality as the effects see it (what `sim.get` reports as `light_quality`).
fn light_quality_json(q: &crate::light_quality::LightQuality) -> Value {
    json!({
        "mode": q.mode.name(),
        "level": q.level,
        "level_name": crate::light_quality::level_name(q.level),
        "values": {
            "shafts": q.shafts.name(),
            "shaft_steps": q.shafts.steps(),
            "night_shafts": q.night_shafts,
            "lantern_cones": q.lantern_cones,
            "headlight_cones": q.headlight_cones,
            "cone_cap": q.cone_cap,
            "cone_depth_fade": q.cone_depth_fade,
            "mirror_count": q.mirror_count,
            "mirror_every": q.mirror_every,
            "glass_reflections": q.glass_reflections,
            "bloom": q.bloom,
        },
    })
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

    #[test]
    fn sim_get_reports_the_lighting_quality_when_the_app_has_one() {
        let mut world = World::new();
        world.insert_resource(Time::<Virtual>::default());
        world.insert_resource(Time::<Real>::default());
        world.insert_resource(ControlState::default());
        let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
            panic!("sim.get did not answer at once");
        };
        assert!(v.get("light_quality").is_none());
        world.insert_resource(crate::light_quality::ladder_preset(2));
        let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
            panic!("sim.get did not answer at once");
        };
        let l = &v["light_quality"];
        assert_eq!(l["mode"], "automatic");
        assert_eq!(l["level"], 2);
        assert_eq!(l["level_name"], "headlights-only");
        assert_eq!(l["values"]["shaft_steps"], 16);
        assert_eq!(l["values"]["cone_cap"], 16);
        assert_eq!(l["values"]["lantern_cones"], false);
        assert_eq!(l["values"]["headlight_cones"], true);
        assert!(l.get("target_ms").is_none());
        world.insert_resource(crate::light_quality::GovernorState::default());
        world.insert_resource(crate::light_quality::GovernorBudget::default());
        let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
            panic!("sim.get did not answer at once");
        };
        let l = &v["light_quality"];
        assert!((l["target_ms"].as_f64().unwrap() - 16.667).abs() < 0.01);
        assert_eq!(l["late_share"], 0.0);
        assert_eq!(l["vsync"], false);
        assert!(v.get("light_cones").is_none());
        world.insert_resource(crate::light_cones::ConeStats {
            total: 120,
            eligible: 80,
            active: 64,
            over_budget: 16,
            select_us: 30,
        });
        let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
            panic!("sim.get did not answer at once");
        };
        assert_eq!(v["light_cones"]["active"], 64);
        assert_eq!(v["light_cones"]["hidden_by_budget"], 16);
    }
}
