//! Load, spawn, delete, update and duplicate handlers for the bus configuration control protocol.

use super::*;

pub(super) fn load(world: &mut World, params: &Value) -> Outcome {
    let mut run = || -> Result<Value, RpcError> {
        let key = str_param(params, "id")?.to_string();
        let (path, mut store) = open(world)?;
        let config = store.find(&key).cloned().ok_or_else(|| not_found(&key))?;
        let cat = catalogue(world);
        let reps = Repaints::new(world);
        let checked = config.check(&cat, |v| reps.of(v));
        let problems: Vec<Value> = checked
            .problems
            .iter()
            .map(|p| json!({ "code": p.code(), "message": p.to_string() }))
            .collect();
        if !checked.usable() {
            return Err(RpcError::new(
                code::FAILED,
                format!(
                    "configuration `{}` cannot be loaded: {}",
                    config.name, checked.problems[0]
                ),
            )
            .with_data(json!({ "problems": problems })));
        }
        if drive_vehicle(world, &cat).is_some() {
            return Err(RpcError::new(
                code::UNSUPPORTED,
                "configurations are loaded in Plan a drive; a running drive is not changed",
            ));
        }
        let Some(mut setup) = world.get_resource_mut::<NewGameSetup>() else {
            return Err(RpcError::new(
                code::NOT_READY,
                "Plan a drive is not available",
            ));
        };
        let changed_vehicle = config.apply_to(&cat, &checked, &mut setup).is_some();
        debug_assert!(changed_vehicle);
        // The wizard builds its lists and the repaint data from the plan again.
        if let Some(mut liveries) = world.get_resource_mut::<LiveryState>() {
            liveries.vehicle.clear();
            liveries.repaints = None;
        }
        if let Some(mut planner) = world.get_resource_mut::<PlannerState>() {
            planner.built = None;
        }
        store.touch(&config.id, now());
        commit(world, &path, &store)?;
        Ok(json!({
            "id": config.id,
            "name": config.name,
            "context": "wizard",
            "vehicle": config.vehicle,
            "repaint": checked.repaint,
            "setvars": checked.setvars.iter().cloned().collect::<HashMap<_, _>>(),
            "problems": problems,
        }))
    };
    run().into()
}

/// `config.spawn {id, entry?, entry_id?}`: builds a saved configuration as a new session vehicle
/// of the running drive (the same request as `session.spawn`, from the checked configuration).
pub(super) fn spawn(world: &mut World, params: &Value) -> Outcome {
    let mut run = || -> Result<Value, RpcError> {
        crate::control_session::session_ready(world)?;
        let key = str_param(params, "id")?.to_string();
        let entry = crate::control_session::entry_param(params)?;
        let (path, mut store) = open(world)?;
        let config = store.find(&key).cloned().ok_or_else(|| not_found(&key))?;
        let cat = catalogue(world);
        let reps = Repaints::new(world);
        let checked = config.check(&cat, |v| reps.of(v));
        let problems: Vec<Value> = checked
            .problems
            .iter()
            .map(|p| json!({ "code": p.code(), "message": p.to_string() }))
            .collect();
        let job = world
            .get_resource::<WorldLoadJob>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "the world is not loaded"))?;
        let request = crate::bus_config_spawn::spawn_params(
            &config,
            &checked,
            &cat,
            |f| job.bus_file(f),
            entry,
        )
        .map_err(|e| {
            RpcError::new(
                code::FAILED,
                format!("configuration `{}` cannot be spawned: {e}", config.name),
            )
            .with_data(json!({ "problems": problems }))
        })?;
        let name = request.name.clone();
        let id = world
            .resource_mut::<crate::session_vehicles::SessionVehicles>()
            .request_spawn(request);
        store.touch(&config.id, now());
        commit(world, &path, &store)?;
        Ok(
            json!({ "id": id, "queued": true, "configuration": config.id, "name": name, "problems": problems }),
        )
    };
    run().into()
}

pub(super) fn delete(world: &mut World, params: &Value) -> Outcome {
    let mut run = || -> Result<Value, RpcError> {
        let key = str_param(params, "id")?;
        let (path, mut store) = open(world)?;
        let id = store.find(key).map(|c| c.id.clone());
        let Some(id) = id else {
            return Err(not_found(key));
        };
        store.delete(&id);
        commit(world, &path, &store)?;
        Ok(json!({ "ok": true, "id": id }))
    };
    run().into()
}

pub(super) fn update(world: &mut World, params: &Value) -> Outcome {
    let mut run = || -> Result<Value, RpcError> {
        let key = str_param(params, "id")?.to_string();
        let name = opt_str(params, "name")?;
        let notes = opt_str(params, "notes")?;
        let favourite = opt_bool(params, "favourite")?;
        if name.is_none() && notes.is_none() && favourite.is_none() {
            return Err(RpcError::invalid_params(
                "give at least one of `name`, `notes`, `favourite`",
            ));
        }
        let (path, mut store) = open(world)?;
        let id = store
            .find(&key)
            .map(|c| c.id.clone())
            .ok_or_else(|| not_found(&key))?;
        if let Some(n) = name {
            if !store.rename(&id, n) {
                return Err(RpcError::invalid_params(
                    "the name is empty or another configuration has it",
                ));
            }
        }
        if let Some(f) = favourite {
            store.set_favourite(&id, f);
        }
        if let Some(n) = notes {
            if let Some(c) = store.configurations.iter_mut().find(|c| c.id == id) {
                c.notes = n.to_string();
            }
        }
        commit(world, &path, &store)?;
        let cat = catalogue(world);
        let c = store.find(&id).cloned().unwrap_or_default();
        Ok(json!({ "configuration": describe(&c, &cat, None) }))
    };
    run().into()
}

pub(super) fn duplicate(world: &mut World, params: &Value) -> Outcome {
    let mut run = || -> Result<Value, RpcError> {
        let key = str_param(params, "id")?.to_string();
        let name = str_param(params, "name")?.to_string();
        let (path, mut store) = open(world)?;
        if store.find(&key).is_none() {
            return Err(not_found(&key));
        }
        let id = store.duplicate(&key, &name, now()).ok_or_else(|| {
            RpcError::invalid_params("the name is empty or another configuration has it")
        })?;
        commit(world, &path, &store)?;
        let cat = catalogue(world);
        let c = store.find(&id).cloned().unwrap_or_default();
        Ok(json!({ "id": id, "configuration": describe(&c, &cat, None) }))
    };
    run().into()
}
