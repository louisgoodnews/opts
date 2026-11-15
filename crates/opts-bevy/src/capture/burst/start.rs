//! The burst start: directory, job slot and metadata of the request.

use std::sync::mpsc;

use bevy::ecs::world::World;
use bevy::prelude::Time;
use opts_cli::protocol::RpcError;
use serde_json::Value;

use crate::control::params::u64_param;
use super::super::jobs::{CaptureJobs, CaptureLimits};
use super::super::metadata::{self, Subject};
use super::super::storage::{self, CaptureState};
use super::super::backend::CaptureBackend;
use super::options::{check_limits, still_options};
use super::state::Burst;

pub(super) fn start(world: &mut World, params: &Value) -> Result<Burst, RpcError> {
    let count = u64_param(params, "count")?;
    let interval_ms = u64_param(params, "interval_ms")?;
    check_limits(count, interval_ms)?;
    let (format, quality, with_meta, dir_name) = still_options(params)?;
    let backend = world
        .get_resource::<CaptureBackend>()
        .copied()
        .unwrap_or_default();
    if let Some(e) = (backend.ready)(world) {
        return Err(e.into());
    }
    let limits = world
        .get_resource::<CaptureLimits>()
        .cloned()
        .unwrap_or_default();
    let (dir, session) = {
        let mut state = world.get_resource_or_insert_with(CaptureState::default);
        let root = state.ensure_root()?;
        let rel = dir_name.unwrap_or_else(|| format!("{}_burst", state.next_stem()));
        (storage::resolve_dir(&root, &rel)?, state.session_id.clone())
    };
    let (id, guard) = world
        .get_resource_or_insert_with(CaptureJobs::default)
        .begin("burst", &session, &limits)?;
    let base = with_meta.then(|| {
        metadata::collect(
            world,
            Subject {
                kind: "burst",
                file: &dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                width: 0,
                height: 0,
                format: format.name(),
            },
        )
    });
    let now = world
        .resource::<Time<bevy::time::Real>>()
        .elapsed_secs_f64();
    let (tx, rx) = mpsc::channel();
    Ok(Burst {
        _guard: guard,
        id,
        dir,
        format,
        quality,
        count,
        interval: interval_ms as f64 / 1000.0,
        started: now,
        next_due: now,
        backend,
        tx,
        rx,
        frames: Vec::new(),
        tasks: Vec::new(),
        dropped: 0,
        last_error: None,
        base,
    })
}
