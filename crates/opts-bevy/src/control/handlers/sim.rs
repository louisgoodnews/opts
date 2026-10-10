//! `sim.pause`, `sim.resume`, `sim.step`, `sim.time_scale`.

use bevy::ecs::world::World;
use bevy::time::{Time, Virtual};
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use crate::control::params::{f64_param, u64_param};
use crate::control::state::{ControlRegistry, ControlState, Outcome};

/// Most frames one `sim.step` may advance.
const MAX_STEP_FRAMES: u64 = 100_000;

/// Registers the simulation methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("sim.pause", pause);
    r.register("sim.resume", resume);
    r.register("sim.step", step);
    r.register("sim.time_scale", time_scale);
}

fn pause(world: &mut World, _: &Value) -> Outcome {
    world.resource_mut::<Time<Virtual>>().pause();
    Outcome::ok(json!({ "paused": true }))
}

fn resume(world: &mut World, _: &Value) -> Outcome {
    world.resource_mut::<Time<Virtual>>().unpause();
    Outcome::ok(json!({ "paused": false }))
}

/// Runs `frames` frames of simulation time, then pauses again; answers after the last frame.
fn step(world: &mut World, params: &Value) -> Outcome {
    let frames = match u64_param(params, "frames") {
        Ok(f) if (1..=MAX_STEP_FRAMES).contains(&f) => f,
        Ok(_) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "`frames` must be 1..={MAX_STEP_FRAMES}"
            )))
        }
        Err(e) => return Outcome::err(e),
    };
    let start = world.resource::<ControlState>().frame;
    world.resource_mut::<Time<Virtual>>().unpause();
    Outcome::Wait(Box::new(move |world| {
        let done = world.resource::<ControlState>().frame.saturating_sub(start);
        if done < frames {
            return None;
        }
        let mut time = world.resource_mut::<Time<Virtual>>();
        time.pause();
        Some(Ok(
            json!({ "stepped": frames, "sim_time": time.elapsed_secs_f64() }),
        ))
    }))
}

fn time_scale(world: &mut World, params: &Value) -> Outcome {
    let factor = match f64_param(params, "factor") {
        Ok(f) if f > 0.0 && f <= 20.0 => f,
        Ok(_) => {
            return Outcome::err(RpcError::invalid_params(
                "`factor` must be greater than 0 and at most 20",
            ))
        }
        Err(e) => return Outcome::err(e),
    };
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed_f64(factor);
    Outcome::ok(json!({ "time_scale": factor }))
}
