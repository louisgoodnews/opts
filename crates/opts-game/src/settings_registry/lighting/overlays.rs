//! The "Window and body overlays" setting (`graphics.overlays`, M-0120): its descriptor, and the
//! live step that applies it to the process-wide level and to the overlays already built.

use super::*;
use opts_bevy::overlays::{
    despawn_disallowed, overlay_level, set_overlay_level, OverlayKind, OverlayLevel,
};

/// The setting of the overlay level (`off`, `simple`, `full`; default `full`).
pub const OVERLAY_KEY: &str = "graphics.overlays";

const LEVELS: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-overlays-off"),
    opt("simple", "set-opt-overlays-simple"),
    opt("full", "set-opt-overlays-full"),
];

/// The descriptor of the overlay setting.
#[must_use]
pub fn descriptors() -> Vec<D> {
    vec![d(
        OVERLAY_KEY,
        K::Enum(LEVELS),
        "full",
        112,
        "set-overlays",
        "set-overlays-desc",
        &[
            "overlays",
            "dirt",
            "rain",
            "condensation",
            "windows",
            "body",
            "decals",
        ],
    )]
}

/// The overlay level the settings hold (`full` when missing or unknown).
#[must_use]
pub fn overlay_level_of(settings: &GameSettings) -> OverlayLevel {
    settings
        .raw(OVERLAY_KEY)
        .and_then(|t| OverlayLevel::parse(&t))
        .unwrap_or_default()
}

/// Keeps the process-wide level equal to the setting, live, and removes the overlays it drops.
pub fn sync_overlay_level(
    settings: Res<GameSettings>,
    mut commands: Commands,
    overlays: Query<(Entity, &OverlayKind)>,
) {
    let level = overlay_level_of(&settings);
    if overlay_level() != level {
        set_overlay_level(level);
    }
    despawn_disallowed(&mut commands, level, &overlays);
}
