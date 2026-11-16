//! Automatic switching is locked in the game, and a theme inserted before the plugin is kept.

use super::*;

#[test]
fn automatic_switching_is_locked_while_loading_or_driving() {
    let mut app = app(GameSettings::default());
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    app.update();
    app.update();
    assert!(
        !app.world().resource::<ThemeSwitchLock>().0,
        "free in menus"
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    app.update();
    assert!(
        app.world().resource::<ThemeSwitchLock>().0,
        "locked in game"
    );
}

#[test]
fn a_theme_inserted_before_the_plugin_is_kept() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(MaterialTheme::light())
        .add_plugins(OptsGuiPlugin);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<MaterialTheme>().kind(),
        ThemeKind::Light
    );
    assert_eq!(
        app.world().resource::<ThemePreferences>().mode,
        ThemeMode::Light
    );
}
