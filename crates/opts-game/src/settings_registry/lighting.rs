//! The lighting-quality settings (`graphics.light_*`, `graphics.mirror_*`, `graphics.glass_reflections`,
//! `graphics.bloom`) and their link to [`opts_bevy::light_quality`]
//! (`docs/src/architecture/lighting-system.md` section 9).
//!
//! `graphics.light_quality_mode` chooses who decides: `automatic` (the governor, the controls
//! below show what it applies and are disabled) or `custom` (the controls are authoritative).

use bevy::prelude::*;
use opts_bevy::light_quality::{
    ladder_preset, BloomLevel, GovernorGate, GovernorTargetFps, LightMode, LightQuality,
    LightQualityInput, LightQualitySet, Shafts,
};

use super::descriptor::{opt, Category as C, SettingDescriptor as D, SettingKind as K};
use super::{ChangeSource, SettingChanged, SettingsApi};
use crate::settings::GameSettings;

mod custom;
mod list;
mod systems;
pub use custom::*;
pub use list::descriptors;
pub use systems::*;

/// The mode setting.
pub const MODE_KEY: &str = "graphics.light_quality_mode";

/// The target frame rate of the automatic governor (30..=240, default 60).
pub const TARGET_FPS_KEY: &str = "graphics.light_governor_target_fps";

const MODES: &[super::EnumOption] = &[
    opt("automatic", "set-opt-lightmode-automatic"),
    opt("custom", "set-opt-lightmode-custom"),
];

const SHAFTS: &[super::EnumOption] = &[
    opt("off", "set-opt-shafts-off"),
    opt("low", "set-opt-shafts-low"),
    opt("medium", "set-opt-shafts-medium"),
    opt("high", "set-opt-shafts-high"),
    opt("ultra", "set-opt-shafts-ultra"),
];

const BLOOM_LEVELS: &[super::EnumOption] = &[
    opt("off", "set-opt-bloom-off"),
    opt("low", "set-opt-bloom-low"),
    opt("natural", "set-opt-bloom-natural"),
    opt("high", "set-opt-bloom-high"),
];

const MIRROR_RATES: &[super::EnumOption] = &[
    opt("1", "set-opt-mirror-rate-1"),
    opt("2", "set-opt-mirror-rate-2"),
    opt("4", "set-opt-mirror-rate-4"),
];

/// Whether the stored mode is automatic (the default; an unknown text counts as automatic).
#[must_use]
pub fn is_automatic(settings: &GameSettings) -> bool {
    mode_of(settings) == LightMode::Automatic
}

/// The stored mode.
#[must_use]
pub fn mode_of(settings: &GameSettings) -> LightMode {
    settings
        .raw(MODE_KEY)
        .and_then(|m| LightMode::parse(&m))
        .unwrap_or_default()
}

fn d(
    key: &'static str,
    kind: K,
    default: &'static str,
    order: u16,
    label: &'static str,
    desc: &'static str,
    keywords: &'static [&'static str],
) -> D {
    D::new(key, kind, default, C::Graphics, order, label, desc)
        .keywords(keywords)
        .since("0.3")
}

/// The target frame rate the settings hold (`30..=240`, 60 when missing or unreadable).
#[must_use]
pub fn target_fps_of(settings: &GameSettings) -> u32 {
    settings
        .raw(TARGET_FPS_KEY)
        .and_then(|v| v.trim().parse::<u32>().ok())
        .map_or(60, |v| v.clamp(30, 240))
}
