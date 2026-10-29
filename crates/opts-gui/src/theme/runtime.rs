//! Choosing the active theme at run time: [`ThemePreferences`] in, [`MaterialTheme`] out.
//!
//! * `Dark` and `Light` are fixed; `System` follows the window's theme (`Window::window_theme`,
//!   `WindowThemeChanged`) and falls back to dark when the desktop reports nothing; `AutoClock`
//!   follows [`theme_for`] on the device's local time, re-evaluated **once per minute**.
//! * **High contrast** is a separate switch that overrides the mode and is never turned on
//!   automatically.
//! * An explicit change of the preferences applies at once. An **automatic** change (clock tick,
//!   OS change) is held back while a modal dialog is open or [`ThemeSwitchLock`] is set (the game
//!   sets it during loading and a drive) and happens when the obstacle is gone.

use std::sync::Arc;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowTheme, WindowThemeChanged};
use chrono::NaiveTime;

use super::accent::AccentPreset;
use super::color::ColorScheme;
use super::custom::CustomTheme;
use super::mode::{theme_for, ThemeKind, ThemeMode, DEFAULT_DARK_FROM, DEFAULT_LIGHT_FROM};
use super::theme::MaterialTheme;
use crate::components::modal::ModalRoot;

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

/// While `true`, automatic theme changes wait (set by the game during loading and a drive).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThemeSwitchLock(pub bool);

/// What the operating system reports (`None` until it says anything).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemColorScheme(pub Option<ThemeKind>);

/// The clock the Auto mode reads; replaceable in tests.
#[derive(Resource, Clone)]
pub struct ThemeClock(Arc<dyn Fn() -> NaiveTime + Send + Sync>);

impl ThemeClock {
    /// A clock that returns `f()`.
    pub fn new(f: impl Fn() -> NaiveTime + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    /// The device's local time of day (timezone and daylight saving from the OS).
    pub fn local() -> Self {
        Self::new(|| chrono::Local::now().time())
    }

    /// The current time of day.
    pub fn now(&self) -> NaiveTime {
        (self.0)()
    }
}

impl Default for ThemeClock {
    fn default() -> Self {
        Self::local()
    }
}

/// Bookkeeping of [`resolve_theme`].
#[derive(Resource, Debug)]
pub struct ThemeRuntime {
    timer: Timer,
    checked: bool,
    pending: bool,
}

impl Default for ThemeRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::from_secs(60), TimerMode::Repeating),
            checked: false,
            pending: false,
        }
    }
}

fn from_window_theme(theme: WindowTheme) -> ThemeKind {
    match theme {
        WindowTheme::Light => ThemeKind::Light,
        WindowTheme::Dark => ThemeKind::Dark,
    }
}

/// Tracks what the OS reports: the window's current theme and `WindowThemeChanged` messages.
pub fn track_system_theme(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut changes: MessageReader<WindowThemeChanged>,
    mut system: ResMut<SystemColorScheme>,
) {
    let mut reported = windows
        .single()
        .ok()
        .and_then(|w| w.window_theme)
        .map(from_window_theme);
    for change in changes.read() {
        reported = Some(from_window_theme(change.theme));
    }
    if let Some(kind) = reported {
        if system.0 != Some(kind) {
            tracing::info!(?kind, "the operating system reports its colour scheme");
            system.0 = Some(kind);
        }
    }
}

/// Parameters for [`resolve_theme`].
#[derive(SystemParam)]
pub struct ThemeResolveParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub prefs: Res<'w, ThemePreferences>,
    pub system: Res<'w, SystemColorScheme>,
    pub clock: Res<'w, ThemeClock>,
    pub lock: Res<'w, ThemeSwitchLock>,
    pub modals: Query<'w, 's, (), With<ModalRoot>>,
    pub runtime: ResMut<'w, ThemeRuntime>,
    pub theme: ResMut<'w, MaterialTheme>,
}

/// Applies the theme the preferences select: immediately after an explicit change, held back
/// while a modal is open or the lock is set when the change is automatic.
pub fn resolve_theme(mut params: ThemeResolveParams) {
    let explicit = params.prefs.is_changed();
    let tick = params
        .runtime
        .timer
        .tick(params.time.delta())
        .just_finished();
    // Cheap early-out: nothing could have changed since the last evaluation.
    if !(explicit
        || tick
        || params.system.is_changed()
        || !params.runtime.checked
        || params.runtime.pending)
    {
        return;
    }
    params.runtime.checked = true;
    let wanted = params
        .prefs
        .resolve_kind(params.system.0, params.clock.now());
    if !explicit && wanted == params.theme.scheme.kind {
        params.runtime.pending = false;
        return;
    }
    if !explicit && (params.lock.0 || !params.modals.is_empty()) {
        params.runtime.pending = true; // retried every frame until the obstacle is gone
        return;
    }
    params.runtime.pending = false;
    let scheme = params.prefs.scheme_of(wanted);
    if params.theme.scheme != scheme {
        params.theme.scheme = scheme;
    }
}

#[cfg(test)]
mod tests;
