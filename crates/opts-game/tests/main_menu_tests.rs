//! Automated integration tests validating Main Menu states, components, and settings.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    menu::{
        entries::QuitButton,
        glass::GlassSurface,
        layout::{MainMenuRoot, MenuCard},
        theme_toggle::ThemeToggleBtn,
        MainMenuPlugin,
    },
    settings::GameSettings,
    stub_screens::StubScreensPlugin,
};
use opts_gui::prelude::*;

#[test]
fn test_game_settings_defaults_and_toml_roundtrip() {
    let default_settings = GameSettings::default();
    assert!(default_settings.recent_map_path.is_none());
    assert!(!default_settings.is_dark_theme);

    let serialized = toml::to_string(&default_settings).expect("Serialization failed");
    let deserialized: GameSettings = toml::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(
        default_settings.recent_map_path,
        deserialized.recent_map_path
    );
    assert_eq!(default_settings.is_dark_theme, deserialized.is_dark_theme);
}

#[test]
fn test_main_menu_hierarchy_and_glass_surfaces() {
    let mut app = App::new();

    // Headless testing setup
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::light())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(StubScreensPlugin);

    // Initial state transition from Loading to MainMenu
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    app.update();

    // Verify MainMenuRoot is spawned
    let mut root_query = app
        .world_mut()
        .query_filtered::<Entity, With<MainMenuRoot>>();
    let root_count = root_query.iter(app.world()).count();
    assert_eq!(root_count, 1, "Expected exactly 1 MainMenuRoot entity");

    // Verify MenuCard is spawned with GlassSurface
    let mut card_query = app
        .world_mut()
        .query_filtered::<(Entity, &GlassSurface), With<MenuCard>>();
    let card_count = card_query.iter(app.world()).count();
    assert_eq!(card_count, 1, "Expected exactly 1 MenuCard entity");

    // Verify Quit button exists
    let mut quit_query = app.world_mut().query_filtered::<Entity, With<QuitButton>>();
    let quit_count = quit_query.iter(app.world()).count();
    assert_eq!(quit_count, 1, "Expected exactly 1 QuitButton entity");

    // Verify ThemeToggle button exists
    let mut toggle_query = app
        .world_mut()
        .query_filtered::<Entity, With<ThemeToggleBtn>>();
    let toggle_count = toggle_query.iter(app.world()).count();
    assert_eq!(
        toggle_count, 1,
        "Expected exactly 1 ThemeToggle button entity"
    );

    // Verify Disabled button exists (Load Game)
    let mut disabled_query = app.world_mut().query_filtered::<Entity, With<Disabled>>();
    let disabled_count = disabled_query.iter(app.world()).count();
    assert!(
        disabled_count >= 1,
        "Expected at least 1 Disabled component on Load button"
    );
}

#[test]
fn test_app_state_cleanup_on_exit() {
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
        .add_plugins(MainMenuPlugin)
        .add_plugins(StubScreensPlugin);

    // Enter MainMenu
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    app.update();

    let root_count_before = app
        .world_mut()
        .query_filtered::<Entity, With<MainMenuRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(root_count_before, 1);

    // Transition to Settings stub
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();

    let root_count_after = app
        .world_mut()
        .query_filtered::<Entity, With<MainMenuRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(
        root_count_after, 0,
        "MainMenu entities must be cleaned up on state exit"
    );
}

#[test]
fn test_theme_toggle_flips_color_scheme() {
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
        .add_plugins(MainMenuPlugin);

    assert!(!app.world().resource::<MaterialTheme>().scheme.is_dark());

    // Send ThemeChanged event
    app.world_mut()
        .write_message(ThemeChanged { is_dark: true });
    app.update();

    assert!(app.world().resource::<MaterialTheme>().scheme.is_dark());
}

#[test]
fn test_escape_key_in_stub_screen_returns_to_main_menu() {
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
        .add_plugins(MainMenuPlugin)
        .add_plugins(StubScreensPlugin);

    // Enter Settings stub
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Settings
    );

    // Send KeyboardInput event for Escape
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
        });
    app.update(); // PreUpdate processes KeyboardInput, Update runs escape_to_menu_system
    app.update(); // StateTransition applies transition to MainMenu

    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
}
