//! `drive.snapshot`: one document with the state, screen, drive, HUD, camera, simulation and the latest warnings, optionally with a screenshot.

use super::*;

fn result_of(outcome: Outcome) -> Value {
    match outcome {
        Outcome::Done(Ok(v)) => v,
        Outcome::Done(Err(e)) => json!({ "error": e }),
        Outcome::Wait(_) => json!({ "error": "not immediate" }),
    }
}

/// The path, the screenshot flag and the number of warnings of a `drive.snapshot` request.
fn snapshot_params(params: &Value) -> Result<(Option<String>, bool, usize), RpcError> {
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_string);
    let shot = opts_bevy::control::params::opt_bool(params, "screenshot")?.unwrap_or(false);
    let n = opt_u64(params, "warnings")?.unwrap_or(20) as usize;
    Ok((path, shot, n))
}

/// The parts of the snapshot that need no waiting: state, screen, drive, HUD, camera, simulation
/// and the latest warnings.
fn snapshot_document(world: &mut World, n_warn: usize) -> serde_json::Map<String, Value> {
    let mut doc = serde_json::Map::new();
    doc.insert(
        "state".into(),
        result_of(dispatch(world, "state", &Value::Null)),
    );
    doc.insert("screen".into(), crate::control_ui::screen_json(world));
    if let Some(d) = drive_state_json(world) {
        doc.insert("drive".into(), d);
    }
    if world
        .get_resource::<crate::hud::build::ActiveProfile>()
        .is_some()
    {
        doc.insert(
            "hud".into(),
            result_of(dispatch(world, "hud.state", &Value::Null)),
        );
    }
    doc.insert(
        "camera".into(),
        result_of(dispatch(world, "camera.get", &Value::Null)),
    );
    doc.insert(
        "sim".into(),
        result_of(dispatch(world, "sim.get", &Value::Null)),
    );
    let warnings: Vec<Value> = opts_logging::recent_logs(
        n_warn,
        &opts_logging::LogQuery {
            min_level: Some(tracing::Level::WARN),
            ..Default::default()
        },
    )
    .into_iter()
    .map(|r| json!({ "ts": r.ts, "level": r.level, "target": r.target, "message": r.message }))
    .collect();
    doc.insert("warnings".into(), Value::Array(warnings));
    doc
}

pub(in crate::control_drive) fn snapshot(world: &mut World, params: &Value) -> Outcome {
    let (path, want_shot, n_warn) = match snapshot_params(params) {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let json_path = match path.as_deref() {
        Some(p) => match world
            .resource::<opts_bevy::control::PathPolicy>()
            .check_write(std::path::Path::new(p))
        {
            Ok(p) => Some(p),
            Err(e) => return Outcome::err(e),
        },
        None => None,
    };
    let mut doc = snapshot_document(world, n_warn);
    let finish = move |doc: serde_json::Map<String, Value>| -> Result<Value, RpcError> {
        let value = Value::Object(doc);
        if let Some(p) = &json_path {
            let text = serde_json::to_string_pretty(&value).unwrap_or_default();
            std::fs::write(p, text).map_err(|e| {
                RpcError::new(code::FAILED, format!("cannot write {}: {e}", p.display()))
            })?;
        }
        Ok(value)
    };
    if !want_shot {
        return Outcome::from(finish(doc));
    }
    match dispatch(world, "screenshot", &Value::Null) {
        Outcome::Wait(mut waiter) => Outcome::Wait(Box::new(move |world| {
            let shot = waiter(world)?;
            let mut doc = std::mem::take(&mut doc);
            match shot {
                Ok(s) => doc.insert("screenshot".into(), s),
                Err(e) => doc.insert("screenshot".into(), json!({ "error": e })),
            };
            Some(finish(doc))
        })),
        Outcome::Done(Err(e)) => {
            doc.insert("screenshot".into(), json!({ "error": e }));
            Outcome::from(finish(doc))
        }
        Outcome::Done(Ok(s)) => {
            doc.insert("screenshot".into(), s);
            Outcome::from(finish(doc))
        }
    }
}
