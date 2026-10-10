//! Integration tests validating post-new-game menu loading, progressive synthesis,
//! telemetry HUD instantiation, and clean state teardown on exit.

use bevy::prelude::*;
use opts_bevy::PlayerVehicle;
use opts_game::{
    app_state::AppState,
    in_game::{
        CancelLoadingBtn, InGameCamera, InGameEntity, InGameHudRoot, InGamePlugin,
        InGameSessionInfo, LoadingScreenRoot, PauseMenuBtn, ProgressBarFill, ProgressPercentText,
        SimulationLoadState, SpeedometerText, StageIndicatorText, StatusTickerText,
    },
    new_game_screen::{LaunchGameBtn, NewGameMenuPlugin, NewGameSetup},
    settings::GameSettings,
};
use opts_gui::{interaction::events::ButtonClicked, prelude::*};

fn setup_test_app() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .insert_resource(opts_game::catalogue_service::testing::fixture_lists())
        .add_plugins(NewGameMenuPlugin)
        .add_plugins(InGamePlugin);

    app
}

#[test]
fn test_post_new_game_launch_spawns_loading_screen_and_widgets() {
    let mut app = setup_test_app();

    // Start in VehicleSelection (New Game Menu)
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Configure test parameters
    {
        let mut setup = app.world_mut().resource_mut::<NewGameSetup>();
        setup.selected_map = "Berlin-Spandau 1989".to_string();
        setup.selected_entry_point = "Freudstr. Terminal".to_string();
        setup.selected_vehicle = "MAN SD77".to_string();
        setup.selected_livery = "BVG Traditional Sandgelb".to_string();
        setup.weather_preset = "Clear Summer Day".to_string();
        setup.hour = 8;
        setup.minute = 30;
    }

    // Find Launch Simulation button and click it
    let launch_entity = app
        .world_mut()
        .query_filtered::<Entity, With<LaunchGameBtn>>()
        .iter(app.world())
        .next()
        .expect("LaunchGameBtn must be present in menu");

    app.world_mut().write_message(ButtonClicked(launch_entity));
    app.update();
    app.update();

    // State must now be InGame
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame,
        "Clicking launch button must transition to AppState::InGame"
    );

    // Verify LoadingScreenRoot hierarchy spawned
    let loading_count = app
        .world_mut()
        .query_filtered::<Entity, With<LoadingScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(
        loading_count, 1,
        "LoadingScreenRoot overlay must be spawned on entering InGame"
    );

    // Verify progress widgets
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<ProgressBarFill>>()
            .iter(app.world())
            .count(),
        1,
        "ProgressBarFill must be present"
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<ProgressPercentText>>()
            .iter(app.world())
            .count(),
        1,
        "ProgressPercentText must be present"
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<StatusTickerText>>()
            .iter(app.world())
            .count(),
        1,
        "StatusTickerText must be present"
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<StageIndicatorText>>()
            .iter(app.world())
            .count(),
        1,
        "StageIndicatorText must be present"
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<CancelLoadingBtn>>()
            .iter(app.world())
            .count(),
        1,
        "CancelLoadingBtn must be present on loading screen"
    );

    // Verify session info resource captured configured params
    let session = app.world().resource::<InGameSessionInfo>();
    assert_eq!(session.map_name, "Berlin-Spandau 1989");
    assert_eq!(session.entry_point, "Freudstr. Terminal");
    assert_eq!(session.vehicle_name, "MAN SD77");
    assert_eq!(session.livery_name, "BVG Traditional Sandgelb");
    assert_eq!(session.start_hour, 8);
    assert_eq!(session.start_minute, 30);
}

#[test]
fn test_loading_unresolvable_map_spawns_no_placeholder_geometry() {
    // A selection that resolves to no map folder: the pipeline must log an error and load
    // nothing (hermetic: independent of any OMSI installation on the machine). It must not substitute
    // fabricated geometry (boxes, planes) or a fake vehicle.
    let mut app = setup_test_app();
    app.world_mut().resource_mut::<NewGameSetup>().selected_map = "__no_such_map__".to_string();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();

    for _ in 0..12 {
        app.update();
    }

    let load_state = app.world().resource::<SimulationLoadState>();
    assert!(
        load_state.atmosphere_spawned,
        "lighting is not OMSI content and still spawns"
    );
    assert!(
        load_state.is_complete,
        "loading must finish even when no map is given"
    );

    let player_veh_count = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerVehicle>>()
        .iter(app.world())
        .count();
    assert_eq!(
        player_veh_count, 0,
        "no vehicle may be fabricated without a .bus file"
    );

    let cam_count = app
        .world_mut()
        .query_filtered::<Entity, With<InGameCamera>>()
        .iter(app.world())
        .count();
    assert_eq!(
        cam_count, 0,
        "no tracking camera without a vehicle to track"
    );

    assert_eq!(
        app.world().resource::<Assets<Mesh>>().len(),
        0,
        "no meshes may be created without real content"
    );
}

#[test]
fn test_loading_completion_activates_hud_and_dismisses_overlay() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();

    // Advance until complete and HUD is instantiated
    for _ in 0..15 {
        app.update();
    }

    let load_state = app.world().resource::<SimulationLoadState>();
    assert!(load_state.is_complete, "Loading should reach completion");
    assert!(load_state.hud_spawned, "Driving HUD should be spawned");

    // LoadingScreenRoot must be despawned
    let loading_count = app
        .world_mut()
        .query_filtered::<Entity, With<LoadingScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(
        loading_count, 0,
        "Loading screen overlay must be despawned once simulation commences"
    );

    // InGameHudRoot must now exist
    let hud_count = app
        .world_mut()
        .query_filtered::<Entity, With<InGameHudRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(hud_count, 1, "InGameHudRoot overlay must be present");

    // Speedometer and PauseMenuBtn must exist
    let speed_count = app
        .world_mut()
        .query_filtered::<Entity, With<SpeedometerText>>()
        .iter(app.world())
        .count();
    assert_eq!(speed_count, 1, "Speedometer readout must be present");

    let pause_count = app
        .world_mut()
        .query_filtered::<Entity, With<PauseMenuBtn>>()
        .iter(app.world())
        .count();
    assert_eq!(
        pause_count, 1,
        "Pause / Exit to Menu button must be present"
    );
}

#[test]
fn test_cancel_loading_returns_to_main_menu_with_cleanup() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();

    let cancel_btn = app
        .world_mut()
        .query_filtered::<Entity, With<CancelLoadingBtn>>()
        .iter(app.world())
        .next()
        .expect("CancelLoadingBtn must exist on loading screen");

    app.world_mut().write_message(ButtonClicked(cancel_btn));
    app.update();
    app.update();

    // State must be MainMenu
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu,
        "Clicking cancel must return to AppState::MainMenu"
    );

    // All InGameEntity instances must be cleanly despawned
    let leftover_entities = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(
        leftover_entities, 0,
        "All InGameEntity instances must be destroyed on exiting InGame"
    );
}

#[test]
fn test_escape_key_in_game_returns_to_main_menu() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();

    // Simulate pressing Escape key
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu,
        "Pressing Escape in-game must return to AppState::MainMenu"
    );

    let leftover_entities = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(
        leftover_entities, 0,
        "InGameEntity instances must be zero after Escape exit"
    );
}
