//! The main-thread side: drains the command channel once per frame and runs the handlers.

mod deferred;
mod inputs;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::time::{Real, Time};
use opts_cli::commands;
use opts_cli::protocol::{code, Response, RpcError};
use serde_json::Value;

use super::state::{ControlRegistry, ControlState, Deferred, DeferredCall, Outcome};

use deferred::poll_deferred;
use inputs::{release_due_inputs, take_pending};

/// Commands executed per frame; the rest wait in the queue.
const COMMANDS_PER_FRAME: usize = 64;
/// A deferred command that is still unfinished after this long fails with `1005`.
const DEFERRED_LIMIT: Duration = Duration::from_secs(120);

/// Runs `method` through the registry. Unknown methods answer `-32601`, specified but
/// unimplemented ones `1007`, and a panicking handler `-32603` (the app keeps running).
pub fn dispatch(world: &mut World, method: &str, params: &Value) -> Outcome {
    let handler = world.resource::<ControlRegistry>().get(method);
    let Some(handler) = handler else {
        let error = if commands::find(method).is_some() {
            RpcError::new(
                code::UNSUPPORTED,
                format!("`{method}` is not implemented in this build"),
            )
        } else {
            RpcError::new(code::METHOD_NOT_FOUND, format!("unknown method `{method}`"))
        };
        return Outcome::err(error);
    };
    match catch_unwind(AssertUnwindSafe(|| handler(world, params))) {
        Ok(outcome) => outcome,
        Err(_) => {
            tracing::error!(target: "opts_bevy::control", method, "control handler panicked");
            Outcome::err(RpcError::new(
                code::INTERNAL_ERROR,
                format!("`{method}` failed unexpectedly"),
            ))
        }
    }
}

fn answer(
    pending_id: Value,
    reply: &std::sync::mpsc::Sender<Response>,
    result: Result<Value, RpcError>,
) {
    let response = match result {
        Ok(v) => Response::ok(pending_id, v),
        Err(e) => Response::err(pending_id, e),
    };
    let _ = reply.send(response);
}

/// The per-frame system: bookkeeping, input releases, new commands, deferred commands.
pub fn drain_control_commands(world: &mut World) {
    {
        let (dt, now) = {
            let t = world.resource::<Time<Real>>();
            (t.delta_secs(), t.elapsed_secs_f64())
        };
        let mut state = world.resource_mut::<ControlState>();
        state.frame += 1;
        state.frame_log.push_back((now, dt * 1000.0));
        while state
            .frame_log
            .front()
            .is_some_and(|(at, _)| now - at > 10.0)
        {
            state.frame_log.pop_front();
        }
        if dt > 0.0 {
            state.fps = if state.fps <= 0.0 {
                1.0 / dt
            } else {
                state.fps + (1.0 / dt - state.fps) * 0.1
            };
        }
    }
    release_due_inputs(world);
    for pending in take_pending(world) {
        if pending.flags.cancelled.load(Ordering::SeqCst) {
            continue;
        }
        pending.flags.started.store(true, Ordering::SeqCst);
        let id = pending.request.id.clone().unwrap_or(Value::Null);
        match dispatch(world, &pending.request.method, &pending.request.params) {
            Outcome::Done(result) => answer(id, &pending.reply, result),
            Outcome::Wait(waiter) => {
                let call = DeferredCall {
                    id,
                    reply: pending.reply,
                    waiter,
                    deadline: Instant::now() + DEFERRED_LIMIT,
                };
                if let Ok(mut guard) = world.resource::<Deferred>().0.lock() {
                    guard.push(call);
                }
            }
        }
    }
    poll_deferred(world);
}
