//! `batch`: several commands in one round trip, executed in order.

use bevy::ecs::world::World;
use opts_cli::protocol::{RpcError, MAX_BATCH};
use serde_json::{json, Value};

use crate::control::channel::dispatch;
use crate::control::params::opt_bool;
use crate::control::state::{ControlRegistry, Outcome, Waiter};

/// Registers `batch`.
pub fn register(r: &mut ControlRegistry) {
    r.register("batch", batch);
}

fn entry(result: Result<Value, RpcError>) -> Value {
    match result {
        Ok(v) => json!({ "result": v }),
        Err(e) => json!({ "error": e.structured() }),
    }
}

fn parse_commands(params: &Value) -> Result<Vec<(String, Value)>, RpcError> {
    let list = params
        .get("commands")
        .and_then(Value::as_array)
        .ok_or_else(|| RpcError::invalid_params("`commands` must be an array"))?;
    if list.len() > MAX_BATCH {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_BATCH} commands per batch"
        )));
    }
    list.iter()
        .map(|c| {
            let method = c
                .get("method")
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::invalid_params("every command needs a `method`"))?;
            if method == "batch" {
                return Err(RpcError::invalid_params("batches cannot be nested"));
            }
            Ok((
                method.to_string(),
                c.get("params").cloned().unwrap_or(Value::Null),
            ))
        })
        .collect()
}

fn batch(_world: &mut World, params: &Value) -> Outcome {
    let commands = match parse_commands(params) {
        Ok(c) => c,
        Err(e) => return Outcome::err(e),
    };
    let stop_on_error = match opt_bool(params, "stop_on_error") {
        Ok(s) => s.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let mut next = 0usize;
    let mut results: Vec<Value> = Vec::new();
    let mut current: Option<Waiter> = None;
    Outcome::Wait(Box::new(move |world| loop {
        if let Some(waiter) = current.as_mut() {
            let result = waiter(world)?;
            current = None;
            let failed = result.is_err();
            results.push(entry(result));
            if failed && stop_on_error {
                return Some(Ok(json!({ "results": results })));
            }
        }
        let Some((method, params)) = commands.get(next) else {
            return Some(Ok(json!({ "results": results })));
        };
        next += 1;
        match dispatch(world, method, params) {
            Outcome::Done(result) => {
                let failed = result.is_err();
                results.push(entry(result));
                if failed && stop_on_error {
                    return Some(Ok(json!({ "results": results })));
                }
            }
            Outcome::Wait(w) => {
                current = Some(w);
                return None;
            }
        }
    }))
}
