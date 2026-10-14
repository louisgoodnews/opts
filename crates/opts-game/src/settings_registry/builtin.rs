//! The settings the game itself registers: the keys of earlier versions (their stored names and
//! values are unchanged) and the new ones of M-0063.

use opts_gui::l10n::keys::theme;
use opts_gui::not_yet::{
    SETTINGS_AUDIO, SETTINGS_REBIND, SETTINGS_TIME_SCALE, SETTINGS_VIEW_DISTANCE,
};

use super::descriptor::{opt, Apply, Category as C, SettingDescriptor as D, SettingKind as K};

/// Stored values of the window modes.
const WINDOW_MODES: &[super::EnumOption] = &[
    opt("windowed", "set-opt-window-windowed"),
    opt("borderless", "set-opt-window-borderless"),
    opt("fullscreen", "set-opt-window-fullscreen"),
];

/// The window sizes offered (stored as `WIDTHxHEIGHT`; labels are the numbers themselves).
const RESOLUTIONS: &[super::EnumOption] = &[
    opt("1024x640", "set-opt-res-1024x640"),
    opt("1280x720", "set-opt-res-1280x720"),
    opt("1600x900", "set-opt-res-1600x900"),
    opt("1920x1080", "set-opt-res-1920x1080"),
    opt("2560x1440", "set-opt-res-2560x1440"),
    opt("3840x2160", "set-opt-res-3840x2160"),
];

const FILTERING: &[super::EnumOption] = &[
    opt("off", "set-opt-filter-off"),
    opt("4", "set-opt-filter-4"),
    opt("8", "set-opt-filter-8"),
    opt("16", "set-opt-filter-16"),
];

const LOOKS: &[super::EnumOption] = &[
    opt("vivid", "set-opt-look-vivid"),
    opt("natural", "set-opt-look-natural"),
    opt("bevy", "set-opt-look-bevy"),
];

const THEME_MODES: &[super::EnumOption] = &[
    opt("dark", theme::MODE_DARK),
    opt("light", theme::MODE_LIGHT),
    opt("system", theme::MODE_SYSTEM),
    opt("auto", theme::MODE_AUTO),
];

const ACCENTS: &[super::EnumOption] = &[
    opt("Default", theme::ACCENT_DEFAULT),
    opt("Amber", theme::ACCENT_AMBER),
    opt("Violet", theme::ACCENT_VIOLET),
    opt("Emerald", theme::ACCENT_EMERALD),
    opt("Teal", theme::ACCENT_TEAL),
];

const DENSITIES: &[super::EnumOption] = &[
    opt("comfortable", theme::DENSITY_COMFORTABLE),
    opt("compact", theme::DENSITY_COMPACT),
    opt("dense", theme::DENSITY_DENSE),
];

const UNITS: &[super::EnumOption] = &[
    opt("kmh", "set-opt-units-kmh"),
    opt("mph", "set-opt-units-mph"),
];

const LOG_LEVELS: &[super::EnumOption] = &[
    opt("error", "set-opt-log-error"),
    opt("warn", "set-opt-log-warn"),
    opt("info", "set-opt-log-info"),
    opt("debug", "set-opt-log-debug"),
];

/// Part 1: graphics, audio, controls.
fn graphics() -> Vec<D> {
    vec![
        D::new(
            "graphics.window_mode",
            K::Enum(WINDOW_MODES),
            "windowed",
            C::Graphics,
            10,
            "set-window-mode",
            "set-window-mode-desc",
        )
        .keywords(&["fullscreen", "borderless", "window"])
        .apply(Apply::WithRevert(15))
        .since("0.2"),
        D::new(
            "graphics.resolution",
            K::Enum(RESOLUTIONS),
            "1920x1080",
            C::Graphics,
            20,
            "set-resolution",
            "set-resolution-desc",
        )
        .keywords(&["size", "window", "screen"])
        .apply(Apply::WithRevert(15))
        .since("0.2"),
        D::new(
            "graphics.vsync",
            K::Bool,
            "false",
            C::Graphics,
            30,
            "set-vsync",
            "set-vsync-desc",
        )
        .keywords(&["tearing", "frame rate", "fps"])
        .since("0.2"),
        D::new(
            "texture_filtering",
            K::Enum(FILTERING),
            "16",
            C::Graphics,
            40,
            "set-texture-filtering",
            "set-texture-filtering-desc",
        )
        .keywords(&["anisotropic", "mipmap", "sharp"]),
        D::new(
            "look",
            K::Enum(LOOKS),
            "vivid",
            C::Graphics,
            50,
            "set-look",
            "set-look-desc",
        )
        .keywords(&["colour", "color", "tonemapping", "saturation"]),
        D::new(
            "graphics.view_distance",
            K::Int {
                min: 1,
                max: 8,
                step: 1,
            },
            "4",
            C::Graphics,
            60,
            "set-view-distance",
            "set-view-distance-desc",
        )
        .keywords(&["draw distance", "lod"])
        .not_yet(&SETTINGS_VIEW_DISTANCE),
        D::new(
            "audio.master",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            10,
            "set-audio-master",
            "set-audio-master-desc",
        )
        .keywords(&["volume", "sound"])
        .not_yet(&SETTINGS_AUDIO),
        D::new(
            "audio.engine",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "80",
            C::Audio,
            20,
            "set-audio-engine",
            "set-audio-engine-desc",
        )
        .keywords(&["volume", "sound"])
        .not_yet(&SETTINGS_AUDIO),
        D::new(
            "audio.ambience",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "60",
            C::Audio,
            30,
            "set-audio-ambience",
            "set-audio-ambience-desc",
        )
        .keywords(&["volume", "sound"])
        .not_yet(&SETTINGS_AUDIO),
        D::new(
            "audio.interface",
            K::Int {
                min: 0,
                max: 100,
                step: 5,
            },
            "60",
            C::Audio,
            40,
            "set-audio-interface",
            "set-audio-interface-desc",
        )
        .keywords(&["volume", "sound", "clicks"])
        .not_yet(&SETTINGS_AUDIO),
        D::new(
            "controls.rebind",
            K::Action,
            "",
            C::Controls,
            20,
            "set-rebind",
            "set-rebind-desc",
        )
        .keywords(&["keys", "keyboard", "gamepad", "wheel", "binding"])
        .not_yet(&SETTINGS_REBIND),
    ]
}

/// Part 2: interface and gameplay.
fn interface() -> Vec<D> {
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
        .keywords(&["size", "zoom", "text size", "accessibility"]),
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
                max: 200,
                step: 5,
            },
            "30",
            C::Gameplay,
            10,
            "set-traffic",
            "set-traffic-desc",
        )
        .keywords(&["ai", "vehicles", "cars", "density"])
        .since("0.2"),
        D::new(
            "gameplay.pedestrians",
            K::Int {
                min: 0,
                max: 200,
                step: 5,
            },
            "40",
            C::Gameplay,
            20,
            "set-pedestrians",
            "set-pedestrians-desc",
        )
        .keywords(&["people", "walkers", "density"])
        .since("0.2"),
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
    ]
}

/// Part 3: content and advanced.
fn content() -> Vec<D> {
    vec![
        D::new(
            "omsi_path",
            K::Path,
            "",
            C::Content,
            10,
            "set-omsi-path",
            "set-omsi-path-desc",
        )
        .keywords(&["folder", "installation", "directory", "steam"])
        .apply(Apply::OnRestart),
        D::new(
            "discovery_interval_days",
            K::Int {
                min: 1,
                max: 90,
                step: 1,
            },
            "1",
            C::Content,
            20,
            "set-discovery-interval",
            "set-discovery-interval-desc",
        )
        .keywords(&["catalogue", "scan", "refresh"]),
        D::new(
            "content.rescan",
            K::Action,
            "",
            C::Content,
            30,
            "set-rescan",
            "set-rescan-desc",
        )
        .keywords(&["catalogue", "discovery", "refresh", "scan"])
        .since("0.2"),
        D::new(
            "content.clear_cache",
            K::Action,
            "",
            C::Content,
            40,
            "set-clear-cache",
            "set-clear-cache-desc",
        )
        .keywords(&["textures", "decoder", "disk", "space"])
        .since("0.2"),
        D::new(
            "recent_map_path",
            K::Path,
            "",
            C::Content,
            90,
            "set-recent-map",
            "set-recent-map-desc",
        )
        .hidden(),
        D::new(
            "advanced.developer_panel",
            K::Bool,
            "false",
            C::Advanced,
            10,
            "set-developer-panel",
            "set-developer-panel-desc",
        )
        .keywords(&["f10", "debug", "statistics", "overlay"])
        .since("0.2"),
        D::new(
            "advanced.log_level",
            K::Enum(LOG_LEVELS),
            "info",
            C::Advanced,
            20,
            "set-log-level",
            "set-log-level-desc",
        )
        .keywords(&["logging", "diagnostics", "debug"])
        .since("0.2"),
    ]
}

/// The on/off switches of the HUD widgets (`hud.<name>`), one per widget of the HUD registry
/// (`crate::hud::widget::WIDGETS`); the hints widget uses the older `interface.hud_hints`.
fn hud() -> Vec<D> {
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

/// Every built-in descriptor.
#[must_use]
pub fn all() -> Vec<D> {
    let mut v = graphics();
    v.extend(interface());
    v.extend(hud());
    v.extend(content());
    v.extend(crate::hardware::descriptors());
    v
}
