//! The Interface page settings (theme, scale, language, units, HUD).

use super::*;

mod gameplay;

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

/// UI scale, fonts, density and reduced motion.
fn scale_and_fonts() -> Vec<D> {
    vec![
        D::new(
            "ui_scale",
            K::Int {
                min: 75,
                max: 200,
                step: 25,
            },
            "100",
            C::Interface,
            40,
            "set-ui-scale",
            "set-ui-scale-desc",
        )
        .presets(&["100", "125", "150", "200"], " %")
        .keywords(&["size", "zoom", "text size", "accessibility"]),
        D::new(
            "ui_font",
            K::Text,
            "",
            C::Interface,
            42,
            "set-ui-font",
            "set-ui-font-desc",
        )
        .keywords(&["font", "typeface", "dyslexia", "accessibility", "readable"]),
        D::new(
            "ui_font_size",
            K::Int {
                min: 12,
                max: 24,
                step: 1,
            },
            "14",
            C::Interface,
            44,
            "set-ui-font-size",
            "set-ui-font-size-desc",
        )
        .presets(&["12", "14", "16", "18", "20", "24"], " px")
        .keywords(&[
            "font",
            "text size",
            "base size",
            "accessibility",
            "larger text",
        ]),
        D::new(
            "density",
            K::Enum(DENSITIES),
            "comfortable",
            C::Interface,
            50,
            "set-density",
            "set-density-desc",
        )
        .keywords(&["spacing", "compact"]),
        D::new(
            "reduced_motion",
            K::Bool,
            "false",
            C::Interface,
            60,
            "set-reduced-motion",
            "set-reduced-motion-desc",
        )
        .keywords(&["animation", "accessibility", "transitions"]),
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
