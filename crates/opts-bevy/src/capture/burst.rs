//! `capture.burst`: N frames at a fixed interval, written as a PNG (or JPEG) sequence.

mod collect;
mod options;
mod result;
mod start;
mod state;

pub use options::{MAX_COUNT, MAX_INTERVAL_MS, MAX_TOTAL_MS, MIN_INTERVAL_MS};

use bevy::ecs::world::World;
use bevy::prelude::Time;
use serde_json::Value;

use crate::control::state::{Outcome, Waiter};

use self::start::start;
use self::state::Burst;

/// Registers `capture.burst`.
pub fn register(r: &mut crate::control::state::ControlRegistry) {
    r.register("capture.burst", handle);
}

fn handle(world: &mut World, params: &Value) -> Outcome {
    match start(world, params) {
        Ok(burst) => Outcome::Wait(waiter(burst)),
        Err(e) => Outcome::err(e),
    }
}

fn waiter(mut burst: Burst) -> Waiter {
    Box::new(move |world| {
        let now = world
            .resource::<Time<bevy::time::Real>>()
            .elapsed_secs_f64();
        if let Err(e) = burst.request_due(world, now) {
            return Some(Err(e));
        }
        burst.collect();
        let overdue = now - burst.started > burst.interval * burst.count as f64 + 30.0;
        if burst.finished() || overdue {
            let pending: Vec<u64> = burst
                .frames
                .iter()
                .filter(|f| !f.resolved)
                .map(|f| f.index)
                .collect();
            for i in pending {
                burst.mark_lost(i, "the burst ran out of time");
            }
            return Some(burst.result());
        }
        None
    })
}
