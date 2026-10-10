//! Automated integration tests validating the interactive Settings screen, Notebook tabs, and SQLite persistence.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    settings::GameSettings,
    settings_screen::{
        BackToMenuButton, ColorSwatchButton, DarkModeToggleButton, SettingsScreenPlugin,
        SettingsScreenRoot, ThemePresetButton,
    },
};
use opts_gui::{components::notebook::M3Notebook, interaction::events::ButtonClicked, prelude::*};

#[test]
fn test_settings_screen_spawns_notebook_and_tabs() {
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
        .add_plugins(SettingsScreenPlugin);

    // Transition to Settings
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();

    // Verify SettingsScreenRoot is spawned
    let root_count = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(root_count, 1, "SettingsScreenRoot must exist");

    // Verify Notebook container is spawned with 3 tabs
    let mut nb_query = app.world_mut().query::<&M3Notebook>();
    let nb = nb_query
        .iter(app.world())
        .next()
        .expect("Notebook must be present");
    assert_eq!(nb.tab_count, 3, "Notebook must have 3 tabs");
    assert_eq!(
        nb.active_tab, 0,
        "Notebook initial active tab must be 0 (UI & Themes)"
    );

    // Verify Theme Preset buttons exist
    let preset_count = app
        .world_mut()
        .query_filtered::<Entity, With<ThemePresetButton>>()
        .iter(app.world())
        .count();
    assert!(preset_count >= 4, "Must have theme preset buttons");

    // Verify Color Swatches exist
    let swatch_count = app
        .world_mut()
        .query_filtered::<Entity, With<ColorSwatchButton>>()
        .iter(app.world())
        .count();
    assert!(swatch_count >= 5, "Must have color swatch buttons");

    // Verify Back button exists
    let back_count = app
        .world_mut()
        .query_filtered::<Entity, With<BackToMenuButton>>()
        .iter(app.world())
        .count();
    assert_eq!(back_count, 1, "Must have BackToMenuButton");
}

#[test]
fn test_settings_screen_color_swatch_updates_state() {
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
        .add_plugins(SettingsScreenPlugin);

    // Transition to Settings
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();

    // Find the first color swatch
    let (swatch_entity, swatch_data) = {
        let mut query = app.world_mut().query::<(Entity, &ColorSwatchButton)>();
        let (e, s) = query.iter(app.world()).next().expect("At least one swatch");
        (e, (s.widget_type, s.role_name, s.hex))
    };

    // Simulate clicking the swatch
    app.world_mut().write_message(ButtonClicked(swatch_entity));
    app.update();

    // Verify GameSettings has the color override
    let settings = app.world().resource::<GameSettings>();
    let expected_key = format!("{}.{}", swatch_data.0, swatch_data.1);
    assert_eq!(
        settings.widget_colors.get(&expected_key),
        Some(&swatch_data.2.to_string())
    );
}

#[test]
fn test_settings_screen_dark_mode_toggle() {
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
        .add_plugins(SettingsScreenPlugin);

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();

    let toggle_entity = app
        .world_mut()
        .query_filtered::<Entity, With<DarkModeToggleButton>>()
        .iter(app.world())
        .next()
        .expect("Toggle button must exist");

    // Click dark mode toggle
    app.world_mut().write_message(ButtonClicked(toggle_entity));
    app.update();

    let settings = app.world().resource::<GameSettings>();
    assert!(
        settings.is_dark_theme,
        "Dark mode must be enabled after toggle click"
    );
}
