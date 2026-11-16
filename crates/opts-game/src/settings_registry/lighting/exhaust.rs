//! The exhaust settings (`graphics.exhaust`, `graphics.exhaust_particles`, M-0117): whether the
//! engine's exhaust plume is drawn, and how many puffs may be alive at once.

use super::*;

/// The on/off setting of the exhaust plume (default `on`).
pub const EXHAUST_KEY: &str = "graphics.exhaust";

/// The cap on the live puffs of the exhaust (4..=48, default 12).
pub const EXHAUST_CAP_KEY: &str = "graphics.exhaust_particles";

const LEVELS: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-exhaust-off"),
    opt("on", "set-opt-exhaust-on"),
];

/// The descriptors of the exhaust settings.
#[must_use]
pub fn descriptors() -> Vec<D> {
    vec![
        d(
            EXHAUST_KEY,
            K::Enum(LEVELS),
            "on",
            113,
            "set-exhaust",
            "set-exhaust-desc",
            &["exhaust", "smoke", "plume", "cold start", "puffs"],
        ),
        d(
            EXHAUST_CAP_KEY,
            super::list::int(4, 48, 4),
            "12",
            114,
            "set-exhaust-particles",
            "set-exhaust-particles-desc",
            &["exhaust", "particles", "cap", "performance", "puffs"],
        ),
    ]
}

/// Whether the exhaust plume is drawn (`on` unless the setting is `off`).
#[must_use]
pub fn exhaust_on_of(settings: &GameSettings) -> bool {
    settings.raw(EXHAUST_KEY).as_deref() != Some("off")
}

/// The cap on the live puffs the settings hold (12 when missing or unreadable, 4..=48).
#[must_use]
pub fn exhaust_cap_of(settings: &GameSettings) -> u32 {
    settings
        .raw(EXHAUST_CAP_KEY)
        .and_then(|v| v.trim().parse::<u32>().ok())
        .map_or(12, |v| v.clamp(4, 48))
}
