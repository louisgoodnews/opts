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

/// Thresholds of the governor; overridable for tests.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GovernorSettings {
    /// Frames in the sliding window.
    pub window_frames: usize,
    /// The percentile that is judged (0.95).
    pub percentile: f32,
    /// One level down when the percentile exceeds this (ms) ...
    pub down_above_ms: f32,
    /// ... for this long (s).
    pub down_after_s: f32,
    /// One level up when the percentile stays below this (ms) ...
    pub up_below_ms: f32,
    /// ... for this long (s).
    pub up_after_s: f32,
    /// At most one step per this many seconds.
    pub min_step_interval_s: f32,
    /// A frame longer than this (ms) is a hitch (tile streaming) and is not counted ...
    pub hitch_ms: f32,
    /// ... unless this many such frames follow each other: that is a real slowdown, not a hitch.
    pub hitch_run_limit: u32,
}

impl Default for GovernorSettings {
    fn default() -> Self {
        Self {
            window_frames: 120,
            percentile: 0.95,
            down_above_ms: 15.0,
            down_after_s: 2.0,
            up_below_ms: 11.0,
            up_after_s: 15.0,
            min_step_interval_s: 5.0,
            hitch_ms: 50.0,
            hitch_run_limit: 3,
        }
    }
}

/// What the governor remembers.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GovernorState {
    /// The ladder level, `0..=5`.
    pub level: u8,
    window: VecDeque<f32>,
    over_s: f32,
    under_s: f32,
    since_step_s: f32,
    hitch_run: u32,
}

impl Default for GovernorState {
    fn default() -> Self {
        Self {
            level: 0,
            window: VecDeque::new(),
            over_s: 0.0,
            under_s: 0.0,
            // The first step may happen as soon as the evidence is in.
            since_step_s: f32::MAX,
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

    /// Frames currently in the window.
    #[must_use]
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    fn restart_window(&mut self) {
        self.window.clear();
        self.over_s = 0.0;
        self.under_s = 0.0;
        self.since_step_s = 0.0;
    }

    fn percentile(&self, p: f32) -> f32 {
        let mut v: Vec<f32> = self.window.iter().copied().collect();
        v.sort_by(f32::total_cmp);
        if v.is_empty() {
            return 0.0;
        }
        let rank = (p * v.len() as f32).ceil() as usize;
        v[rank.clamp(1, v.len()) - 1]
    }
}

/// A level change the governor made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    /// The level before.
    pub from: u8,
    /// The level now.
    pub to: u8,
    /// The 95th-percentile frame time (ms) that decided.
    pub p95_ms: f32,
}

/// Feeds one frame (`frame_ms`, taking `dt` seconds of wall time) to the governor.
///
/// * A frame longer than `hitch_ms` is a hitch (a tile streamed in): it is neither counted in the
///   window nor in the timers, unless `hitch_run_limit` such frames follow each other.
/// * The judgement needs a full window. The percentile above `down_above_ms` for
///   `down_after_s` moves one level down, below `up_below_ms` for `up_after_s` one level up;
///   steps are at least `min_step_interval_s` apart, the level stays in `0..=5`.
/// * After a step the window starts again, so the new level is judged on its own frames.
pub fn step(
    state: &mut GovernorState,
    cfg: &GovernorSettings,
    frame_ms: f32,
    dt: f32,
) -> Option<Change> {
    state.since_step_s = (state.since_step_s + dt).min(1.0e9);
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
    let p = state.percentile(cfg.percentile);
    state.over_s = if p > cfg.down_above_ms {
        state.over_s + dt
    } else {
        0.0
    };
    state.under_s = if p < cfg.up_below_ms {
        state.under_s + dt
    } else {
        0.0
    };
    let ready = state.since_step_s >= cfg.min_step_interval_s;
    let from = state.level;
    let to = if ready && state.over_s >= cfg.down_after_s && from < MAX_LEVEL {
        from + 1
    } else if ready && state.under_s >= cfg.up_after_s && from > 0 {
        from - 1
    } else {
        return None;
    };
    state.level = to;
    state.restart_window();
    Some(Change {
        from,
        to,
        p95_ms: p,
    })
}

/// Whether the game lets the governor count frames now (in a drive, not paused, not loading).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GovernorGate {
    /// Frames count only while this is true.
    pub open: bool,
}

/// Runs the governor on the real frame time (`Time<Real>`, not the simulation clock).
pub fn run_governor(
    time: Res<Time<Real>>,
    gate: Res<GovernorGate>,
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
    if input.mode != LightMode::Automatic || !gate.open {
        return;
    }
    let dt = time.delta_secs();
    if let Some(c) = step(&mut state, &cfg, dt * 1000.0, dt) {
        tracing::info!(
            from = c.from,
            to = c.to,
            p95_ms = c.p95_ms,
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
            .add_systems(
                Update,
                (run_governor, resolve_light_quality)
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
