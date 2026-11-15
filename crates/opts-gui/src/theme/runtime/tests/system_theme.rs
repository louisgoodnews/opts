//! Tests of the operating-system theme: window reports, missing reports and the dark default.

use super::*;

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
