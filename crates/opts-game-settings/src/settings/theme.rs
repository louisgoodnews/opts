//! The theme and interface keys of [`GameSettings`]: parsing and writing `theme_mode`,
//! `auto_light_from`, `auto_dark_from`, `high_contrast`, `active_theme`, `ui_scale`, `density`,
//! `reduced_motion` and `theme_file`, and the migration from the old `is_dark_theme` key. The
//! systems that copy them into the `opts-gui` resources stay in `opts-game`.

use std::collections::BTreeMap;

use opts_gui::theme::{mode::parse_time, Density, ThemeMode, UiScalePercent};

use super::GameSettings;

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
    pub fn migrate_legacy_theme(&mut self, mode_present: bool) {
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
