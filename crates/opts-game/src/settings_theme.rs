//! Theme and interface part of [`GameSettings`]: parsing and writing the keys `theme_mode`,
//! `auto_light_from`, `auto_dark_from`, `high_contrast`, `active_theme`, `ui_scale`, `density`
//! and `reduced_motion`, the migration from the old `is_dark_theme` key, and the bridge to the
//! `opts-gui` resources.

use std::collections::BTreeMap;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use chrono::NaiveTime;
use opts_gui::theme::{
    mode::{format_time, parse_time, DEFAULT_DARK_FROM, DEFAULT_LIGHT_FROM},
    motion::MotionPreference,
    AccentPreset, Density, SystemColorScheme, ThemeKind, ThemeMode, ThemePreferences,
    ThemeSwitchLock, UiScalePercent,
};

use crate::app_state::AppState;
use crate::settings::GameSettings;

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

/// Whether the desktop asks for reduced motion (best effort, Linux/GNOME only; `false` when the
/// answer is not available). Read once at start-up. `OPTS_REDUCED_MOTION=1` forces it (tests,
/// other desktops).
pub fn os_reduced_motion_hint() -> bool {
    if std::env::var("OPTS_REDUCED_MOTION").is_ok_and(|v| v == "1" || v == "true") {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        // `enable-animations` is false when the user turned animations off.
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .output();
        if let Ok(out) = out {
            return String::from_utf8_lossy(&out.stdout).trim() == "false";
        }
    }
    false
}

/// The colour scheme a `gsettings` answer (`'prefer-dark'`, `'prefer-light'`, `'default'`) means;
/// `'default'` is "no preference", so nothing is reported (the theme then falls back to dark).
pub fn parse_gsettings_scheme(answer: &str) -> Option<ThemeKind> {
    match answer.trim().trim_matches('\'') {
        "prefer-dark" => Some(ThemeKind::Dark),
        "prefer-light" => Some(ThemeKind::Light),
        _ => None,
    }
}

/// Asks the desktop for its colour-scheme preference (best effort, Linux/GNOME `gsettings`).
/// Needed because winit does not report a window theme on every desktop (it reports none on
/// Wayland/GNOME at start-up).
pub fn os_color_scheme_hint() -> Option<ThemeKind> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok()?;
        parse_gsettings_scheme(&String::from_utf8_lossy(&out.stdout))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Polls the desktop's colour scheme on a worker thread while the *System* theme mode is active
/// and the window itself reports nothing: at once when the mode is chosen, then once a minute.
pub struct OsSchemePoll {
    timer: Timer,
    tx: Sender<Option<ThemeKind>>,
    rx: std::sync::Mutex<Receiver<Option<ThemeKind>>>,
    in_flight: bool,
}

impl Default for OsSchemePoll {
    fn default() -> Self {
        let (tx, rx) = channel();
        let mut timer = Timer::new(Duration::from_secs(60), TimerMode::Repeating);
        timer.set_elapsed(Duration::from_secs(60)); // poll immediately
        Self {
            timer,
            tx,
            rx: std::sync::Mutex::new(rx),
            in_flight: false,
        }
    }
}

/// Feeds the polled desktop preference into [`SystemColorScheme`].
pub fn poll_os_color_scheme(
    time: Res<Time<Real>>,
    prefs: Res<ThemePreferences>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut poll: Local<OsSchemePoll>,
    mut system: ResMut<SystemColorScheme>,
) {
    let answers: Vec<_> = poll
        .rx
        .lock()
        .map(|rx| rx.try_iter().collect())
        .unwrap_or_default();
    for answer in answers {
        poll.in_flight = false;
        if let Some(kind) = answer {
            if system.0 != Some(kind) {
                tracing::info!(?kind, "the desktop prefers this colour scheme (gsettings)");
                system.0 = Some(kind);
            }
        }
    }
    let window_reports = windows.iter().any(|w| w.window_theme.is_some());
    if prefs.mode != ThemeMode::System || window_reports {
        return;
    }
    if poll.timer.tick(time.delta()).just_finished() && !poll.in_flight {
        poll.in_flight = true;
        let tx = poll.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(os_color_scheme_hint());
        });
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
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn load(rows: &[(&str, &str)]) -> GameSettings {
        let conn = Connection::open_in_memory().expect("db");
        GameSettings::init_database(&conn).expect("schema");
        for (k, v) in rows {
            conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)",
                [k, v],
            )
            .expect("insert");
        }
        let mut s = GameSettings::default();
        s.apply_from_connection(&conn).expect("load");
        s
    }

    #[test]
    fn gsettings_answers_are_understood() {
        assert_eq!(
            parse_gsettings_scheme("'prefer-dark'\n"),
            Some(ThemeKind::Dark)
        );
        assert_eq!(
            parse_gsettings_scheme("'prefer-light'"),
            Some(ThemeKind::Light)
        );
        assert_eq!(
            parse_gsettings_scheme("'default'"),
            None,
            "no preference falls back"
        );
        assert_eq!(parse_gsettings_scheme(""), None);
    }

    #[test]
    fn neither_key_means_dark() {
        assert_eq!(load(&[]).theme_mode, ThemeMode::Dark);
    }

    #[test]
    fn the_legacy_flag_migrates_when_theme_mode_is_absent() {
        assert_eq!(
            load(&[("is_dark_theme", "true")]).theme_mode,
            ThemeMode::Dark
        );
        assert_eq!(
            load(&[("is_dark_theme", "false")]).theme_mode,
            ThemeMode::Light
        );
    }

    #[test]
    fn theme_mode_wins_over_the_legacy_flag_in_any_row_order() {
        for rows in [
            [("theme_mode", "auto"), ("is_dark_theme", "false")],
            [("is_dark_theme", "false"), ("theme_mode", "auto")],
        ] {
            assert_eq!(load(&rows).theme_mode, ThemeMode::AutoClock);
        }
    }

    #[test]
    fn garbage_values_fall_back_to_defaults() {
        let s = load(&[
            ("theme_mode", "sepia"),
            ("auto_light_from", "25:99"),
            ("ui_scale", "9000"),
            ("density", "tiny"),
        ]);
        assert_eq!(s.theme_mode, ThemeMode::Dark);
        assert_eq!(s.auto_light_from, "07:00");
        assert_eq!(s.ui_scale, 200, "out-of-range scale is clamped");
        assert_eq!(s.density, Density::Comfortable);
    }

    #[test]
    fn old_accent_names_keep_working_and_unknown_ones_fall_back() {
        for (name, preset) in [
            ("Default", AccentPreset::TransitBlue),
            ("Amber", AccentPreset::Amber),
            ("Teal", AccentPreset::Teal),
            ("Unknown", AccentPreset::TransitBlue),
        ] {
            assert_eq!(load(&[("active_theme", name)]).accent(), preset);
        }
    }

    #[test]
    fn preferences_carry_the_times() {
        let s = load(&[("auto_light_from", "6:30"), ("auto_dark_from", "21:00")]);
        let p = s.theme_preferences();
        assert_eq!(format_time(p.light_from), "06:30");
        assert_eq!(format_time(p.dark_from), "21:00");
    }
}
