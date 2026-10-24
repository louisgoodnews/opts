//! The Interface page settings (theme, scale, language, units, HUD).

use super::*;

/// Part 2: interface and gameplay.
pub(super) fn interface() -> Vec<D> {
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
        D::new(
            "gameplay.traffic",
            K::Int {
                min: 0,
                max: 1000,
                step: 10,
            },
            "150",
            C::Gameplay,
            10,
            "set-traffic",
            "set-traffic-desc",
        )
        .keywords(&["ai", "vehicles", "cars", "density", "baseline"])
        .since("0.2"),
        D::new(
            "gameplay.parked",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "100",
            C::Gameplay,
            12,
            "set-parked",
            "set-parked-desc",
        )
        .keywords(&["parked", "cars", "bays", "density"])
        .since("0.5"),
        D::new(
            "gameplay.pedestrians",
            K::Int {
                min: 0,
                max: 1000,
                step: 10,
            },
            "120",
            C::Gameplay,
            20,
            "set-pedestrians",
            "set-pedestrians-desc",
        )
        .keywords(&["people", "walkers", "density"])
        .since("0.2"),
        D::new(
            "gameplay.passengers",
            K::Bool,
            "true",
            C::Gameplay,
            25,
            "set-passengers",
            "set-passengers-desc",
        )
        .keywords(&["people", "boarding", "bus stop", "tickets", "passengers"])
        .since("0.4"),
        D::new(
            "gameplay.passenger_rate",
            K::Int {
                min: 0,
                max: 1000,
                step: 10,
            },
            "120",
            C::Gameplay,
            26,
            "set-passenger-rate",
            "set-passenger-rate-desc",
        )
        .keywords(&["people", "boarding", "bus stop", "crowd", "load", "full"])
        .since("0.5"),
        D::new(
            "gameplay.time_scale",
            K::Float {
                min: 0.0,
                max: 60.0,
                step: 1.0,
            },
            "1",
            C::Gameplay,
            30,
            "set-time-scale",
            "set-time-scale-desc",
        )
        .keywords(&["clock", "speed", "time"])
        .not_yet(&SETTINGS_TIME_SCALE),
        D::new(
            "gameplay.autosave_minutes",
            K::Int {
                min: 0,
                max: 60,
                step: 5,
            },
            "0",
            C::Gameplay,
            40,
            "set-autosave",
            "set-autosave-desc",
        )
        .keywords(&["save", "backup", "interval"])
        .since("0.3"),
        D::new(
            "camera.door_overlay",
            K::Bool,
            "false",
            C::Gameplay,
            95,
            "set-door-overlay",
            "set-door-overlay-desc",
        )
        .keywords(&[
            "door",
            "camera",
            "overlay",
            "picture in picture",
            "boarding",
        ])
        .since("0.3"),
        D::new(
            "gameplay.passenger_weight",
            K::Bool,
            "true",
            C::Gameplay,
            96,
            "set-passenger-weight",
            "set-passenger-weight-desc",
        )
        .keywords(&["passengers", "mass", "weight", "load", "braking"])
        .since("0.3"),
        D::new(
            "gameplay.passenger_mass_kg",
            K::Int {
                min: 40,
                max: 120,
                step: 5,
            },
            "75",
            C::Gameplay,
            97,
            "set-passenger-mass",
            "set-passenger-mass-desc",
        )
        .keywords(&["passengers", "mass", "weight", "kilogram"])
        .since("0.3"),
        D::new(
            "gameplay.load_all_tiles",
            K::Bool,
            "false",
            C::Gameplay,
            50,
            "set-load-all-tiles",
            "set-load-all-tiles-desc",
        )
        .keywords(&[
            "map",
            "tiles",
            "streaming",
            "memory",
            "preload",
            "everything",
        ])
        .since("0.3"),
        D::new(
            "gameplay.tile_radius",
            K::Int {
                min: 0,
                max: 16,
                step: 1,
            },
            "0",
            C::Gameplay,
            60,
            "set-tile-radius",
            "set-tile-radius-desc",
        )
        .keywords(&["map", "tiles", "streaming", "distance", "view", "automatic"])
        .since("0.3"),
    ]
}
