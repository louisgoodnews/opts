//! Editing the saved configurations (`config.delete`, `config.update`, `config.duplicate`).

use super::*;

pub(in crate::control_configs) fn delete(world: &mut World, params: &Value) -> Outcome {
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

pub(in crate::control_configs) fn update(world: &mut World, params: &Value) -> Outcome {
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

pub(in crate::control_configs) fn duplicate(world: &mut World, params: &Value) -> Outcome {
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
