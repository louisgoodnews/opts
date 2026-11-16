//! The Interface page settings (theme, scale, language, units, HUD).

use super::*;

mod gameplay;
mod scale;
use scale::scale_and_fonts;

/// Part 2: interface and gameplay.
pub(super) fn interface() -> Vec<D> {
    let mut out = [theme(), scale_and_fonts(), language_and_units()].concat();
    out.extend(gameplay::gameplay());
    out
}

/// Theme mode, automatic switching, contrast and the theme file.
fn theme() -> Vec<D> {
    vec![
        D::new(
            "theme_mode",
            K::Enum(THEME_MODES),
            "dark",
            C::Interface,
            10,
            "set-theme-mode",
            "set-theme-mode-desc",
        )
        .keywords(&["dark", "light", "night", "day", "appearance"]),
        D::new(
            "auto_light_from",
            K::Time,
            "07:00",
            C::Interface,
            11,
            "set-auto-light-from",
            "set-auto-light-from-desc",
        )
        .keywords(&["theme", "clock", "schedule"]),
        D::new(
            "auto_dark_from",
            K::Time,
            "19:00",
            C::Interface,
            12,
            "set-auto-dark-from",
            "set-auto-dark-from-desc",
        )
        .keywords(&["theme", "clock", "schedule"]),
        D::new(
            "high_contrast",
            K::Bool,
            "false",
            C::Interface,
            20,
            "set-high-contrast",
            "set-high-contrast-desc",
        )
        .keywords(&["accessibility", "contrast"]),
        D::new(
            "active_theme",
            K::Enum(ACCENTS),
            "Default",
            C::Interface,
            30,
            "set-accent",
            "set-accent-desc",
        )
        .keywords(&["colour", "color", "accent", "preset"]),
        D::new(
            "theme_file",
            K::Text,
            "",
            C::Interface,
            35,
            "set-theme-file",
            "set-theme-file-desc",
        )
        .keywords(&["theme", "custom", "colours"]),
    ]
}

/// Language, units and the HUD hints.
fn language_and_units() -> Vec<D> {
    vec![
        D::new(
            "interface.language",
            K::Text,
            "en",
            C::Interface,
            70,
            "set-language",
            "set-language-desc",
        )
        .keywords(&["sprache", "deutsch", "german", "english", "locale"])
        .since("0.2"),
        D::new(
            "interface.language_reload",
            K::Action,
            "",
            C::Interface,
            71,
            "set-language-reload",
            "set-language-reload-desc",
        )
        .keywords(&["languages", "packs", "translation", "refresh", "rescan"])
        .since("0.3"),
        D::new(
            "interface.units",
            K::Enum(UNITS),
            "kmh",
            C::Interface,
            80,
            "set-units",
            "set-units-desc",
        )
        .keywords(&["speed", "km/h", "mph", "metric"])
        .since("0.2"),
        D::new(
            "interface.hud_hints",
            K::Bool,
            "true",
            C::Interface,
            90,
            "set-hud-hints",
            "set-hud-hints-desc",
        )
        .keywords(&["tips", "help", "hud"])
        .since("0.2"),
    ]
}
