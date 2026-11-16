//! The custom lighting controls: their keys, their stored values, and the [`LightQuality`] they make.

use super::*;

/// The custom controls (every key except the mode), in page order.
pub const CUSTOM_KEYS: [&str; 10] = [
    "graphics.light_shafts",
    "graphics.light_night_shafts",
    "graphics.light_lantern_cones",
    "graphics.light_headlight_cones",
    "graphics.light_cone_cap",
    "graphics.light_cone_depth_fade",
    "graphics.mirror_count",
    "graphics.mirror_rate",
    "graphics.glass_reflections",
    "graphics.bloom",
];

/// Whether `key` is one of the custom controls (disabled while the mode is automatic).
#[must_use]
pub fn is_custom_key(key: &str) -> bool {
    CUSTOM_KEYS.contains(&key)
}

/// The custom values the settings hold (`mode` and `level` of the result are placeholders).
#[must_use]
pub fn custom_of(settings: &GameSettings) -> LightQuality {
    let def = ladder_preset(0);
    let flag = |key: &str, default: bool| match settings.raw(key).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    };
    let num = |key: &str, default: u32| {
        settings
            .raw(key)
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(default)
    };
    LightQuality {
        mode: LightMode::Custom,
        level: 0,
        shafts: settings
            .raw(CUSTOM_KEYS[0])
            .and_then(|s| Shafts::parse(&s))
            .unwrap_or(def.shafts),
        night_shafts: flag(CUSTOM_KEYS[1], def.night_shafts),
        lantern_cones: flag(CUSTOM_KEYS[2], def.lantern_cones),
        headlight_cones: flag(CUSTOM_KEYS[3], def.headlight_cones),
        cone_cap: num(CUSTOM_KEYS[4], def.cone_cap),
        cone_depth_fade: flag(CUSTOM_KEYS[5], def.cone_depth_fade),
        mirror_count: num(CUSTOM_KEYS[6], u32::from(def.mirror_count)) as u8,
        mirror_every: num(CUSTOM_KEYS[7], u32::from(def.mirror_every)) as u8,
        glass_reflections: flag(CUSTOM_KEYS[8], def.glass_reflections),
        bloom: settings
            .raw(CUSTOM_KEYS[9])
            .and_then(|s| BloomLevel::parse(&s))
            .unwrap_or(def.bloom),
    }
    .clamped()
}

/// The stored text of each custom control for `q`, in [`CUSTOM_KEYS`] order.
#[must_use]
pub fn texts_of(q: &LightQuality) -> [String; 10] {
    [
        q.shafts.name().to_string(),
        q.night_shafts.to_string(),
        q.lantern_cones.to_string(),
        q.headlight_cones.to_string(),
        q.cone_cap.to_string(),
        q.cone_depth_fade.to_string(),
        q.mirror_count.to_string(),
        q.mirror_every.to_string(),
        q.glass_reflections.to_string(),
        q.bloom.name().to_string(),
    ]
}

/// The stored text of the custom control `key` for `q`.
#[must_use]
pub fn text_for(q: &LightQuality, key: &str) -> Option<String> {
    let i = CUSTOM_KEYS.iter().position(|k| *k == key)?;
    Some(texts_of(q)[i].clone())
}
