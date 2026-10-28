//! Automatic lighting quality governor based on measured frame deadlines.

use super::*;

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

/// The frame-time budget (ms) the governor judges against.
///
/// * vsync on: the refresh interval of the window's monitor ([`FALLBACK_VSYNC_MS`] when
///   unknown), but never shorter than `1000 / target_fps` (the setting can ask for a lower
///   target, e.g. 30 fps on a 144 Hz display);
/// * vsync off: `1000 / target_fps`.
///
/// `target_fps` is clamped to [`MIN_TARGET_FPS`]`..=`[`MAX_TARGET_FPS`].
#[must_use]
pub fn target_ms(vsync: bool, refresh_millihertz: Option<u32>, target_fps: u32) -> f32 {
    let wanted = 1000.0 / target_fps.clamp(MIN_TARGET_FPS, MAX_TARGET_FPS) as f32;
    if vsync {
        let interval = refresh_millihertz
            .filter(|m| *m > 0)
            .map_or(FALLBACK_VSYNC_MS, |m| 1.0e6 / m as f32);
        interval.max(wanted)
    } else {
        wanted
    }
}

/// Whether a present mode waits for the vertical blank.
#[must_use]
pub fn is_vsync(mode: bevy::window::PresentMode) -> bool {
    use bevy::window::PresentMode as P;
    matches!(mode, P::Fifo | P::FifoRelaxed | P::AutoVsync)
}

/// Thresholds of the governor; overridable for tests. Defaults are the named constants.
///
/// The governor judges **missed deadlines**, not absolute frame time: see [`step`].
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GovernorSettings {
    /// Frames in the sliding window.
    pub window_frames: usize,
    /// A frame is late above `late_factor * target_ms`.
    pub late_factor: f32,
    /// One level down when at least this share of a full window is late ...
    pub down_late_share: f32,
    /// ... for this long (s).
    pub down_after_s: f32,
    /// At most one step per this many seconds.
    pub min_step_interval_s: f32,
    /// One level up after this many seconds with no late frame in a full window (the first wait).
    pub up_after_s: f32,
    /// The wait doubles after every failed probation, up to this.
    pub up_after_max_s: f32,
    /// A level down within this long (s) of a level up is a failed probation.
    pub probation_s: f32,
    /// The wait is back to `up_after_s` after this long (s) without a level down.
    pub backoff_reset_s: f32,
    /// A frame longer than this (ms) is a hitch (tile streaming) and is not counted ...
    pub hitch_ms: f32,
    /// ... unless this many such frames follow each other: that is a real slowdown, not a hitch.
    pub hitch_run_limit: u32,
    /// Seconds ignored after the gate opens (shader/pipeline compilation).
    pub warmup_s: f32,
}

impl Default for GovernorSettings {
    fn default() -> Self {
        Self {
            window_frames: WINDOW_FRAMES,
            late_factor: LATE_FACTOR,
            down_late_share: DOWN_LATE_SHARE,
            down_after_s: DOWN_AFTER_S,
            min_step_interval_s: MIN_STEP_INTERVAL_S,
            up_after_s: UP_AFTER_S,
            up_after_max_s: UP_AFTER_MAX_S,
            probation_s: PROBATION_S,
            backoff_reset_s: BACKOFF_RESET_S,
            hitch_ms: HITCH_MS,
            hitch_run_limit: HITCH_RUN_LIMIT,
            warmup_s: WARMUP_S,
        }
    }
}

/// What the governor remembers.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GovernorState {
    /// The ladder level, `0..=5`.
    pub level: u8,
    /// The deadline budget (ms) of the latest frame (for reporting).
    pub target_ms: f32,
    /// The share of late frames in the window at the latest judgement (for reporting).
    pub late_share: f32,
    /// Seconds a level up currently needs (20, 40, 80 ... up to the cap).
    pub up_wait_s: f32,
    window: VecDeque<f32>,
    over_s: f32,
    clean_s: f32,
    since_step_s: f32,
    since_up_s: f32,
    since_down_s: f32,
    warmup_s: f32,
    hitch_run: u32,
}

impl Default for GovernorState {
    fn default() -> Self {
        Self {
            level: 0,
            target_ms: FALLBACK_VSYNC_MS,
            late_share: 0.0,
            up_wait_s: UP_AFTER_S,
            window: VecDeque::new(),
            over_s: 0.0,
            clean_s: 0.0,
            // The first step may happen as soon as the evidence is in.
            since_step_s: f32::MAX,
            since_up_s: f32::MAX,
            since_down_s: f32::MAX,
            warmup_s: 0.0,
            hitch_run: 0,
        }
    }
}

impl GovernorState {
    /// A governor that starts at `level` (clamped to `0..=5`).
    #[must_use]
    pub fn at(level: u8) -> Self {
        Self {
            level: level.min(MAX_LEVEL),
            ..Self::default()
        }
    }

    /// Back to level 0 with an empty window (custom to automatic).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The gate is closed (paused, loading, minimised, custom mode): forget the window and the
    /// timers and ignore the next [`GovernorSettings::warmup_s`] seconds once it opens again.
    /// The level and the up-backoff stay.
    pub fn pause(&mut self) {
        self.window.clear();
        self.over_s = 0.0;
        self.clean_s = 0.0;
        self.warmup_s = 0.0;
        self.hitch_run = 0;
    }

    /// Frames currently in the window.
    #[must_use]
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    fn restart_window(&mut self) {
        self.window.clear();
        self.over_s = 0.0;
        self.clean_s = 0.0;
        self.since_step_s = 0.0;
    }
}

/// A level change the governor made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    /// The level before.
    pub from: u8,
    /// The level now.
    pub to: u8,
    /// The deadline budget (ms) the frames were judged against.
    pub target_ms: f32,
    /// The share of late frames in the window that decided (0 for a level up).
    pub late_share: f32,
}

/// Feeds one frame (`frame_ms`, taking `dt` seconds of wall time) to the governor, judged
/// against the deadline `target_ms`.
///
/// * The first `warmup_s` seconds after the gate opened are ignored.
/// * A frame longer than `hitch_ms` is a hitch (a tile streamed in): it is neither counted in the
///   window nor in the timers, unless `hitch_run_limit` such frames follow each other.
/// * A frame is **late** above `late_factor * target_ms`. The judgement needs a full window.
///   At least `down_late_share` late frames for `down_after_s` move one level down (steps are at
///   least `min_step_interval_s` apart).
/// * A level up is a probation: after `up_wait_s` seconds with no late frame in a full window
///   the governor tries one level up. If the level goes down again within `probation_s` the wait
///   doubles (`up_after_s`, 2x, 4x ... up to `up_after_max_s`); after `backoff_reset_s` without a
///   level down it is `up_after_s` again. Nothing is inferred from short frames, so this works
///   under vsync where a frame can never be shorter than the refresh interval.
/// * The level stays in `0..=5`; after a step the window starts again.
pub fn step(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    target_ms: f32,
    frame_ms: f32,
    dt: f32,
) -> Option<Change> {
    state.target_ms = target_ms;
    state.since_step_s = (state.since_step_s + dt).min(1.0e9);
    state.since_up_s = (state.since_up_s + dt).min(1.0e9);
    state.since_down_s = (state.since_down_s + dt).min(1.0e9);
    if state.since_down_s >= cfg.backoff_reset_s {
        state.up_wait_s = cfg.up_after_s;
    }
    if state.warmup_s < cfg.warmup_s {
        state.warmup_s += dt;
        return None;
    }
    if frame_ms > cfg.hitch_ms {
        state.hitch_run += 1;
        if state.hitch_run <= cfg.hitch_run_limit {
            return None;
        }
    } else {
        state.hitch_run = 0;
    }
    state.window.push_back(frame_ms);
    while state.window.len() > cfg.window_frames {
        state.window.pop_front();
    }
    if state.window.len() < cfg.window_frames {
        return None;
    }
    let late_above = cfg.late_factor * target_ms;
    let late = state.window.iter().filter(|ms| **ms > late_above).count();
    let share = late as f32 / state.window.len() as f32;
    state.late_share = share;
    state.over_s = if share >= cfg.down_late_share {
        state.over_s + dt
    } else {
        0.0
    };
    state.clean_s = if late == 0 { state.clean_s + dt } else { 0.0 };
    let ready = state.since_step_s >= cfg.min_step_interval_s;
    let from = state.level;
    let to = if ready && state.over_s >= cfg.down_after_s && from < MAX_LEVEL {
        if state.since_up_s < cfg.probation_s {
            state.up_wait_s = (state.up_wait_s * 2.0).min(cfg.up_after_max_s);
        }
        state.since_down_s = 0.0;
        from + 1
    } else if ready && state.clean_s >= state.up_wait_s && from > 0 {
        state.since_up_s = 0.0;
        from - 1
    } else {
        return None;
    };
    state.level = to;
    state.restart_window();
    Some(Change {
        from,
        to,
        target_ms,
        late_share: share,
    })
}

/// Whether the game lets the governor count frames now (in a drive, not paused, not loading).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GovernorGate {
    /// Frames count only while this is true.
    pub open: bool,
}

/// The target frame rate the user asked for (`graphics.light_governor_target_fps`, written by
/// the game). Used as the deadline when vsync is off and as a lower bound of the target when
/// vsync is on.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernorTargetFps(pub u32);

impl Default for GovernorTargetFps {
    fn default() -> Self {
        Self(DEFAULT_TARGET_FPS)
    }
}

/// What the governor knows about the display this frame.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GovernorBudget {
    /// The deadline (ms), see [`target_ms`].
    pub target_ms: f32,
    /// Whether the window presents with vsync.
    pub vsync: bool,
    /// The refresh rate of the window's monitor, when known.
    pub refresh_millihertz: Option<u32>,
    /// False while the window is unfocused or minimised: no frames count then.
    pub window_active: bool,
}

impl Default for GovernorBudget {
    fn default() -> Self {
        Self {
            target_ms: FALLBACK_VSYNC_MS,
            vsync: false,
            refresh_millihertz: None,
            window_active: true,
        }
    }
}

/// Reads the primary window (present mode, focus, size) and its monitor (refresh rate) and
/// computes the [`GovernorBudget`]. Without a window (headless) the budget is the target fps.
pub fn update_governor_budget(
    windows: Query<(&Window, Option<&bevy::window::OnMonitor>), With<bevy::window::PrimaryWindow>>,
    monitors: Query<&bevy::window::Monitor>,
    primary_monitor: Query<&bevy::window::Monitor, With<bevy::window::PrimaryMonitor>>,
    fps: Res<GovernorTargetFps>,
    mut budget: ResMut<GovernorBudget>,
) {
    let want = match windows.single() {
        Ok((window, on)) => {
            let vsync = is_vsync(window.present_mode);
            let refresh = on
                .and_then(|m| monitors.get(m.0).ok())
                .or_else(|| primary_monitor.iter().next())
                .and_then(|m| m.refresh_rate_millihertz);
            GovernorBudget {
                target_ms: target_ms(vsync, refresh, fps.0),
                vsync,
                refresh_millihertz: refresh,
                window_active: window.focused
                    && window.resolution.physical_width() > 0
                    && window.resolution.physical_height() > 0,
            }
        }
        Err(_) => GovernorBudget {
            target_ms: target_ms(false, None, fps.0),
            ..GovernorBudget::default()
        },
    };
    if *budget != want {
        *budget = want;
    }
}

/// Runs the governor on the real frame time (`Time<Real>`, not the simulation clock).
pub fn run_governor(
    time: Res<Time<Real>>,
    gate: Res<GovernorGate>,
    budget: Res<GovernorBudget>,
    cfg: Res<GovernorSettings>,
    input: Res<LightQualityInput>,
    mut state: ResMut<GovernorState>,
    mut last_mode: Local<LightMode>,
) {
    if *last_mode != input.mode {
        // Custom to automatic restarts at level 0; automatic to custom keeps the level.
        if input.mode == LightMode::Automatic {
            state.reset();
        }
        *last_mode = input.mode;
    }
    if input.mode != LightMode::Automatic || !gate.open || !budget.window_active {
        state.pause();
        return;
    }
    let dt = time.delta_secs();
    if let Some(c) = step(&mut state, &cfg, budget.target_ms, dt * 1000.0, dt) {
        tracing::info!(
            from = c.from,
            to = c.to,
            target_ms = c.target_ms,
            late_share = c.late_share,
            vsync = budget.vsync,
            name = level_name(c.to),
            "lighting quality level changed"
        );
    }
}
