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
mod ladder;
mod resolve;

pub use ladder::*;
pub use opts_light_quality_core::*;
pub use resolve::*;

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
    /// Bloom strength on the main camera.
    pub bloom: BloomLevel,
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

#[cfg(test)]
mod tests;
