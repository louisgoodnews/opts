//! `capture.record.start`, `capture.record.stop` and `capture.record.status`.

use std::time::Instant;

use bevy::ecs::world::World;
use bevy::prelude::Time;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::control::params::{opt_f64, opt_str, opt_u64, str_param};
use crate::control::state::{ControlRegistry, Outcome};

use super::backend::CaptureBackend;
use super::error::CaptureError;
use super::jobs::{CaptureJobs, CaptureLimits};
use super::metadata::{self, Subject};
use super::recording::{launch, status_json, RecFormat, Recorder, StartPlan};
use super::storage::{self, CaptureState};
use super::worker::WorkerPlan;

/// Default downscale of a recording. Measured on a 1920x1013 window: the pure-Rust AV1 encoder
/// keeps about 9 fps at full size, 23 fps at 1/2 and 29 fps at 1/4 (design, Phase C).
pub const DEFAULT_SCALE: f64 = 0.5;

/// Registers the recording commands.
pub fn register(r: &mut ControlRegistry) {
    r.register("capture.record.start", start);
    r.register("capture.record.stop", stop);
    r.register("capture.record.status", status);
}

fn parse(params: &Value, limits: &CaptureLimits) -> Result<StartPlan, RpcError> {
    let format = match opt_str(params, "format")? {
        None | Some("av1") => RecFormat::Av1,
        Some("png_sequence") => RecFormat::PngSequence,
        Some("h264") => {
            return Err(RpcError::invalid_params(
                "format `h264` needs a build with the optional `capture-h264` feature, which this build does not have; use `av1` (remux with ffmpeg for MP4)",
            ))
        }
        Some(other) => {
            return Err(RpcError::invalid_params(format!(
                "unknown format `{other}`; valid: av1, png_sequence"
            )))
        }
    };
    let fps = opt_u64(params, "fps")?.unwrap_or(30);
    if !(1..=60).contains(&fps) {
        return Err(RpcError::invalid_params("`fps` must be 1..=60"));
    }
    let duration_ms = opt_u64(params, "duration_ms")?.unwrap_or(limits.default_duration_ms);
    if !(1..=limits.max_duration_ms).contains(&duration_ms) {
        return Err(RpcError::invalid_params(format!(
            "`duration_ms` must be 1..={} (30 minutes)",
            limits.max_duration_ms
        )));
    }
    let size_mb = opt_u64(params, "max_size_mb")?.unwrap_or(limits.default_size_mb);
    if !(1..=limits.max_size_mb).contains(&size_mb) {
        return Err(RpcError::invalid_params(format!(
            "`max_size_mb` must be 1..={}",
            limits.max_size_mb
        )));
    }
    let quality = match opt_u64(params, "quality")? {
        Some(q @ 1..=100) => q as u8,
        Some(_) => return Err(RpcError::invalid_params("`quality` must be 1..=100")),
        None => 70,
    };
    let scale = opt_f64(params, "scale")?.unwrap_or(DEFAULT_SCALE);
    if !(0.25..=1.0).contains(&scale) {
        return Err(RpcError::invalid_params("`scale` must be 0.25..=1.0"));
    }
    Ok(StartPlan {
        format,
        fps: fps as u32,
        duration_ms,
        max_bytes: size_mb * 1024 * 1024,
        quality,
        scale: scale as f32,
    })
}

fn start(world: &mut World, params: &Value) -> Outcome {
    match try_start(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

/// Refuses to start when the disk has less free space than the recording limits require.
fn check_free_disk(
    state: &CaptureState,
    root: &std::path::Path,
    limits: &CaptureLimits,
) -> Result<(), RpcError> {
    if let Some(free) = (state.free_space)(root) {
        let need = limits.min_free_disk_mb * 1024 * 1024;
        if free < need {
            return Err(CaptureError::Failed(format!(
                "not enough free disk space for a recording: {} MiB free, at least {} MiB required",
                free / (1024 * 1024),
                limits.min_free_disk_mb
            ))
            .into());
        }
    }
    Ok(())
}

/// The output path and the sidecar path of a recording named `stem`.
fn output_paths(
    root: &std::path::Path,
    stem: String,
    format: RecFormat,
) -> Result<(std::path::PathBuf, std::path::PathBuf), RpcError> {
    let paths = match format {
        RecFormat::Av1 => {
            let file = if stem.ends_with(".ivf") {
                stem
            } else {
                format!("{stem}.ivf")
            };
            let out = storage::resolve_name(root, &file)?;
            let side = super::screenshot::sidecar_path(&out);
            (out, side)
        }
        RecFormat::PngSequence => {
            let out = storage::resolve_dir(root, &format!("{stem}_rec"))?;
            let side = out.join("manifest.json");
            (out, side)
        }
    };
    Ok(paths)
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
