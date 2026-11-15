//! Switching tests: the clock boundary, the lock and the modal hold-back, and explicit choices.

use super::*;

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
