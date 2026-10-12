//! The theme and interface choices of the UI tab: what each button means, how it changes the
//! settings, and the click handler.
//!
//! The handler only edits [`GameSettings`] and saves; `crate::settings_theme::sync_theme_settings`
//! moves the new values into the `opts-gui` resources (theme, scale, density, motion).

use bevy::prelude::*;
use chrono::{Duration, NaiveTime};
use opts_gui::prelude::*;
use opts_gui::theme::{mode::format_time, AccentPreset, Density, ThemeMode, UiScalePercent};

use crate::settings::GameSettings;
use crate::settings_screen::display_tab::choice_colors;
use crate::settings_screen::types::{
    AutoTimeLabel, ThemeChoice, ThemeChoiceButton, ThemeFileButton,
};

/// Step of the auto-mode time buttons.
pub const TIME_STEP_MINUTES: i32 = 30;

/// Moves a time of day by `minutes`, wrapping around midnight.
pub fn shift_time(time: NaiveTime, minutes: i32) -> NaiveTime {
    time + Duration::minutes(i64::from(minutes))
}

impl ThemeChoice {
    /// The label of the button.
    pub fn label(self) -> String {
        match self {
            Self::Mode(m) => m.label().to_string(),
            Self::HighContrast(on) | Self::ReducedMotion(on) => {
                if on { "On" } else { "Off" }.to_string()
            }
            Self::Accent(a) => a.label().to_string(),
            Self::Scale(p) => format!("{p} %"),
            Self::Density(d) => d.label().to_string(),
            Self::LightFromStep(d) | Self::DarkFromStep(d) => {
                if d < 0 { "-30 min" } else { "+30 min" }.to_string()
            }
        }
    }

    /// Whether two choices belong to the same setting (one radio group).
    pub fn same_group(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// Whether this option is the one stored in `settings` (step buttons never are).
    pub fn is_selected(self, settings: &GameSettings) -> bool {
        match self {
            Self::Mode(m) => settings.theme_mode == m,
            Self::HighContrast(on) => settings.high_contrast == on,
            Self::Accent(a) => settings.accent() == a,
            Self::Scale(p) => settings.ui_scale_percent() == UiScalePercent(p),
            Self::Density(d) => settings.density == d,
            Self::ReducedMotion(on) => settings.reduced_motion == on,
            Self::LightFromStep(_) | Self::DarkFromStep(_) => false,
        }
    }

    /// Stores this choice in `settings`.
    pub fn apply(self, settings: &mut GameSettings) {
        match self {
            Self::Mode(m) => settings.theme_mode = m,
            Self::HighContrast(on) => settings.high_contrast = on,
            Self::Accent(a) => settings.active_theme = a.key().to_string(),
            Self::Scale(p) => settings.ui_scale = UiScalePercent::clamped(p).0,
            Self::Density(d) => settings.density = d,
            Self::ReducedMotion(on) => settings.reduced_motion = on,
            Self::LightFromStep(d) => {
                let (l, k) = (shift_time(settings.light_from(), d), settings.dark_from());
                settings.set_auto_times(l, k);
            }
            Self::DarkFromStep(d) => {
                let (l, k) = (settings.light_from(), shift_time(settings.dark_from(), d));
                settings.set_auto_times(l, k);
            }
        }
    }

    /// All modes as choices.
    pub fn modes() -> Vec<Self> {
        ThemeMode::ALL.into_iter().map(Self::Mode).collect()
    }

    /// All accent presets as choices.
    pub fn accents() -> Vec<Self> {
        AccentPreset::ALL.into_iter().map(Self::Accent).collect()
    }

    /// The offered UI scales.
    pub fn scales() -> Vec<Self> {
        [75, 100, 125, 150, 175, 200]
            .into_iter()
            .map(Self::Scale)
            .collect()
    }

    /// All densities as choices.
    pub fn densities() -> Vec<Self> {
        Density::ALL.into_iter().map(Self::Density).collect()
    }
}

/// Applies the clicked choice, persists it and refreshes the buttons of its group and the
/// auto-time labels.
pub fn handle_theme_choice_clicks(
    mut events: MessageReader<ButtonClicked>,
    clicked: Query<&ThemeChoiceButton>,
    mut buttons: Query<(&ThemeChoiceButton, &mut BorderColor, &mut BackgroundColor)>,
    mut labels: Query<(&AutoTimeLabel, &mut Text)>,
    mut settings: ResMut<GameSettings>,
    theme: Res<MaterialTheme>,
) {
    let Some(choice) = events
        .read()
        .filter_map(|e| clicked.get(e.0).ok().map(|b| b.0))
        .last()
    else {
        return;
    };
    choice.apply(&mut settings);
    tracing::info!(?choice, "theme choice changed");
    for (button, mut border, mut bg) in &mut buttons {
        if button.0.same_group(choice) {
            let (b, g) = choice_colors(&theme, button.0.is_selected(&settings));
            *border = BorderColor::all(b);
            *bg = BackgroundColor(g);
        }
    }
    for (label, mut text) in &mut labels {
        let t = if label.0 {
            settings.light_from()
        } else {
            settings.dark_from()
        };
        text.0 = format_time(t);
    }
    if let Err(err) = settings.save() {
        tracing::warn!(err = ?err, "Failed to persist the theme choice to SQLite");
    }
}

/// Applies the clicked theme file (or none), persists it and refreshes the row.
pub fn handle_theme_file_clicks(
    mut events: MessageReader<ButtonClicked>,
    clicked: Query<&ThemeFileButton>,
    mut buttons: Query<(&ThemeFileButton, &mut BorderColor, &mut BackgroundColor)>,
    mut settings: ResMut<GameSettings>,
    theme: Res<MaterialTheme>,
) {
    let Some(choice) = events
        .read()
        .filter_map(|e| clicked.get(e.0).ok().map(|b| b.0.clone()))
        .last()
    else {
        return;
    };
    settings.theme_file = choice.clone().unwrap_or_default();
    tracing::info!(file = ?choice, "theme file changed");
    for (button, mut border, mut bg) in &mut buttons {
        let selected = button.0.clone().unwrap_or_default() == settings.theme_file;
        let (b, g) = choice_colors(&theme, selected);
        *border = BorderColor::all(b);
        *bg = BackgroundColor(g);
    }
    if let Err(err) = settings.save() {
        tracing::warn!(err = ?err, "Failed to persist the theme file choice to SQLite");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_shift_and_wrap() {
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).expect("t");
        assert_eq!(shift_time(t(7, 0), 30), t(7, 30));
        assert_eq!(shift_time(t(0, 0), -30), t(23, 30));
        assert_eq!(shift_time(t(23, 45), 30), t(0, 15));
    }

    #[test]
    fn choices_edit_the_settings() {
        let mut s = GameSettings::default();
        ThemeChoice::Mode(ThemeMode::AutoClock).apply(&mut s);
        ThemeChoice::HighContrast(true).apply(&mut s);
        ThemeChoice::Accent(AccentPreset::Teal).apply(&mut s);
        ThemeChoice::Scale(150).apply(&mut s);
        ThemeChoice::Density(Density::Dense).apply(&mut s);
        ThemeChoice::ReducedMotion(true).apply(&mut s);
        ThemeChoice::LightFromStep(30).apply(&mut s);
        ThemeChoice::DarkFromStep(-30).apply(&mut s);
        assert_eq!(s.theme_mode, ThemeMode::AutoClock);
        assert!(s.high_contrast);
        assert_eq!(s.active_theme, "Teal");
        assert_eq!(s.ui_scale, 150);
        assert_eq!(s.density, Density::Dense);
        assert!(s.reduced_motion);
        assert_eq!(s.auto_light_from, "07:30");
        assert_eq!(s.auto_dark_from, "18:30");
        assert!(ThemeChoice::Scale(150).is_selected(&s));
        assert!(!ThemeChoice::Scale(100).is_selected(&s));
        assert!(ThemeChoice::Mode(ThemeMode::Dark).same_group(ThemeChoice::Mode(ThemeMode::Light)));
        assert!(!ThemeChoice::Scale(100).same_group(ThemeChoice::Mode(ThemeMode::Light)));
    }

    #[test]
    fn scale_choices_stay_in_range() {
        for c in ThemeChoice::scales() {
            if let ThemeChoice::Scale(p) = c {
                assert!((75..=200).contains(&p));
            }
        }
    }
}
