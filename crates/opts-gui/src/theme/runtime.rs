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
            system.0 = Some(kind);
        }
    }
}

/// Applies the theme the preferences select: immediately after an explicit change, held back
/// while a modal is open or the lock is set when the change is automatic.
#[allow(clippy::too_many_arguments)]
pub fn resolve_theme(
    time: Res<Time<Real>>,
    prefs: Res<ThemePreferences>,
    system: Res<SystemColorScheme>,
    clock: Res<ThemeClock>,
    lock: Res<ThemeSwitchLock>,
    modals: Query<(), With<ModalRoot>>,
    mut runtime: ResMut<ThemeRuntime>,
    mut theme: ResMut<MaterialTheme>,
) {
    let explicit = prefs.is_changed();
    let tick = runtime.timer.tick(time.delta()).just_finished();
    // Cheap early-out: nothing could have changed since the last evaluation.
    if !(explicit || tick || system.is_changed() || !runtime.checked || runtime.pending) {
        return;
    }
    runtime.checked = true;
    let wanted = prefs.resolve_kind(system.0, clock.now());
    if !explicit && wanted == theme.scheme.kind {
        runtime.pending = false;
        return;
    }
    if !explicit && (lock.0 || !modals.is_empty()) {
        runtime.pending = true; // retried every frame until the obstacle is gone
        return;
    }
    runtime.pending = false;
    let scheme = prefs.scheme_of(wanted);
    if theme.scheme != scheme {
        theme.scheme = scheme;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("time")
    }

    #[test]
    fn resolve_kind_follows_mode_system_and_clock() {
        let mut p = ThemePreferences::default();
        assert_eq!(p.resolve_kind(None, t(12, 0)), ThemeKind::Dark);
        p.mode = ThemeMode::Light;
        assert_eq!(p.resolve_kind(None, t(3, 0)), ThemeKind::Light);
        p.mode = ThemeMode::System;
        assert_eq!(p.resolve_kind(None, t(12, 0)), ThemeKind::Dark, "fallback");
        assert_eq!(
            p.resolve_kind(Some(ThemeKind::Light), t(12, 0)),
            ThemeKind::Light
        );
        p.mode = ThemeMode::AutoClock;
        assert_eq!(p.resolve_kind(None, t(12, 0)), ThemeKind::Light);
        assert_eq!(p.resolve_kind(None, t(23, 0)), ThemeKind::Dark);
        p.high_contrast = true;
        assert_eq!(p.resolve_kind(None, t(12, 0)), ThemeKind::HighContrast);
    }

    fn app(now: std::sync::Arc<std::sync::Mutex<NaiveTime>>, mode: ThemeMode) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::ZERO,
            ))
            .init_resource::<SystemColorScheme>()
            .init_resource::<ThemeSwitchLock>()
            .init_resource::<ThemeRuntime>()
            .insert_resource(ThemeClock::new(move || {
                now.lock().map(|g| *g).unwrap_or(NaiveTime::MIN)
            }))
            .insert_resource(ThemePreferences {
                mode,
                ..Default::default()
            })
            .insert_resource(MaterialTheme::dark())
            .add_systems(Update, resolve_theme);
        app
    }

    fn minute(app: &mut App) {
        let strategy = |app: &mut App, d: Duration| {
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(d));
        };
        strategy(app, Duration::from_secs(61));
        app.update();
        strategy(app, Duration::ZERO);
    }

    #[test]
    fn auto_clock_switches_at_the_boundary_once_per_minute() {
        let now = Arc::new(std::sync::Mutex::new(t(6, 59)));
        let mut app = app(now.clone(), ThemeMode::AutoClock);
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Dark
        );
        if let Ok(mut g) = now.lock() {
            *g = t(7, 0);
        }
        app.update();
        // not re-evaluated before the minute is over
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Dark
        );
        minute(&mut app);
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Light
        );
    }

    #[test]
    fn an_automatic_switch_waits_for_the_lock_and_for_modals() {
        let now = Arc::new(std::sync::Mutex::new(t(6, 59)));
        let mut app = app(now.clone(), ThemeMode::AutoClock);
        app.update();
        if let Ok(mut g) = now.lock() {
            *g = t(7, 30);
        }
        app.world_mut().resource_mut::<ThemeSwitchLock>().0 = true;
        minute(&mut app);
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Dark
        );
        app.world_mut().resource_mut::<ThemeSwitchLock>().0 = false;
        let modal = app.world_mut().spawn(ModalRoot).id();
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Dark
        );
        app.world_mut().despawn(modal);
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Light
        );
    }

    #[test]
    fn an_explicit_choice_applies_at_once_even_when_locked() {
        let now = Arc::new(std::sync::Mutex::new(t(12, 0)));
        let mut app = app(now, ThemeMode::Dark);
        app.world_mut().resource_mut::<ThemeSwitchLock>().0 = true;
        app.update();
        app.world_mut().resource_mut::<ThemePreferences>().mode = ThemeMode::Light;
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Light
        );
        app.world_mut()
            .resource_mut::<ThemePreferences>()
            .high_contrast = true;
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::HighContrast
        );
    }

    #[test]
    fn the_system_theme_is_followed_and_defaults_to_dark() {
        let now = Arc::new(std::sync::Mutex::new(t(12, 0)));
        let mut app = app(now, ThemeMode::System);
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Dark
        );
        app.world_mut().resource_mut::<SystemColorScheme>().0 = Some(ThemeKind::Light);
        app.update();
        assert_eq!(
            app.world().resource::<MaterialTheme>().kind(),
            ThemeKind::Light
        );
    }
}
