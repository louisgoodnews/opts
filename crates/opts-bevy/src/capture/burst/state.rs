//! The burst record: one requested frame and the job state shared by the burst's stages.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use bevy::prelude::Image;
use bevy::tasks::Task;
use serde_json::{Map, Value};

use super::super::backend::CaptureBackend;
use super::super::error::CaptureError;
use super::super::jobs::JobGuard;
use super::super::screenshot::StillFormat;

pub(super) struct Frame {
    pub(super) index: u64,
    pub(super) sim_time: f64,
    pub(super) frame: u64,
    pub(super) wall_ms: u64,
    pub(super) requested: Instant,
    pub(super) path: Option<PathBuf>,
    pub(super) resolved: bool,
}

pub(super) struct Burst {
    pub(super) _guard: JobGuard,
    pub(super) id: String,
    pub(super) dir: PathBuf,
    pub(super) format: StillFormat,
    pub(super) quality: u8,
    pub(super) count: u64,
    pub(super) interval: f64,
    pub(super) started: f64,
    pub(super) next_due: f64,
    pub(super) backend: CaptureBackend,
    pub(super) tx: mpsc::Sender<(u64, Image)>,
    pub(super) rx: mpsc::Receiver<(u64, Image)>,
    pub(super) frames: Vec<Frame>,
    pub(super) tasks: Vec<(u64, Task<Result<(), CaptureError>>)>,
    pub(super) dropped: u64,
    pub(super) last_error: Option<String>,
    pub(super) base: Option<Map<String, Value>>,
}
