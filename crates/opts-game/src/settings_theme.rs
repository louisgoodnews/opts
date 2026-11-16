//! Theme and interface part of [`GameSettings`]: parsing and writing the keys `theme_mode`,
//! `auto_light_from`, `auto_dark_from`, `high_contrast`, `active_theme`, `ui_scale`, `density`
//! and `reduced_motion`, the migration from the old `is_dark_theme` key, and the bridge to the
//! `opts-gui` resources.

use std::collections::BTreeMap;

use bevy::prelude::*;

use opts_gui::theme::{
    mode::parse_time, motion::MotionPreference, Density, ThemeMode, ThemePreferences,
    ThemeSwitchLock, UiScalePercent,
};

use crate::app_state::AppState;
use crate::settings::GameSettings;

mod desktop;
mod poll;
mod values;

pub use desktop::{os_color_scheme_hint, os_reduced_motion_hint, parse_gsettings_scheme};
pub use poll::{poll_os_color_scheme, OsSchemePoll};

impl GameSettings {
    /// Applies one stored key if it is a theme or interface key; unreadable values leave the
    /// current (default) value. Returns `true` when the key was one of ours.
    pub(crate) fn apply_theme_key(&mut self, key: &str, value: &str) -> bool {
        match key {
            "theme_mode" => {
                if let Some(mode) = ThemeMode::parse(value) {
                    self.theme_mode = mode;
                }
            }
            "is_dark_theme" => {
                self.is_dark_theme = value.trim().parse::<bool>().ok();
            }
            "auto_light_from" => {
                if parse_time(value).is_some() {
                    self.auto_light_from = value.trim().to_string();
                }
            }
            "auto_dark_from" => {
                if parse_time(value).is_some() {
                    self.auto_dark_from = value.trim().to_string();
                }
            }
            "high_contrast" => {
                self.high_contrast = value.trim().parse::<bool>().unwrap_or(false);
            }
            "active_theme" => {
                self.active_theme = value.to_string();
            }
            "theme_file" => {
                self.theme_file = value.trim().to_string();
            }
            "ui_scale" => {
                if let Ok(p) = value.trim().parse::<u32>() {
                    self.ui_scale = UiScalePercent::clamped(p).0;
                }
            }
            "density" => {
                if let Some(d) = Density::parse(value) {
                    self.density = d;
                }
            }
            "reduced_motion" => {
                self.reduced_motion = value.trim().parse::<bool>().unwrap_or(false);
            }
            _ => return false,
        }
        true
    }

    /// Maps the legacy `is_dark_theme` flag to `theme_mode` when the settings did not contain a
    /// `theme_mode` (`mode_present` false); with neither the default (dark) stays.
    pub(crate) fn migrate_legacy_theme(&mut self, mode_present: bool) {
        if !mode_present {
            if let Some(dark) = self.is_dark_theme {
                self.theme_mode = ThemeMode::from_legacy_is_dark(dark);
            }
        }
    }

    /// The persisted theme and interface values as key/value pairs.
    pub(crate) fn theme_entries(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("theme_mode".into(), self.theme_mode.key().to_string());
        m.insert("auto_light_from".into(), self.auto_light_from.clone());
        m.insert("auto_dark_from".into(), self.auto_dark_from.clone());
        m.insert("high_contrast".into(), self.high_contrast.to_string());
        m.insert("active_theme".into(), self.active_theme.clone());
        m.insert("theme_file".into(), self.theme_file.clone());
        m.insert("ui_scale".into(), self.ui_scale.to_string());
        m.insert("density".into(), self.density.key().to_string());
        m.insert("reduced_motion".into(), self.reduced_motion.to_string());
        m
    }
}

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
