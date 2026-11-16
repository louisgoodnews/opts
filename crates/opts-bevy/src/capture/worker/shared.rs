//! The lifecycle of the worker and the counters it shares with the main thread.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
