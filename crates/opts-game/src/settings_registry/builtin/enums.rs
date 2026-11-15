//! The option lists of the built-in settings: the stored values and their message keys.

use opts_gui::l10n::keys::theme;

use crate::settings_registry::descriptor::opt;

/// Stored values of the window modes.
pub(super) const WINDOW_MODES: &[crate::settings_registry::EnumOption] = &[
    opt("windowed", "set-opt-window-windowed"),
    opt("borderless", "set-opt-window-borderless"),
    opt("fullscreen", "set-opt-window-fullscreen"),
];

/// The window sizes offered (stored as `WIDTHxHEIGHT`; labels are the numbers themselves).
pub(super) const RESOLUTIONS: &[crate::settings_registry::EnumOption] = &[
    opt("1024x640", "set-opt-res-1024x640"),
    opt("1280x720", "set-opt-res-1280x720"),
    opt("1600x900", "set-opt-res-1600x900"),
    opt("1920x1080", "set-opt-res-1920x1080"),
    opt("2560x1440", "set-opt-res-2560x1440"),
    opt("3840x2160", "set-opt-res-3840x2160"),
];

pub(super) const FILTERING: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-filter-off"),
    opt("4", "set-opt-filter-4"),
    opt("8", "set-opt-filter-8"),
    opt("16", "set-opt-filter-16"),
];

pub(super) const LOOKS: &[crate::settings_registry::EnumOption] = &[
    opt("vivid", "set-opt-look-vivid"),
    opt("natural", "set-opt-look-natural"),
    opt("bevy", "set-opt-look-bevy"),
];

pub(super) const THEME_MODES: &[crate::settings_registry::EnumOption] = &[
    opt("dark", theme::MODE_DARK),
    opt("light", theme::MODE_LIGHT),
    opt("system", theme::MODE_SYSTEM),
    opt("auto", theme::MODE_AUTO),
];

pub(super) const ACCENTS: &[crate::settings_registry::EnumOption] = &[
    opt("Default", theme::ACCENT_DEFAULT),
    opt("Amber", theme::ACCENT_AMBER),
    opt("Violet", theme::ACCENT_VIOLET),
    opt("Emerald", theme::ACCENT_EMERALD),
    opt("Teal", theme::ACCENT_TEAL),
];

pub(super) const DENSITIES: &[crate::settings_registry::EnumOption] = &[
    opt("comfortable", theme::DENSITY_COMFORTABLE),
    opt("compact", theme::DENSITY_COMPACT),
    opt("dense", theme::DENSITY_DENSE),
];

pub(super) const PRECIPITATION: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-precip-off"),
    opt("low", "set-opt-precip-low"),
    opt("medium", "set-opt-precip-medium"),
    opt("high", "set-opt-precip-high"),
];

pub(super) const UNITS: &[crate::settings_registry::EnumOption] = &[
    opt("kmh", "set-opt-units-kmh"),
    opt("mph", "set-opt-units-mph"),
];

pub(super) const MINIMAP_ZOOMS: &[crate::settings_registry::EnumOption] = &[
    opt("100", "set-opt-minimap-zoom-100"),
    opt("200", "set-opt-minimap-zoom-200"),
    opt("500", "set-opt-minimap-zoom-500"),
    opt("1000", "set-opt-minimap-zoom-1000"),
];

pub(super) const MINIMAP_ORIENTATIONS: &[crate::settings_registry::EnumOption] = &[
    opt("heading", "set-opt-minimap-heading"),
    opt("north", "set-opt-minimap-north"),
];

pub(super) const MINIMAP_ZOOM_MODES: &[crate::settings_registry::EnumOption] = &[
    opt("speed", "set-opt-minimap-zoom-mode-speed"),
    opt("fixed", "set-opt-minimap-zoom-mode-fixed"),
];

pub(super) const LOG_LEVELS: &[crate::settings_registry::EnumOption] = &[
    opt("error", "set-opt-log-error"),
    opt("warn", "set-opt-log-warn"),
    opt("info", "set-opt-log-info"),
    opt("debug", "set-opt-log-debug"),
];

pub(super) const TRACE_MODES: &[crate::settings_registry::EnumOption] = &[
    opt("off", "set-opt-trace-off"),
    opt("summary", "set-opt-trace-summary"),
    opt("full", "set-opt-trace-full"),
];
