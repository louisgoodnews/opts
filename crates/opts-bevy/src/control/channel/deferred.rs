//! Deferred commands: polled once per frame until they finish, or fail when their deadline passes.

use std::time::Instant;

use bevy::ecs::world::World;
use opts_cli::protocol::{code, RpcError};

use super::super::state::{Deferred, DeferredCall};
use super::answer;

pub(super) fn poll_deferred(world: &mut World) {
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
