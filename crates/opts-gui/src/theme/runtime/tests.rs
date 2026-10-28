//! Unit tests for `super` (`runtime.rs`).

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
fn the_window_theme_is_read_into_the_system_scheme() {
    let mut app = App::new();
    app.init_resource::<SystemColorScheme>()
        .add_message::<WindowThemeChanged>()
        .add_systems(Update, track_system_theme);
    let window = app
        .world_mut()
        .spawn((
            Window {
                window_theme: Some(WindowTheme::Light),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().resource::<SystemColorScheme>().0,
        Some(ThemeKind::Light)
    );
    // a change message wins over the stored value
    app.world_mut().write_message(WindowThemeChanged {
        window,
        theme: WindowTheme::Dark,
    });
    app.update();
    assert_eq!(
        app.world().resource::<SystemColorScheme>().0,
        Some(ThemeKind::Dark)
    );
}

#[test]
fn a_window_that_reports_nothing_leaves_the_scheme_unknown() {
    let mut app = App::new();
    app.init_resource::<SystemColorScheme>()
        .add_message::<WindowThemeChanged>()
        .add_systems(Update, track_system_theme);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    assert_eq!(app.world().resource::<SystemColorScheme>().0, None);
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
