//! The encoder worker thread and the counters it shares with the main thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::prelude::Image;
use serde_json::{json, Map, Value};

use super::av1::Av1Ivf;
use super::encoder::{Encoder, FrameInfo, PngSequence, RgbFrame};
use super::error::CaptureError;
use super::recording::RecFormat;
use super::storage;

/// Worker lifecycle, stored in [`Shared::state`].
pub const RECORDING: u8 = 0;
/// The worker is flushing and closing the output.
pub const FINISHING: u8 = 1;
/// The output is closed.
pub const FINISHED: u8 = 2;
/// The encoder failed; the partial output is kept.
pub const FAILED: u8 = 3;

/// Counters shared between the main thread and the worker.
pub struct Shared {
    /// Frames encoded.
    pub frames: AtomicU64,
    /// Bytes written so far.
    pub bytes: AtomicU64,
    /// Capture ticks that produced no frame.
    pub dropped: AtomicU64,
    /// Frames requested from the renderer and not yet delivered.
    pub in_flight: AtomicUsize,
    /// See the lifecycle constants.
    pub state: AtomicU8,
    /// Whether the capture rate is currently halved.
    pub degraded: AtomicBool,
    /// Total microseconds the worker spent converting frames (RGB, scaling).
    pub convert_us: AtomicU64,
    /// Total microseconds the worker spent in the encoder.
    pub encode_us: AtomicU64,
    /// Total microseconds between requesting a frame and its delivery.
    pub delivery_us: AtomicU64,
    /// Frames delivered by the renderer.
    pub delivered: AtomicU64,
    /// Milliseconds since the start at which the last frame was delivered.
    pub last_delivery_ms: AtomicU64,
    /// Milliseconds since the start at which the queue became full (0 = not full).
    pub full_since_ms: AtomicU64,
    /// The encoder error, if any.
    pub error: Mutex<Option<String>>,
    /// Start of the recording.
    pub started: Instant,
}

impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            frames: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            in_flight: AtomicUsize::new(0),
            state: AtomicU8::new(RECORDING),
            degraded: AtomicBool::new(false),
            convert_us: AtomicU64::new(0),
            encode_us: AtomicU64::new(0),
            delivery_us: AtomicU64::new(0),
            delivered: AtomicU64::new(0),
            last_delivery_ms: AtomicU64::new(0),
            full_since_ms: AtomicU64::new(0),
            error: Mutex::new(None),
            started: Instant::now(),
        })
    }

    pub fn ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
}

pub enum Msg {
    /// A captured frame and its identity.
    Frame(Image, FrameInfo),
}

/// Everything the worker needs to create its encoder and describe the result.
pub struct WorkerPlan {
    /// Output file (AV1) or directory (PNG sequence).
    pub output: PathBuf,
    /// Sidecar path.
    pub sidecar: PathBuf,
    /// Format.
    pub format: RecFormat,
    /// Frames per second.
    pub fps: u32,
    /// Encoder quality.
    pub quality: u8,
    /// Downscale factor.
    pub scale: f32,
    /// Metadata collected at the start.
    pub base: Map<String, Value>,
}

fn make_encoder(plan: &WorkerPlan, frame: &RgbFrame) -> Result<Box<dyn Encoder>, CaptureError> {
    Ok(match plan.format {
        RecFormat::Av1 => Box::new(Av1Ivf::create(
            &plan.output,
            frame.width,
            frame.height,
            plan.fps,
            plan.quality,
        )?),
        RecFormat::PngSequence => Box::new(PngSequence::create(&plan.output)?),
    })
}

pub fn set_error(shared: &Shared, e: &CaptureError) {
    if let Ok(mut g) = shared.error.lock() {
        g.get_or_insert_with(|| e.to_string());
    }
}

/// Writes the sidecar file of the recording; returns whether that failed.
fn write_sidecar(
    plan: &WorkerPlan,
    shared: &Shared,
    size: (u32, u32),
    infos: &[FrameInfo],
) -> bool {
    let mut failed = false;
    let mut meta = plan.base.clone();
    meta.insert("format".into(), json!(plan.format.name()));
    meta.insert("fps".into(), json!(plan.fps));
    meta.insert("width".into(), json!(size.0));
    meta.insert("height".into(), json!(size.1));
    meta.insert("frame_count".into(), json!(infos.len()));
    meta.insert(
        "dropped".into(),
        json!(shared.dropped.load(Ordering::Relaxed)),
    );
    meta.insert("bytes".into(), json!(shared.bytes.load(Ordering::Relaxed)));
    if let Some(err) = shared.error.lock().ok().and_then(|g| g.clone()) {
        meta.insert("error".into(), json!(err));
    }
    meta.insert("frames".into(), json!(infos));
    match serde_json::to_vec_pretty(&meta) {
        Ok(text) => {
            if let Err(e) = storage::write_atomic(&plan.sidecar, &text) {
                set_error(shared, &e);
                failed = true;
            }
        }
        Err(e) => {
            set_error(
                shared,
                &CaptureError::Failed(format!("sidecar serialisation failed: {e}")),
            );
            failed = true;
        }
    }
    failed
}

pub fn worker(rx: &Receiver<Msg>, plan: &WorkerPlan, shared: &Shared) {
    let mut encoder: Option<Box<dyn Encoder>> = None;
    let mut size = (0, 0);
    let mut infos: Vec<FrameInfo> = Vec::new();
    let mut failed = false;
    while let Ok(Msg::Frame(image, info)) = rx.recv() {
        let step = (|| {
            let t0 = Instant::now();
            let mut frame = RgbFrame::from_image(image, plan.scale)?;
            shared
                .convert_us
                .fetch_add(t0.elapsed().as_micros() as u64, Ordering::Relaxed);
            if encoder.is_none() {
                size = (frame.width, frame.height);
                encoder = Some(make_encoder(plan, &frame)?);
            } else if (frame.width, frame.height) != size {
                frame = frame.fitted(size.0, size.1);
            }
            let t1 = Instant::now();
            let bytes = encoder.as_mut().map_or(Ok(0), |e| e.push(&frame, &info))?;
            shared
                .encode_us
                .fetch_add(t1.elapsed().as_micros() as u64, Ordering::Relaxed);
            Ok::<u64, CaptureError>(bytes)
        })();
        match step {
            Ok(bytes) => {
                shared.bytes.store(bytes, Ordering::Relaxed);
                shared.frames.fetch_add(1, Ordering::Relaxed);
                infos.push(info);
            }
            Err(e) => {
                set_error(shared, &e);
                failed = true;
                break;
            }
        }
    }
    shared.state.store(FINISHING, Ordering::Relaxed);
    if let Some(enc) = encoder {
        match enc.finish() {
            Ok(bytes) => shared.bytes.store(bytes, Ordering::Relaxed),
            Err(e) => {
                set_error(shared, &e);
                failed = true;
            }
        }
    }
    failed |= write_sidecar(plan, shared, size, &infos);
    shared
        .state
        .store(if failed { FAILED } else { FINISHED }, Ordering::Release);
}
