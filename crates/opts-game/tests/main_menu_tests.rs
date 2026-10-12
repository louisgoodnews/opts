//! Settings defaults and the theme change message (the menu itself is tested in `shell_tests.rs`).

use bevy::prelude::*;
use opts_game::{app_state::AppState, settings::GameSettings, shell::ShellPlugin};
use opts_gui::prelude::*;

#[test]
fn test_game_settings_defaults_and_toml_roundtrip() {
    let default_settings = GameSettings::default();
    assert!(default_settings.recent_map_path.is_none());
    assert_eq!(
        default_settings.theme_mode,
        opts_gui::theme::ThemeMode::Dark
    );

    let serialized = toml::to_string(&default_settings).expect("Serialization failed");
    let deserialized: GameSettings = toml::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(
        default_settings.recent_map_path,
        deserialized.recent_map_path
    );
    assert_eq!(default_settings.theme_mode, deserialized.theme_mode);
}

#[test]
fn test_theme_toggle_flips_color_scheme() {
    opts_game::isolation::isolate_user_dirs();
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::light())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin);

    assert!(!app.world().resource::<MaterialTheme>().scheme.is_dark());

    // Send ThemeChanged event
    app.world_mut()
        .write_message(ThemeChanged { is_dark: true });
    app.update();

    assert!(app.world().resource::<MaterialTheme>().scheme.is_dark());
}
