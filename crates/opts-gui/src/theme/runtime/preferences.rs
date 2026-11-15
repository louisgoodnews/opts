//! The user's theme choices and the palette they select.

use std::sync::Arc;

use bevy::prelude::*;
use chrono::NaiveTime;

use crate::theme::accent::AccentPreset;
use crate::theme::color::ColorScheme;
use crate::theme::custom::CustomTheme;
use crate::theme::mode::{theme_for, ThemeKind, ThemeMode, DEFAULT_DARK_FROM, DEFAULT_LIGHT_FROM};

/// The user's theme choices (settings `theme_mode`, `auto_light_from`, `auto_dark_from`,
/// `active_theme`, `high_contrast`).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ThemePreferences {
    /// Dark, Light, System or Auto (clock).
    pub mode: ThemeMode,
    /// Local time the light theme starts in Auto (clock).
    pub light_from: NaiveTime,
    /// Local time the dark theme starts in Auto (clock).
    pub dark_from: NaiveTime,
    /// The accessibility theme; overrides `mode`.
    pub high_contrast: bool,
    /// The accent preset.
    pub accent: AccentPreset,
    /// A theme loaded from a theme file; its schemes replace the generated ones.
    pub custom: Option<Arc<CustomTheme>>,
}

impl Default for ThemePreferences {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            light_from: DEFAULT_LIGHT_FROM,
            dark_from: DEFAULT_DARK_FROM,
            high_contrast: false,
            accent: AccentPreset::default(),
            custom: None,
        }
    }
}

impl ThemePreferences {
    /// The accent colour in force (the custom theme's, if it names one).
    pub fn accent_color(&self) -> Color {
        self.custom
            .as_ref()
            .and_then(|c| c.accent)
            .unwrap_or_else(|| self.accent.color())
    }

    /// The palette the preferences select at local time `now`, given what the OS reports.
    ///
    /// Pure; the systems use it, and the game uses it to build the first theme before any
    /// system has run.
    pub fn resolve_kind(&self, system: Option<ThemeKind>, now: NaiveTime) -> ThemeKind {
        if self.high_contrast {
            return ThemeKind::HighContrast;
        }
        match self.mode {
            ThemeMode::Dark => ThemeKind::Dark,
            ThemeMode::Light => ThemeKind::Light,
            ThemeMode::System => system.unwrap_or(ThemeKind::Dark),
            ThemeMode::AutoClock => theme_for(now, self.light_from, self.dark_from),
        }
    }

    /// The colour scheme these preferences select at `now`.
    pub fn scheme(&self, system: Option<ThemeKind>, now: NaiveTime) -> ColorScheme {
        self.scheme_of(self.resolve_kind(system, now))
    }

    /// The colour scheme of `kind`: the custom theme's when there is one, else generated.
    pub fn scheme_of(&self, kind: ThemeKind) -> ColorScheme {
        match &self.custom {
            Some(custom) => custom.scheme(kind).clone(),
            None => ColorScheme::for_kind(kind, self.accent_color()),
        }
    }
}
