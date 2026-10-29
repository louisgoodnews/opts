//! The list of lighting descriptors (a child of `lighting.rs`, kept apart to stay under the
//! file-size limit).

use super::*;

/// The descriptors; registered by [`super::builtin::all`].
#[must_use]
pub fn descriptors() -> Vec<D> {
    [quality(), light_cones(), mirrors_glass_bloom(), governor()].concat()
}

/// A whole-number setting with its range and step.
fn int(min: i64, max: i64, step: i64) -> K {
    K::Int { min, max, step }
}

/// The lighting mode and the light shafts.
fn quality() -> Vec<D> {
    vec![
        d(
            MODE_KEY,
            K::Enum(MODES),
            "automatic",
            100,
            "set-light-mode",
            "set-light-mode-desc",
            &[
                "lighting",
                "quality",
                "governor",
                "light shafts",
                "performance",
                "auto",
            ],
        ),
        d(
            CUSTOM_KEYS[0],
            K::Enum(SHAFTS),
            "medium",
            101,
            "set-light-shafts",
            "set-light-shafts-desc",
            &["sun rays", "god rays", "volumetric", "fog", "steps"],
        ),
        d(
            CUSTOM_KEYS[1],
            K::Bool,
            "true",
            102,
            "set-light-night-shafts",
            "set-light-night-shafts-desc",
            &["moon", "night", "rays"],
        ),
    ]
}

/// The street lamp and headlight cones.
fn light_cones() -> Vec<D> {
    vec![
        d(
            CUSTOM_KEYS[2],
            K::Bool,
            "true",
            103,
            "set-light-lantern-cones",
            "set-light-lantern-cones-desc",
            &["street lamps", "light cones", "lanterns"],
        ),
        d(
            CUSTOM_KEYS[3],
            K::Bool,
            "true",
            104,
            "set-light-headlight-cones",
            "set-light-headlight-cones-desc",
            &["headlights", "light cones", "vehicles"],
        ),
        d(
            CUSTOM_KEYS[4],
            int(0, 96, 8),
            "64",
            105,
            "set-light-cone-cap",
            "set-light-cone-cap-desc",
            &["light cones", "limit", "maximum"],
        ),
        d(
            CUSTOM_KEYS[5],
            K::Bool,
            "true",
            106,
            "set-light-cone-depth-fade",
            "set-light-cone-depth-fade-desc",
            &["light cones", "soft", "depth prepass"],
        ),
    ]
}

/// Mirrors, glass reflections and bloom.
fn mirrors_glass_bloom() -> Vec<D> {
    vec![
        d(
            CUSTOM_KEYS[6],
            int(0, 2, 1),
            "2",
            107,
            "set-mirror-count",
            "set-mirror-count-desc",
            &["mirrors", "rear view", "reflection"],
        ),
        d(
            CUSTOM_KEYS[7],
            K::Enum(MIRROR_RATES),
            "2",
            108,
            "set-mirror-rate",
            "set-mirror-rate-desc",
            &["mirrors", "update rate", "frames"],
        ),
        d(
            CUSTOM_KEYS[8],
            K::Bool,
            "true",
            109,
            "set-glass-reflections",
            "set-glass-reflections-desc",
            &["windows", "glass", "sky", "reflection"],
        ),
        d(
            CUSTOM_KEYS[9],
            K::Bool,
            "false",
            110,
            "set-bloom",
            "set-bloom-desc",
            &["glow", "hdr", "lights"],
        ),
    ]
}

/// The frame-rate target of the governor.
fn governor() -> Vec<D> {
    vec![d(
        TARGET_FPS_KEY,
        int(30, 240, 10),
        "60",
        111,
        "set-light-governor-fps",
        "set-light-governor-fps-desc",
        &[
            "governor",
            "target",
            "frame rate",
            "fps",
            "automatic",
            "vsync",
        ],
    )]
}
