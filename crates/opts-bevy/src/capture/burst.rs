//! `capture.burst`: N frames at a fixed interval, written as a PNG (or JPEG) sequence.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::prelude::{Image, Time, Virtual};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use opts_cli::protocol::RpcError;
use serde_json::{json, Map, Value};

use crate::control::params::{opt_bool, opt_str, opt_u64, u64_param};
use crate::control::state::{ControlState, Outcome, Waiter};

use super::backend::CaptureBackend;
use super::error::CaptureError;
use super::jobs::{CaptureJobs, CaptureLimits, JobGuard};
use super::metadata::{self, Subject};
use super::screenshot::{encode, StillFormat, DEFAULT_JPEG_QUALITY};
use super::storage::{self, CaptureState};

/// Most frames in one burst.
pub const MAX_COUNT: u64 = 300;
/// Shortest interval (one frame at 60 Hz).
pub const MIN_INTERVAL_MS: u64 = 16;
/// Longest interval.
pub const MAX_INTERVAL_MS: u64 = 60_000;
/// A burst answers within the control server's 120 s limit for deferred replies.
pub const MAX_TOTAL_MS: u64 = 90_000;
/// A requested frame that is not delivered within this time counts as dropped.
const FRAME_WAIT: Duration = Duration::from_secs(2);

struct Frame {
    index: u64,
    sim_time: f64,
    frame: u64,
    wall_ms: u64,
    requested: Instant,
    path: Option<PathBuf>,
    resolved: bool,
}

struct Burst {
    _guard: JobGuard,
    id: String,
    dir: PathBuf,
    format: StillFormat,
    quality: u8,
    count: u64,
    interval: f64,
    started: f64,
    next_due: f64,
    backend: CaptureBackend,
    tx: mpsc::Sender<(u64, Image)>,
    rx: mpsc::Receiver<(u64, Image)>,
    frames: Vec<Frame>,
    tasks: Vec<(u64, Task<Result<(), CaptureError>>)>,
    dropped: u64,
    last_error: Option<String>,
    base: Option<Map<String, Value>>,
}

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

/// The count, the interval and their product must stay inside the limits.
fn check_limits(count: u64, interval_ms: u64) -> Result<(), RpcError> {
    if !(1..=MAX_COUNT).contains(&count) {
        return Err(RpcError::invalid_params(format!(
            "`count` must be 1..={MAX_COUNT}"
        )));
    }
    if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&interval_ms) {
        return Err(RpcError::invalid_params(format!(
            "`interval_ms` must be {MIN_INTERVAL_MS}..={MAX_INTERVAL_MS}"
        )));
    }
    if count * interval_ms > MAX_TOTAL_MS {
        return Err(RpcError::invalid_params(format!(
            "`count` x `interval_ms` may not exceed {MAX_TOTAL_MS} ms (the reply must arrive within the control server's deferred-reply limit)"
        )));
    }
    Ok(())
}

/// The format, JPEG quality, metadata flag and folder name of the request.
fn still_options(params: &Value) -> Result<(StillFormat, u8, bool, Option<String>), RpcError> {
    let format = match opt_str(params, "format")? {
        Some(f) => StillFormat::parse(f)?,
        None => StillFormat::Png,
    };
    let quality = match opt_u64(params, "quality")? {
        Some(q @ 1..=100) => q as u8,
        Some(_) => return Err(RpcError::invalid_params("`quality` must be 1..=100")),
        None => DEFAULT_JPEG_QUALITY,
    };
    let with_meta = opt_bool(params, "metadata")?.unwrap_or(true);
    let dir_name = opt_str(params, "dir")?.map(str::to_string);
    Ok((format, quality, with_meta, dir_name))
}

fn start(world: &mut World, params: &Value) -> Result<Burst, RpcError> {
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

impl Burst {
    fn request_due(&mut self, world: &mut World, now: f64) -> Result<(), RpcError> {
        if self.frames.len() as u64 >= self.count || now < self.next_due {
            return Ok(());
        }
        let index = self.frames.len() as u64;
        self.next_due = (self.next_due + self.interval).max(now - self.interval);
        let sim_time = world
            .get_resource::<Time<Virtual>>()
            .map_or(0.0, |t| (t.elapsed_secs_f64() * 1000.0).round() / 1000.0);
        let frame = world.get_resource::<ControlState>().map_or(0, |c| c.frame);
        self.frames.push(Frame {
            index,
            sim_time,
            frame,
            wall_ms: ((now - self.started) * 1000.0) as u64,
            requested: Instant::now(),
            path: None,
            resolved: false,
        });
        let tx = self.tx.clone();
        let sink = Box::new(move |image| {
            let _ = tx.send((index, image));
        });
        if let Err(e) = (self.backend.request)(world, sink) {
            if index == 0 {
                return Err(e.into());
            }
            self.mark_lost(index, &e.to_string());
        }
        Ok(())
    }

    fn mark_lost(&mut self, index: u64, why: &str) {
        let f = &mut self.frames[index as usize];
        if !f.resolved {
            f.resolved = true;
            self.dropped += 1;
            self.last_error.get_or_insert_with(|| why.to_string());
        }
    }

    fn collect(&mut self) {
        while let Ok((index, image)) = self.rx.try_recv() {
            if self.frames[index as usize].resolved {
                continue;
            }
            let path = self
                .dir
                .join(format!("{:04}.{}", index + 1, self.format.ext()));
            self.frames[index as usize].path = Some(path.clone());
            let (format, quality) = (self.format, self.quality);
            let pool = AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
            self.tasks.push((
                index,
                pool.spawn(async move {
                    let bytes = encode(image, format, quality)?;
                    storage::write_atomic(&path, &bytes)
                }),
            ));
        }
        let mut i = 0;
        while i < self.tasks.len() {
            if let Some(result) = block_on(future::poll_once(&mut self.tasks[i].1)) {
                let (index, _) = self.tasks.swap_remove(i);
                let f = &mut self.frames[index as usize];
                f.resolved = true;
                if let Err(e) = result {
                    f.path = None;
                    self.dropped += 1;
                    self.last_error.get_or_insert_with(|| e.to_string());
                }
            } else {
                i += 1;
            }
        }
        let lost: Vec<u64> = self
            .frames
            .iter()
            .filter(|f| !f.resolved && f.path.is_none() && f.requested.elapsed() > FRAME_WAIT)
            .map(|f| f.index)
            .collect();
        for index in lost {
            self.mark_lost(
                index,
                "the renderer did not deliver a frame in time (is the window occluded?)",
            );
        }
    }

    fn finished(&self) -> bool {
        self.frames.len() as u64 == self.count && self.frames.iter().all(|f| f.resolved)
    }

    fn result(&mut self) -> Result<Value, RpcError> {
        let ok: Vec<&Frame> = self.frames.iter().filter(|f| f.path.is_some()).collect();
        if ok.is_empty() {
            return Err(CaptureError::Failed(format!(
                "no frame was captured: {}",
                self.last_error.as_deref().unwrap_or("unknown cause")
            ))
            .into());
        }
        let frames: Vec<Value> = ok
            .iter()
            .map(|f| {
                json!({
                    "index": f.index,
                    "path": f.path.as_ref().map(|p| p.display().to_string()),
                    "sim_time": f.sim_time,
                    "frame": f.frame,
                    "wall_ms": f.wall_ms,
                })
            })
            .collect();
        let paths: Vec<Value> = frames.iter().map(|f| f["path"].clone()).collect();
        let mut manifest = self.base.take().unwrap_or_default();
        manifest.insert("id".into(), json!(self.id));
        manifest.insert("count".into(), json!(self.count));
        manifest.insert("interval_ms".into(), json!((self.interval * 1000.0) as u64));
        manifest.insert("dropped".into(), json!(self.dropped));
        manifest.insert("frames".into(), json!(frames));
        let manifest_path = self.dir.join("manifest.json");
        let text = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| CaptureError::Failed(format!("manifest serialisation failed: {e}")))?;
        storage::write_atomic(&manifest_path, &text)?;
        Ok(json!({
            "dir": self.dir.display().to_string(),
            "paths": paths,
            "frames": frames,
            "dropped": self.dropped,
            "manifest": manifest_path.display().to_string(),
        }))
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
