//! `capture.record.start`: validates the request, reserves a job and launches the encoder.

use bevy::ecs::world::World;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use crate::control::params::opt_str;
use crate::control::state::Outcome;

use super::super::backend::CaptureBackend;
use super::super::jobs::{CaptureJobs, CaptureLimits};
use super::super::metadata::{self, Subject};
use super::super::recording::launch;
use super::super::storage::CaptureState;
use super::super::worker::WorkerPlan;
use super::parse::parse;
use super::paths::{check_free_disk, output_paths};

pub(super) fn start(world: &mut World, params: &Value) -> Outcome {
    match try_start(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn try_start(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let limits = world
        .get_resource::<CaptureLimits>()
        .cloned()
        .unwrap_or_default();
    let plan = parse(params, &limits)?;
    let name = opt_str(params, "name")?.map(str::to_string);
    let backend = world
        .get_resource::<CaptureBackend>()
        .copied()
        .unwrap_or_default();
    if let Some(e) = (backend.ready)(world) {
        return Err(e.into());
    }
    let (output, sidecar, session) = {
        let mut state = world.get_resource_or_insert_with(CaptureState::default);
        let root = state.ensure_root()?;
        check_free_disk(&state, &root, &limits)?;
        let (output, sidecar) = output_paths(
            &root,
            name.unwrap_or_else(|| state.next_stem()),
            plan.format,
        )?;
        (output, sidecar, state.session_id.clone())
    };
    let (id, guard) = world
        .get_resource_or_insert_with(CaptureJobs::default)
        .begin("recording", &session, &limits)?;
    let file = output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let base = metadata::collect(
        world,
        Subject {
            kind: "recording",
            file: &file,
            width: 0,
            height: 0,
            format: plan.format.name(),
        },
    );
    let result = json!({
        "recording_id": id,
        "path": output.display().to_string(),
        "format": plan.format.name(),
        "fps": plan.fps,
        "effective_limits": {
            "duration_ms": plan.duration_ms,
            "max_size_mb": plan.max_bytes / (1024 * 1024),
            "min_free_disk_mb": limits.min_free_disk_mb,
        },
    });
    let worker_plan = WorkerPlan {
        output,
        sidecar,
        format: plan.format,
        fps: plan.fps,
        quality: plan.quality,
        scale: plan.scale,
        base,
    };
    launch(world, id, guard, &plan, worker_plan, &limits);
    Ok(result)
}
