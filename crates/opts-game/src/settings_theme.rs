//! Theme and interface systems of [`GameSettings`]: the bridge from the settings to the `opts-gui`
//! resources and the OS colour scheme. The keys and values of the settings are in the
//! `opts-game-state` crate.

use bevy::prelude::*;

#[cfg(test)]
use opts_gui::theme::ThemeMode;
use opts_gui::theme::{
    motion::MotionPreference, Density, ThemePreferences, ThemeSwitchLock, UiScalePercent,
};

use crate::app_state::AppState;
use crate::settings::GameSettings;

mod desktop;
mod poll;

pub use desktop::{os_color_scheme_hint, os_reduced_motion_hint, parse_gsettings_scheme};
pub use poll::{poll_os_color_scheme, OsSchemePoll};

/// Copies changed [`GameSettings`] into the `opts-gui` resources: theme preferences, UI scale,
/// density and the motion preference. Only differing values are written, so change detection of
/// the receiving resources stays meaningful.
pub fn sync_theme_settings(
    mut loaded: Local<(String, Option<std::sync::Arc<opts_gui::theme::CustomTheme>>)>,
    settings: Res<GameSettings>,
    mut prefs: ResMut<ThemePreferences>,
    mut scale: ResMut<UiScalePercent>,
    mut density: ResMut<Density>,
    mut motion: ResMut<MotionPreference>,
) {
    if !settings.is_changed() {
        return;
    }
    if loaded.0 != settings.theme_file {
        loaded.1 = crate::theme_files::load_theme_file(&settings.theme_file);
        loaded.0 = settings.theme_file.clone();
    }
    let mut wanted = settings.theme_preferences();
    wanted.custom = loaded.1.clone();
    if *prefs != wanted {
        *prefs = wanted;
    }
    if *scale != settings.ui_scale_percent() {
        *scale = settings.ui_scale_percent();
    }
    if *density != settings.density {
        *density = settings.density;
    }
    if motion.user_setting != settings.reduced_motion {
        motion.user_setting = settings.reduced_motion;
    }
}

/// Holds automatic theme changes back while the application is loading or in the simulation
/// (loading screen and drive); the menus are free to switch.
pub fn lock_theme_switching(state: Res<State<AppState>>, mut lock: ResMut<ThemeSwitchLock>) {
    let locked = matches!(state.get(), AppState::Loading | AppState::InGame);
    if lock.0 != locked {
        lock.0 = locked;
    }
}

#[cfg(test)]
mod tests;
