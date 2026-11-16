//! Deferred world operations: the queue a system fills and the frame start that runs it.

use super::*;

/// An operation on the `World` that waits for the next frame to start.
type WorldOp = Box<dyn FnOnce(&mut World) + Send + Sync>;

/// Vehicle operations that remove entities, held until the next frame starts ([`defer`]).
#[derive(Resource, Default)]
pub struct DeferredOps(Vec<WorldOp>);

/// Runs `op` at the start of the next frame (schedule `First`), when no system has commands
/// waiting. A despawn queued as an ordinary command is applied in the middle of the frame's
/// command buffers: a command of another system that was queued for the vehicle in the same
/// frame (the passengers' `PassengerFeed`, for one) is applied after it and hits a despawned
/// entity, which panics (Bevy's default error handler).
pub fn defer(commands: &mut Commands, op: impl FnOnce(&mut World) + Send + Sync + 'static) {
    commands.queue(move |world: &mut World| {
        world
            .get_resource_or_init::<DeferredOps>()
            .0
            .push(Box::new(op));
    });
}

/// Runs the deferred operations, then keeps the registry honest (both remove entities, so both
/// run where no command of another system is waiting).
pub(super) fn frame_start(world: &mut World) {
    let ops = std::mem::take(&mut world.get_resource_or_init::<DeferredOps>().0);
    for op in ops {
        op(world);
    }
    world.flush();
    if world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::InGame)
    {
        maintain(world);
    }
}
