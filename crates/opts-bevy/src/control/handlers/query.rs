//! `state`, `camera.get`, `sim.get`, `log.tail`, `log.filter`.

use bevy::camera::Camera3d;
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
        world.query_filtered::<(&GlobalTransform, Option<&Projection>), With<Camera3d>>();
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
    Outcome::ok(json!({
        "paused": virt.is_paused(),
        "sim_time": virt.elapsed_secs_f64(),
        "time_scale": virt.relative_speed(),
        "tick": tick,
    }))
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
