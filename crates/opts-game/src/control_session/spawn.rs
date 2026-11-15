//! Spawning, switching, despawning and respawning session vehicles.


use super::*;


pub(super) fn spawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let text = opts_bevy::control::params::str_param(params, "vehicle")?;
        let (catalogue_id, name, file) = resolve_vehicle(world, text)?;
        let setvars = match params.get("setvars") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Object(map)) => {
                let mut v = Vec::new();
                for (k, val) in map {
                    let n = val
                        .as_i64()
                        .or_else(|| val.as_bool().map(i64::from))
                        .ok_or_else(|| {
                            RpcError::invalid_params(format!("setvars.{k} must be 0, 1 or a bool"))
                        })?;
                    v.push((k.clone(), n as i32));
                }
                v.sort();
                v
            }
            Some(_) => {
                return Err(RpcError::invalid_params(
                    "`setvars` must be an object {variable: value}",
                ))
            }
        };
        let entry = match stop_param(world, params)? {
            Some(station) => EntrySel::Stop(station),
            None => entry_param(params)?,
        };
        let id = world
            .resource_mut::<SessionVehicles>()
            .request_spawn(SpawnParams {
                catalogue_id,
                name,
                bus_file: file.display().to_string(),
                repaint: opt_str(params, "repaint")?.unwrap_or_default().to_owned(),
                setvars,
                entry,
                formation: None,
            });
        Ok(json!({ "id": id, "queued": true }))
    })();
    result.into()
}

pub(super) fn switch(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = match (
            id_param(params)?,
            params.get("step").and_then(Value::as_i64),
        ) {
            (Some(id), _) => {
                ops::switch_to(world, id).map_err(op_error)?;
                id
            }
            (None, Some(step)) => ops::switch_step(world, step as i32).map_err(op_error)?,
            (None, None) => {
                return Err(RpcError::invalid_params("give `id` or `step`"));
            }
        };
        Ok(json!({ "ok": true, "active": id }))
    })();
    result.into()
}

pub(super) fn despawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        let out = ops::despawn(world, id, DespawnReason::User).map_err(op_error)?;
        Ok(json!({ "ok": true, "active": out.active, "removed": out.removed }))
    })();
    result.into()
}

pub(super) fn respawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        ops::respawn(world, id).map_err(op_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}
