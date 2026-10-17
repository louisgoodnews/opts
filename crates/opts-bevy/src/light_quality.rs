//! Lighting quality: the state the lighting effects read, the unified fallback ladder and the
//! automatic governor (`docs/src/architecture/lighting-system.md` section 9).
//!
//! * [`LightQuality`] holds the effective values (sun-ray steps, night shafts, cone switches and
//!   cap, depth fade, mirrors, glass reflections, bloom) plus the ladder `level` and the `mode`.
//!   The effects (other modules) only read it.
//! * [`ladder_preset`] is the table of the six ladder levels.
//! * In [`LightMode::Automatic`] the governor ([`step`], run by [`run_governor`]) moves the level
//!   from measured frame times; in [`LightMode::Custom`] the values come from the user's own
//!   controls ([`LightQualityInput::custom`], written by the game from the settings) and the
//!   governor never changes anything.
//!
//! The game decides *when* the governor may count frames ([`GovernorGate`]) and writes the
//! requested mode and custom values; this module has no knowledge of settings or app states.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::time::Real;

/// The highest (cheapest) ladder level.
pub const MAX_LEVEL: u8 = 5;
/// The most simultaneous cones any setting can ask for.
pub const MAX_CONE_CAP: u32 = 96;

/// Sun-ray (volumetric fog) quality: the ray-march step count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shafts {
    /// No rays (the volumetric components are removed).
    Off,
    /// 8 steps.
    Low,
    /// 16 steps (the default).
    #[default]
    Medium,
    /// 32 steps.
    High,
    /// 64 steps.
    Ultra,
}

impl Shafts {
    /// All values, cheapest first.
    pub const ALL: [Self; 5] = [Self::Off, Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// The ray-march steps (0 when off).
    #[must_use]
    pub const fn steps(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 8,
            Self::Medium => 16,
            Self::High => 32,
            Self::Ultra => 64,
        }
    }

    /// The stored/protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }

    /// Parses a [`name`](Self::name).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == text.trim())
    }
}

/// Who decides the values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightMode {
    /// The governor picks the ladder level from the frame time (the default).
    #[default]
    Automatic,
    /// The user's controls are authoritative; the governor does nothing.
    Custom,
}

impl LightMode {
    /// The stored/protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Custom => "custom",
        }
    }

    /// Parses a [`name`](Self::name); anything else is `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "automatic" => Some(Self::Automatic),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// The effective lighting quality. The default is ladder level 0 in automatic mode.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightQuality {
    /// Who decides.
    pub mode: LightMode,
    /// The ladder level (0 full .. 5 volumetrics off). In custom mode the level the values
    /// started from (it no longer moves).
    pub level: u8,
    /// Sun-ray steps.
    pub shafts: Shafts,
    /// Rays also at night (moon as the carrier).
    pub night_shafts: bool,
    /// Cones of street lanterns.
    pub lantern_cones: bool,
    /// Cones of vehicle headlights.
    pub headlight_cones: bool,
    /// Most simultaneous cones, `0..=96`.
    pub cone_cap: u32,
    /// Cones fade where they meet geometry (needs a depth prepass).
    pub cone_depth_fade: bool,
    /// Mirror cameras, `0..=2`.
    pub mirror_count: u8,
    /// A mirror renders every n-th frame (`1`, `2` or `4`).
    pub mirror_every: u8,
    /// Sky-cubemap reflections on glass.
    pub glass_reflections: bool,
    /// Bloom.
    pub bloom: bool,
}

impl Default for LightQuality {
    fn default() -> Self {
        ladder_preset(0)
    }
}

impl LightQuality {
    /// Whether any volumetric component (rays) is wanted at all.
    #[must_use]
    pub const fn shafts_on(&self) -> bool {
        !matches!(self.shafts, Shafts::Off)
    }

    /// The values clamped into their ranges (`cone_cap` 0..=96, `mirror_count` 0..=2,
    /// `mirror_every` 1/2/4, `level` 0..=5).
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.cone_cap = self.cone_cap.min(MAX_CONE_CAP);
        self.mirror_count = self.mirror_count.min(2);
        self.mirror_every = match self.mirror_every {
            0 | 1 => 1,
            2 | 3 => 2,
            _ => 4,
        };
        self.level = self.level.min(MAX_LEVEL);
        self
    }

    /// The same values with another mode and level.
    #[must_use]
    pub const fn with_mode(mut self, mode: LightMode, level: u8) -> Self {
        self.mode = mode;
        self.level = level;
        self
    }

    /// Whether every effect value equals `other`'s (mode and level ignored).
    #[must_use]
    pub fn same_values(&self, other: &Self) -> bool {
        self.with_mode(LightMode::Automatic, 0) == other.with_mode(LightMode::Automatic, 0)
    }
}

/// The unified fallback ladder of `lighting-system.md` section 9 (automatic mode).
///
/// | level | name | rays | night | lanterns | headlights | cap | depth fade | mirrors | glass | bloom |
/// | ---: | :--- | :--- | :--- | :--- | :--- | ---: | :--- | :--- | :--- | :--- |
/// | 0 | Full | medium (16) | on | on | on | 64 | on | 2 every 2nd | on | off |
/// | 1 | Fewer cones | medium | on | on | on | 48 | on | 2 every 2nd | on | off |
/// | 2 | Headlights only | medium | on | off | on | 16 | off | 2 every 2nd | on | off |
/// | 3 | Cones off | medium | on | off | off | 0 | off | 2 every 2nd | on | off |
/// | 4 | Sun rays only | low (8) | off | off | off | 0 | off | 0 | on | off |
/// | 5 | All volumetrics off | off | off | off | off | 0 | off | 0 | on | off |
///
/// The document's "cones 48, then 24" of level 1 is not a separate rung: level 1 is 48 and the
/// next rung already cuts to the 16 headlight cones of level 2 (the measured difference between
/// 24 and 16 cones is about 0.1 ms, not worth a sixth step). Bloom is off in every row (it is
/// off by default; level 4 "no bloom" holds for the whole ladder). HDR is never touched.
/// `level` above 5 is treated as 5.
#[must_use]
pub fn ladder_preset(level: u8) -> LightQuality {
    let level = level.min(MAX_LEVEL);
    LightQuality {
        mode: LightMode::Automatic,
        level,
        shafts: match level {
            0..=3 => Shafts::Medium,
            4 => Shafts::Low,
            _ => Shafts::Off,
        },
        night_shafts: level <= 3,
        lantern_cones: level <= 1,
        headlight_cones: level <= 2,
        cone_cap: match level {
            0 => 64,
            1 => 48,
            2 => 16,
            _ => 0,
        },
        cone_depth_fade: level <= 1,
        mirror_count: if level <= 3 { 2 } else { 0 },
        mirror_every: 2,
        glass_reflections: true,
        bloom: false,
    }
}

/// The name of a ladder level.
#[must_use]
pub const fn level_name(level: u8) -> &'static str {
    match level {
        0 => "full",
        1 => "fewer-cones",
        2 => "headlights-only",
        3 => "cones-off",
        4 => "sun-rays-only",
        _ => "volumetrics-off",
    }
}

/// What the game asks for: the mode and, for [`LightMode::Custom`], the user's values.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LightQualityInput {
    /// The requested mode.
    pub mode: LightMode,
    /// The user's custom values (their `mode` and `level` fields are ignored).
    pub custom: LightQuality,
}

// ---------------------------------------------------------------------------------------------
// Governor
// ---------------------------------------------------------------------------------------------

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

/// Makes [`LightQuality`] follow the mode, the custom values and the governor's level.
pub fn resolve_light_quality(
    input: Res<LightQualityInput>,
    state: Res<GovernorState>,
    mut quality: ResMut<LightQuality>,
) {
    let want = match input.mode {
        LightMode::Automatic => ladder_preset(state.level),
        LightMode::Custom => input
            .custom
            .clamped()
            .with_mode(LightMode::Custom, state.level),
    };
    if *quality != want {
        *quality = want;
    }
}

/// Registers the lighting quality state and the governor.
#[derive(Default)]
pub struct LightQualityPlugin;

impl Plugin for LightQualityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .init_resource::<LightQualityInput>()
            .init_resource::<GovernorSettings>()
            .init_resource::<GovernorState>()
            .init_resource::<GovernorGate>()
            .init_resource::<GovernorTargetFps>()
            .init_resource::<GovernorBudget>()
            .add_systems(
                Update,
                (update_governor_budget, run_governor, resolve_light_quality)
                    .chain()
                    .in_set(LightQualitySet),
            );
    }
}

/// The governor and the resolution of [`LightQuality`]; schedule input writers `.before` it and
/// consumers `.after` it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LightQualitySet;

#[cfg(test)]
mod tests;
