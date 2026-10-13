//! Effects of settings that are not read by a typed consumer: texture filtering, look, language,
//! log level and the developer panel.

use bevy::prelude::*;
use opts_gui::l10n::{Localization, LOCALE_ENV};

use super::SettingChanged;
use crate::look::{ActiveLook, Look};
use crate::settings::GameSettings;

/// The crates whose log level the *Log detail* setting moves.
const LOG_TARGETS: [&str; 6] = [
    "opts_game",
    "opts_bevy",
    "opts_core",
    "opts_sim",
    "opts_gui",
    "opts_catalogue",
];

fn level_of(text: &str) -> tracing::Level {
    match text {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::INFO,
    }
}

/// Applies a changed setting to the running game.
pub fn apply_live_changes(
    mut changes: MessageReader<SettingChanged>,
    mut loc: ResMut<Localization>,
    active: Option<ResMut<ActiveLook>>,
    overlay: Option<ResMut<crate::control_trace::TraceOverlay>>,
) {
    let mut active = active;
    let mut overlay = overlay;
    for c in changes.read() {
        match c.key.as_str() {
            "texture_filtering" => {
                if let Some(f) = opts_bevy::TextureFiltering::parse(&c.new) {
                    opts_bevy::set_texture_filtering(f);
                }
            }
            "look" => {
                if let (Some(a), Some(l)) = (active.as_mut(), Look::parse(&c.new)) {
                    a.0 = l;
                }
            }
            "interface.language" => loc.set_locale(&c.new),
            "advanced.trace_scene" => {
                if let Some(mode) = opts_bevy::scene_trace::Mode::parse(&c.new) {
                    opts_bevy::scene_trace::set_mode(mode);
                }
            }
            "advanced.trace_overlay" => {
                if let Some(o) = overlay.as_mut() {
                    o.enabled = c.new == "true";
                }
            }
            "advanced.log_level" => {
                for t in LOG_TARGETS {
                    let _ = opts_logging::set_level(t, level_of(&c.new));
                }
            }
            _ => {}
        }
    }
}

/// Start-up: the saved language (unless `OPTS_LOCALE` chooses one for this run).
pub fn apply_saved_language(settings: Res<GameSettings>, mut loc: ResMut<Localization>) {
    if std::env::var_os(LOCALE_ENV).is_some() {
        return;
    }
    if let Some(lang) = settings.raw("interface.language").filter(|l| !l.is_empty()) {
        if lang != loc.locale() && loc.can_select(&lang) {
            loc.set_locale(&lang);
        }
    }
}

/// The developer panel (the F10 block of the drive view) follows the setting; F10 still toggles
/// it for the moment.
pub fn sync_developer_panel(
    settings: Res<GameSettings>,
    stats: Option<ResMut<crate::in_game::omsi_world::WorldStats>>,
    mut last: Local<Option<bool>>,
) {
    let Some(mut stats) = stats else { return };
    let want = settings.raw("advanced.developer_panel").as_deref() == Some("true");
    if stats.is_added() || *last != Some(want) {
        stats.overlay_visible = want;
        *last = Some(want);
    }
}

/// Start-up: the saved scene trace mode (unless `--trace-scene` chose one for this run) and overlay.
pub fn apply_saved_trace(
    settings: Res<GameSettings>,
    args: Option<Res<crate::app::AppArgs>>,
    overlay: Option<ResMut<crate::control_trace::TraceOverlay>>,
) {
    if args.is_none_or(|a| a.0.trace_scene.is_none()) {
        if let Some(mode) = settings
            .raw("advanced.trace_scene")
            .and_then(|m| opts_bevy::scene_trace::Mode::parse(&m))
        {
            opts_bevy::scene_trace::set_mode(mode);
        }
    }
    if let Some(mut o) = overlay {
        o.enabled = settings.raw("advanced.trace_overlay").as_deref() == Some("true");
    }
}
