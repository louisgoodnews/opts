//! The main-thread side: drains the command channel once per frame and runs the handlers.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::time::{Real, Time};
use opts_cli::commands;
use opts_cli::protocol::{code, Response, RpcError};
use serde_json::Value;

use super::handlers::input::primary_window_or_placeholder;
use super::state::{
    ControlInbox, ControlRegistry, ControlState, Deferred, DeferredCall, Outcome, Pending,
};

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

fn release_due_inputs(world: &mut World) {
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    let due: Vec<_> = {
        let mut state = world.resource_mut::<ControlState>();
        let (due, keep) = std::mem::take(&mut state.releases)
            .into_iter()
            .partition(|r| r.at <= now);
        state.releases = keep;
        due
    };
    let window = primary_window_or_placeholder(world);
    for release in due {
        if let Some((key_code, logical_key)) = release.key {
            world.write_message(KeyboardInput {
                key_code,
                logical_key,
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window,
            });
        }
        if let Some(button) = release.mouse {
            world.write_message(MouseButtonInput {
                button,
                state: ButtonState::Released,
                window,
            });
        }
    }
}

fn take_pending(world: &World) -> Vec<Pending> {
    let inbox = world.resource::<ControlInbox>();
    let Ok(rx) = inbox.0.lock() else {
        return Vec::new();
    };
    std::iter::from_fn(|| rx.try_recv().ok())
        .take(COMMANDS_PER_FRAME)
        .collect()
}

fn poll_deferred(world: &mut World) {
    let calls: Vec<DeferredCall> = {
        let deferred = world.resource::<Deferred>();
        let Ok(mut guard) = deferred.0.lock() else {
            return;
        };
        std::mem::take(&mut *guard)
    };
    let mut still = Vec::new();
    for mut call in calls {
        match (call.waiter)(world) {
            Some(result) => answer(call.id, &call.reply, result),
            None if Instant::now() > call.deadline => {
                answer(
                    call.id,
                    &call.reply,
                    Err(RpcError::new(
                        code::TIMEOUT,
                        "the command did not finish in time",
                    )),
                );
            }
            None => still.push(call),
        }
    }
    if let Ok(mut guard) = world.resource::<Deferred>().0.lock() {
        still.append(&mut guard);
        *guard = still;
    }
}

/// The per-frame system: bookkeeping, input releases, new commands, deferred commands.
pub fn drain_control_commands(world: &mut World) {
    {
        let dt = world.resource::<Time<Real>>().delta_secs();
        let mut state = world.resource_mut::<ControlState>();
        state.frame += 1;
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
