//! The quality ladder: the preset of each level and the level names.

use super::*;

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
