//! Integration tests of the theme foundations (M-0061 part A): settings drive the theme, scale,
//! density and motion; the loading screen, HUD and stub screens carry no literal colours.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    settings::GameSettings,
    settings_theme::{lock_theme_switching, sync_theme_settings},
};
use opts_gui::prelude::*;
use opts_gui::theme::{
    motion::MotionPreference, Density, LayoutClass, ThemeKind, ThemeMode, ThemePreferences,
    ThemeSwitchLock, UiScalePercent,
};

fn app(settings: GameSettings) -> App {
    let mut app = App::new();
    let prefs = settings.theme_preferences();
    let theme = MaterialTheme::with_scheme(prefs.scheme(None, chrono::NaiveTime::MIN));
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(settings.clone())
        .insert_resource(theme)
        .insert_resource(prefs)
        .insert_resource(settings.ui_scale_percent())
        .insert_resource(settings.density)
        .insert_resource(MotionPreference::default())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_systems(
            PreUpdate,
            (sync_theme_settings, lock_theme_switching)
                .before(opts_gui::theme::runtime::resolve_theme),
        );
    app.update();
    app
}

#[test]
fn a_default_start_is_dark_comfortable_at_100_percent() {
    let app = app(GameSettings::default());
    let theme = app.world().resource::<MaterialTheme>();
    assert_eq!(theme.kind(), ThemeKind::Dark);
    assert_eq!(*app.world().resource::<Density>(), Density::Comfortable);
    assert_eq!(app.world().resource::<UiScale>().0, 1.0);
    assert_eq!(*app.world().resource::<LayoutClass>(), LayoutClass::Medium);
    assert!(!theme.motion.reduced_motion);
}

#[test]
fn changing_the_settings_changes_theme_scale_density_and_motion() {
    let mut app = app(GameSettings::default());
    {
        let mut s = app.world_mut().resource_mut::<GameSettings>();
        s.theme_mode = ThemeMode::Light;
        s.ui_scale = 150;
        s.density = Density::Dense;
        s.reduced_motion = true;
    }
    app.update();
    app.update();
    let w = app.world();
    assert_eq!(w.resource::<MaterialTheme>().kind(), ThemeKind::Light);
    assert_eq!(w.resource::<UiScale>().0, 1.5);
    assert_eq!(*w.resource::<Density>(), Density::Dense);
    assert!(w.resource::<MaterialTheme>().motion.reduced_motion);
    assert_eq!(w.resource::<UiScalePercent>().0, 150);
    // high contrast overrides the mode and is never turned off by it
    app.world_mut().resource_mut::<GameSettings>().high_contrast = true;
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<MaterialTheme>().kind(),
        ThemeKind::HighContrast
    );
}

#[test]
fn an_accent_preset_changes_the_primary_role() {
    let mut app = app(GameSettings::default());
    let before = app
        .world()
        .resource::<MaterialTheme>()
        .scheme
        .get(ColorRole::Primary);
    app.world_mut().resource_mut::<GameSettings>().active_theme = "Amber".into();
    app.update();
    app.update();
    let after = app
        .world()
        .resource::<MaterialTheme>()
        .scheme
        .get(ColorRole::Primary);
    assert_ne!(before, after);
}
mod literals;

mod switching;
