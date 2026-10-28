//! Continuous capture: the encoder worker thread, shared counters and the per-frame driver that
//! enforces the limits and the degradation policy (design A5).
//!
//! Nothing here waits for the GPU or the disk on the main thread: frames arrive through the
//! renderer's asynchronous readback, are handed to a bounded queue with `try_send`, and the
//! worker thread converts, encodes and writes them.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::prelude::{Image, Resource, Time, Virtual};
use serde_json::{json, Value};

use crate::control::state::ControlState;

use super::backend::CaptureBackend;
use super::encoder::FrameInfo;
use super::error::CaptureError;
use super::jobs::{CaptureJobs, CaptureLimits, Finished, JobGuard};
use super::storage::CaptureState;
use super::worker::{set_error, worker, Msg, Shared, WorkerPlan, FAILED, FINISHED};

mod driver;
pub use driver::*;

/// Recording formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecFormat {
    /// AV1 in an IVF container.
    Av1,
    /// One PNG per frame in a directory.
    PngSequence,
}

impl RecFormat {
    /// The `format` parameter value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Av1 => "av1",
            Self::PngSequence => "png_sequence",
        }
    }
}

/// A running recording, owned by [`Recorder`].
pub struct Active {
    /// Recording id.
    pub id: String,
    /// Output file or directory.
    pub output: PathBuf,
    /// Format.
    pub format: RecFormat,
    /// Requested frames per second.
    pub fps: u32,
    /// Duration limit in milliseconds.
    pub duration_ms: u64,
    /// Size limit in bytes.
    pub max_bytes: u64,
    /// Shared counters.
    pub shared: Arc<Shared>,
    pub(super) started: f64,
    pub(super) next_due: f64,
    pub(super) requested: u64,
    pub(super) tx: Option<SyncSender<Msg>>,
    pub(super) handle: Option<JoinHandle<()>>,
    pub(super) stopped_by: Option<&'static str>,
    pub(super) stopped_after_ms: u64,
    pub(super) low_fps_since: Option<f64>,
    pub(super) last_disk_check: f64,
    pub(super) stop_request: bool,
    pub(super) forced_reason: Option<&'static str>,
    _guard: JobGuard,
}

impl Active {
    /// Whether frames are still being captured.
    #[must_use]
    pub fn capturing(&self) -> bool {
        self.tx.is_some()
    }

    /// Asks the recording to stop at the next frame.
    pub fn request_stop(&mut self) {
        self.stop_request = true;
    }
}

/// The active recording, if any.
#[derive(Resource, Default)]
pub struct Recorder {
    /// The recording in progress or finishing.
    pub active: Option<Active>,
}

impl Drop for Recorder {
    /// Closes a running recording when the app exits: the queue is closed and the worker gets
    /// three seconds to finish the file.
    fn drop(&mut self) {
        let Some(mut a) = self.active.take() else {
            return;
        };
        a.tx = None;
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until && a.shared.state.load(Ordering::Acquire) < FINISHED {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Validated start parameters.
pub struct StartPlan {
    /// Format.
    pub format: RecFormat,
    /// Frames per second.
    pub fps: u32,
    /// Duration limit.
    pub duration_ms: u64,
    /// Size limit in bytes.
    pub max_bytes: u64,
    /// Encoder quality.
    pub quality: u8,
    /// Downscale factor.
    pub scale: f32,
}

/// Starts the worker and registers the recording; the caller has validated everything.
pub fn launch(
    world: &mut World,
    id: String,
    guard: JobGuard,
    plan: &StartPlan,
    worker_plan: WorkerPlan,
    limits: &CaptureLimits,
) {
    let shared = Shared::new();
    let (tx, rx) = mpsc::sync_channel::<Msg>(limits.queue_depth);
    let (w_shared, output) = (shared.clone(), worker_plan.output.clone());
    let handle = std::thread::Builder::new()
        .name("opts-capture-encoder".into())
        .spawn(move || {
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                worker(&rx, &worker_plan, &w_shared);
            }));
            if caught.is_err() {
                set_error(
                    &w_shared,
                    &CaptureError::Failed("the encoder thread panicked".into()),
                );
                w_shared.state.store(FAILED, Ordering::Release);
            }
        })
        .ok();
    let now = world
        .resource::<Time<bevy::time::Real>>()
        .elapsed_secs_f64();
    let active = Active {
        id,
        output,
        format: plan.format,
        fps: plan.fps,
        duration_ms: plan.duration_ms,
        max_bytes: plan.max_bytes,
        shared,
        started: now,
        next_due: now,
        requested: 0,
        tx: handle.is_some().then_some(tx),
        handle,
        stopped_by: None,
        stopped_after_ms: 0,
        low_fps_since: None,
        last_disk_check: now,
        stop_request: false,
        forced_reason: None,
        _guard: guard,
    };
    world.resource_mut::<Recorder>().active = Some(active);
}
