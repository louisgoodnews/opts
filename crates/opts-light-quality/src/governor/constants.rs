//! The governor's thresholds: frame-rate limits, the window, the judgement and the timers.

/// The default frame-time target when nothing else is known (60 fps).
pub const DEFAULT_TARGET_FPS: u32 = 60;
/// The lowest target frame rate the setting allows.
pub const MIN_TARGET_FPS: u32 = 30;
/// The highest target frame rate the setting allows.
pub const MAX_TARGET_FPS: u32 = 240;
/// The vsync interval (ms) assumed when the monitor's refresh rate is unknown.
pub const FALLBACK_VSYNC_MS: f32 = 1000.0 / 60.0;
/// Frames in the sliding window.
pub const WINDOW_FRAMES: usize = 120;
/// A frame is late when it takes longer than this multiple of the target (a missed vblank under
/// vsync shows as about 2 times the interval).
pub const LATE_FACTOR: f32 = 1.25;
/// Share of late frames in a full window that counts as "missing the deadline".
pub const DOWN_LATE_SHARE: f32 = 0.10;
/// Seconds the window has to stay over [`DOWN_LATE_SHARE`] before one level goes down.
pub const DOWN_AFTER_S: f32 = 2.0;
/// At most one step per this many seconds.
pub const MIN_STEP_INTERVAL_S: f32 = 5.0;
/// Seconds without a late frame in a full window before the first level up.
pub const UP_AFTER_S: f32 = 20.0;
/// The longest wait before a level up after repeated failed probations.
pub const UP_AFTER_MAX_S: f32 = 160.0;
/// A level down within this many seconds of a level up means the probation failed.
pub const PROBATION_S: f32 = 30.0;
/// Seconds without a level down after which the up-wait is back to [`UP_AFTER_S`].
pub const BACKOFF_RESET_S: f32 = 300.0;
/// A frame longer than this (ms) is a hitch (tile streaming) and is not counted.
pub const HITCH_MS: f32 = 50.0;
/// More than this many hitch frames in a row are a real slowdown and do count.
pub const HITCH_RUN_LIMIT: u32 = 3;
/// Seconds after the gate opens (and after every pause) that are ignored: shader and pipeline
/// compilation hitches.
pub const WARMUP_S: f32 = 5.0;
