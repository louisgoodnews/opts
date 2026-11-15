//! Unit tests for `super` (`runtime.rs`).

use super::*;
use crate::theme::mode::ThemeMode;

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

mod switching;
mod system_theme;
