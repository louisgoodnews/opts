//! `capture.record.start`, `capture.record.stop` and `capture.record.status`.

mod parse;
mod paths;
mod start;

use std::time::Instant;

use bevy::ecs::world::World;
use bevy::prelude::Time;
use opts_cli::protocol::{code, RpcError};
use serde_json::Value;

use crate::control::params::str_param;
use crate::control::state::{ControlRegistry, Outcome};

use super::error::CaptureError;
use super::jobs::{CaptureJobs, CaptureLimits};
use super::recording::{status_json, Recorder};

/// Default downscale of a recording. Measured on a 1920x1013 window: the pure-Rust AV1 encoder
/// keeps about 9 fps at full size, 23 fps at 1/2 and 29 fps at 1/4 (design, Phase C).
pub const DEFAULT_SCALE: f64 = 0.5;

/// Registers the recording commands.
pub fn register(r: &mut ControlRegistry) {
    r.register("capture.record.start", start::start);
    r.register("capture.record.stop", stop);
    r.register("capture.record.status", status);
}

fn id_param(params: &Value) -> Result<String, RpcError> {
    Ok(str_param(params, "recording_id")?.to_string())
}

fn not_found(id: &str) -> RpcError {
    RpcError::new(code::NOT_FOUND, format!("no recording with id `{id}`"))
}

fn stop(world: &mut World, params: &Value) -> Outcome {
    let id = match id_param(params) {
        Ok(i) => i,
        Err(e) => return Outcome::err(e),
    };
    if let Some(done) = world
        .get_resource::<CaptureJobs>()
        .and_then(|j| j.result(&id))
    {
        return Outcome::ok(done.clone());
    }
    match world.get_resource_mut::<Recorder>().and_then(|mut r| {
        r.active
            .as_mut()
            .filter(|a| a.id == id)
            .map(|a| a.request_stop())
    }) {
        Some(()) => {}
        None => return Outcome::err(not_found(&id)),
    }
    let limit = world
        .get_resource::<CaptureLimits>()
        .map_or(std::time::Duration::from_secs(10), |l| l.finish_wait);
    let began = Instant::now();
    Outcome::Wait(Box::new(move |world| {
        if let Some(done) = world
            .get_resource::<CaptureJobs>()
            .and_then(|j| j.result(&id))
        {
            return Some(Ok(done.clone()));
        }
        (began.elapsed() > limit).then(|| {
            Err(CaptureError::Timeout(
                "the encoder did not finish in time; check `capture.record.status`".into(),
            )
            .into())
        })
    }))
}

fn status(world: &mut World, params: &Value) -> Outcome {
    let id = match id_param(params) {
        Ok(i) => i,
        Err(e) => return Outcome::err(e),
    };
    let now = world
        .resource::<Time<bevy::time::Real>>()
        .elapsed_secs_f64();
    if let Some(a) = world
        .get_resource::<Recorder>()
        .and_then(|r| r.active.as_ref())
        .filter(|a| a.id == id)
    {
        return Outcome::ok(status_json(a, now));
    }
    match world
        .get_resource::<CaptureJobs>()
        .and_then(|j| j.result(&id))
    {
        Some(done) => Outcome::ok(done.clone()),
        None => Outcome::err(not_found(&id)),
    }
}

#[cfg(test)]
mod tests;
