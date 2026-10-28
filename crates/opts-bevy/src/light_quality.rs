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

mod governor;
pub use governor::*;

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
/// | 4 | Sun rays only | low (8) | off | off | off | 0 | off | 1 every 4th | on | off |
/// | 5 | All volumetrics off | off | off | off | off | 0 | off | 1 every 4th | on | off |
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
        // The lowest rungs keep the mirrors live at a quarter of the rate (one picture every 4th
        // frame, shared by the mirrors in view): a fixed stock picture in a mirror reads as a bug.
        mirror_count: if level <= 3 { 2 } else { 1 },
        mirror_every: if level <= 3 { 2 } else { 4 },
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
