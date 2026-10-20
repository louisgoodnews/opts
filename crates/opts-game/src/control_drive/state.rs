//! The drive state of the protocol: the absolute position, the drive-state JSON and the snapshot of the vehicle.

use super::*;

pub(super) fn absolute_position(world: &World, local: Vec2) -> (f64, f64) {
    let (start, grid) = world
        .get_resource::<WorldStats>()
        .map_or(((0, 0), Default::default()), |s| (s.start_tile, s.grid));
    let abs = grid.local_to_absolute(local.as_dvec2(), start);
    (abs.x, abs.y)
}

/// The drive state as JSON; `None` without a player vehicle.
pub fn drive_state_json(world: &mut World) -> Option<Value> {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    let c = q.iter(world).next()?.clone();
    let ch = &c.sim.chassis;
    let i = c.inputs();
    let (x, y) = absolute_position(world, ch.position);
    let signed = ch.forward_speed_km_h();
    let virt = world.resource::<Time<Virtual>>();
    let (paused, scale, sim_time) = (
        virt.is_paused(),
        virt.relative_speed(),
        virt.elapsed_secs_f64(),
    );
    let distance = world
        .get_resource::<RemoteControls>()
        .map_or(0.0, |r| r.odometer_m);
    let alert = world
        .get_resource::<crate::hud::alerts::AlertQueue>()
        .and_then(|q| {
            q.visible()
                .map(|a| json!({ "id": a.id, "severity": a.severity.id() }))
        });
    let camera = world.get_resource::<CameraState>().map(|c| c.mode.label());
    let mouse = world.get_resource::<opts_bevy::MouseDriving>().map(
        |m| json!({ "enabled": m.enabled, "on": m.on, "freed": m.freed, "steering": m.steering }),
    );
    Some(json!({
        "speed_kmh": signed.abs(),
        "signed_speed_kmh": signed,
        "x": x, "y": y, "z": ch.elevation_z,
        "heading_deg": ch.heading_rad.to_degrees(),
        "gear": gear_name(i.gear()),
        "throttle": i.throttle(),
        "brake": i.service_brake(),
        "steer": i.steering(),
        "parking_brake": i.is_parking_brake_engaged(),
        "mouse_driving": mouse,
        "rpm": crate::hud::values::RPM_PROVIDED.then(|| c.engine_rpm()),
        "distance_m": distance,
        "camera": camera,
        "paused": paused,
        "time_scale": scale,
        "sim_time": sim_time,
        "alert": alert,
        // The timetable is not implemented yet; both are reported as unavailable, not guessed.
        "route": null,
        "stops": null,
    }))
}

pub(super) fn drive_state(world: &mut World, _: &Value) -> Outcome {
    match drive_state_json(world) {
        Some(v) => Outcome::ok(v),
        None => Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        )),
    }
}

fn result_of(outcome: Outcome) -> Value {
    match outcome {
        Outcome::Done(Ok(v)) => v,
        Outcome::Done(Err(e)) => json!({ "error": e }),
        Outcome::Wait(_) => json!({ "error": "not immediate" }),
    }
}

pub(super) fn snapshot(world: &mut World, params: &Value) -> Outcome {
    let parsed = (|| -> Result<(Option<String>, bool, usize), RpcError> {
        let path = params
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string);
        let shot = opts_bevy::control::params::opt_bool(params, "screenshot")?.unwrap_or(false);
        let n = opt_u64(params, "warnings")?.unwrap_or(20) as usize;
        Ok((path, shot, n))
    })();
    let (path, want_shot, n_warn) = match parsed {
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
