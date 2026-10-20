//! The settings the game itself registers: the keys of earlier versions (their stored names and
//! values are unchanged) and the new ones of M-0063.

use opts_gui::l10n::keys::theme;
use opts_gui::not_yet::{SETTINGS_AUDIO, SETTINGS_REBIND, SETTINGS_TIME_SCALE};

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

const PRECIPITATION: &[super::EnumOption] = &[
    opt("off", "set-opt-precip-off"),
    opt("low", "set-opt-precip-low"),
    opt("medium", "set-opt-precip-medium"),
    opt("high", "set-opt-precip-high"),
];

const UNITS: &[super::EnumOption] = &[
    opt("kmh", "set-opt-units-kmh"),
    opt("mph", "set-opt-units-mph"),
];

const MINIMAP_ZOOMS: &[super::EnumOption] = &[
    opt("100", "set-opt-minimap-zoom-100"),
    opt("200", "set-opt-minimap-zoom-200"),
    opt("500", "set-opt-minimap-zoom-500"),
    opt("1000", "set-opt-minimap-zoom-1000"),
];

const MINIMAP_ORIENTATIONS: &[super::EnumOption] = &[
    opt("heading", "set-opt-minimap-heading"),
    opt("north", "set-opt-minimap-north"),
];

const MINIMAP_ZOOM_MODES: &[super::EnumOption] = &[
    opt("speed", "set-opt-minimap-zoom-mode-speed"),
    opt("fixed", "set-opt-minimap-zoom-mode-fixed"),
];

const LOG_LEVELS: &[super::EnumOption] = &[
    opt("error", "set-opt-log-error"),
    opt("warn", "set-opt-log-warn"),
    opt("info", "set-opt-log-info"),
    opt("debug", "set-opt-log-debug"),
];

const TRACE_MODES: &[super::EnumOption] = &[
    opt("off", "set-opt-trace-off"),
    opt("summary", "set-opt-trace-summary"),
    opt("full", "set-opt-trace-full"),
];

mod graphics;
use graphics::graphics;

mod interface;
use interface::interface;

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
        D::new(
            "advanced.trace_scene",
            K::Enum(TRACE_MODES),
            "off",
            C::Advanced,
            60,
            "set-trace-scene",
            "set-trace-scene-desc",
        )
        .keywords(&[
            "developer",
            "debug",
            "mesh",
            "trace",
            "asset",
            "tile",
            "jsonl",
        ])
        .since("0.3"),
        D::new(
            "advanced.trace_overlay",
            K::Bool,
            "false",
            C::Advanced,
            70,
            "set-trace-overlay",
            "set-trace-overlay-desc",
        )
        .keywords(&["developer", "debug", "boxes", "overlay", "trace"])
        .since("0.3"),
        D::new(
            "advanced.streaming_budget_ms",
            K::Int {
                min: 0,
                max: 50,
                step: 1,
            },
            "8",
            C::Advanced,
            80,
            "set-streaming-budget",
            "set-streaming-budget-desc",
        )
        .keywords(&[
            "map",
            "tiles",
            "streaming",
            "stutter",
            "freeze",
            "frame",
            "performance",
        ])
        .since("0.3"),
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

/// The mini map's own settings (its on/off switch is `hud.minimap`, generated with the widgets).
fn minimap() -> Vec<D> {
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

/// The keys a map profile may set: they change performance or the simulated content of a map
/// (map-profiles design §1). Everything else is the person's or the machine's.
const MAP_PROFILE_KEYS: &[&str] = &[
    "graphics.view_distance",
    "graphics.water",
    "graphics.weather_fog",
    "graphics.weather_clouds",
    "graphics.weather_precipitation",
    "graphics.weather_wet_ground",
    "graphics.light_quality_mode",
    "graphics.light_shafts",
    "graphics.light_night_shafts",
    "graphics.light_lantern_cones",
    "graphics.light_headlight_cones",
    "graphics.light_cone_cap",
    "graphics.light_cone_depth_fade",
    "graphics.light_governor_target_fps",
    "graphics.mirror_count",
    "graphics.mirror_rate",
    "graphics.glass_reflections",
    "graphics.bloom",
    "texture_filtering",
    "gameplay.traffic",
    "gameplay.pedestrians",
    "gameplay.load_all_tiles",
    "gameplay.tile_radius",
    "advanced.streaming_budget_ms",
];

/// Every built-in descriptor.
#[must_use]
pub fn all() -> Vec<D> {
    let mut v = all_unscoped();
    for d in &mut v {
        if MAP_PROFILE_KEYS.contains(&d.key) {
            d.profile = super::descriptor::ProfileScope::Map;
        }
    }
    v
}

fn all_unscoped() -> Vec<D> {
    let mut v = graphics();
    v.extend(super::lighting::descriptors());
    v.extend(interface());
    v.extend(hud());
    v.extend(minimap());
    v.extend(content());
    v.extend(crate::hardware::descriptors());
    v.extend(crate::vehicle_keys::descriptors());
    v
}
