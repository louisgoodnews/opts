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
) {
    let mut active = active;
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
