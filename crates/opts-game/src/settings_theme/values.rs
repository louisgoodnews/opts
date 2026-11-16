//! The theme and interface values of [`GameSettings`] as the rest of the game reads them: the
//! auto-mode times, the accent, the preferences and the UI scale.

use chrono::NaiveTime;
use opts_gui::theme::{
    mode::{format_time, parse_time, DEFAULT_DARK_FROM, DEFAULT_LIGHT_FROM},
    AccentPreset, ThemePreferences, UiScalePercent,
};

use crate::settings::GameSettings;

impl GameSettings {
    /// The time the light theme starts in the auto mode (the default when unreadable).
    pub fn light_from(&self) -> NaiveTime {
        parse_time(&self.auto_light_from).unwrap_or(DEFAULT_LIGHT_FROM)
    }

    /// The time the dark theme starts in the auto mode (the default when unreadable).
    pub fn dark_from(&self) -> NaiveTime {
        parse_time(&self.auto_dark_from).unwrap_or(DEFAULT_DARK_FROM)
    }

    /// Sets the auto-mode switch times.
    pub fn set_auto_times(&mut self, light_from: NaiveTime, dark_from: NaiveTime) {
        self.auto_light_from = format_time(light_from);
        self.auto_dark_from = format_time(dark_from);
    }

    /// The accent preset (the default for an unknown name).
    pub fn accent(&self) -> AccentPreset {
        AccentPreset::parse(&self.active_theme).unwrap_or_default()
    }

    /// The `opts-gui` preferences these settings describe.
    pub fn theme_preferences(&self) -> ThemePreferences {
        ThemePreferences {
            mode: self.theme_mode,
            light_from: self.light_from(),
            dark_from: self.dark_from(),
            high_contrast: self.high_contrast,
            accent: self.accent(),
            custom: None,
        }
    }

    /// The UI scale as a clamped percentage resource.
    pub fn ui_scale_percent(&self) -> UiScalePercent {
        UiScalePercent::clamped(self.ui_scale)
    }
}
