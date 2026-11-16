//! The HUD switches and the mini map settings.

use crate::settings_registry::descriptor::{
    Category as C, SettingDescriptor as D, SettingKind as K,
};

use super::enums::{MINIMAP_ORIENTATIONS, MINIMAP_ZOOMS, MINIMAP_ZOOM_MODES};

/// The on/off switches of the HUD widgets (`hud.<name>`), one per widget of the HUD registry
/// (`crate::hud::widget::WIDGETS`); the hints widget uses the older `interface.hud_hints`.
pub(super) fn hud() -> Vec<D> {
    crate::hud::widget::WIDGETS
        .iter()
        .filter_map(|w| w.switch.as_ref())
        .filter(|sw| sw.key.starts_with("hud."))
        .enumerate()
        .map(|(i, sw)| {
            D::new(
                sw.key,
                K::Bool,
                if sw.default_on { "true" } else { "false" },
                C::Interface,
                100 + u16::try_from(i).unwrap_or(0) * 5,
                sw.label_key,
                sw.description_key,
            )
            .keywords(&["hud", "instrument", "display", "widget"])
            .since("0.2")
        })
        .collect()
}

/// The mini map's own settings (its on/off switch is `hud.minimap`, generated with the widgets).
pub(super) fn minimap() -> Vec<D> {
    vec![
        D::new(
            "hud.minimap_zoom",
            K::Enum(MINIMAP_ZOOMS),
            "200",
            C::Interface,
            160,
            "set-hud-minimap-zoom",
            "set-hud-minimap-zoom-desc",
        )
        .keywords(&["map", "mini map", "zoom", "scale", "radius"])
        .since("0.3"),
        D::new(
            "hud.minimap_zoom_mode",
            K::Enum(MINIMAP_ZOOM_MODES),
            "speed",
            C::Interface,
            162,
            "set-hud-minimap-zoom-mode",
            "set-hud-minimap-zoom-mode-desc",
        )
        .keywords(&["map", "mini map", "zoom", "speed", "automatic"])
        .since("0.3"),
        D::new(
            "hud.minimap_orientation",
            K::Enum(MINIMAP_ORIENTATIONS),
            "heading",
            C::Interface,
            165,
            "set-hud-minimap-orientation",
            "set-hud-minimap-orientation-desc",
        )
        .keywords(&["map", "mini map", "north", "heading", "rotate"])
        .since("0.3"),
    ]
}
