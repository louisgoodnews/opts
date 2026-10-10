//! Automated integration tests validating the interactive Start New Game screen,
//! 3-tab Notebook layout, 30/70 two-frame layout, scrolling, and state transitions.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    new_game_screen::{
        AdjustTimeBtn, BackToMenuBtn, DepartureTimeDisplayText, GameCatalogue, LaunchGameBtn,
        LaunchSummaryText, MapDetailPanelRoot, NewGameMenuPlugin, NewGameScreenRoot, NewGameSetup,
        ScrollContent, ScrollDownBtn, ScrollKind, ScrollableContainer, SelectEntryPointBtn,
        SelectMapBtn, SelectVehicleBtn, SelectWeatherBtn, VehicleDetailPanelRoot,
    },
    settings::GameSettings,
};
use opts_gui::{components::notebook::M3Notebook, interaction::events::ButtonClicked, prelude::*};

fn setup_test_app() -> App {
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
        .insert_resource(opts_game::catalogue_service::testing::fixture_lists())
        .add_plugins(NewGameMenuPlugin);

    app
}

#[test]
fn test_new_game_screen_spawns_notebook_and_3_tabs_with_frames() {
    let mut app = setup_test_app();

    // Transition to VehicleSelection (Start New Game)
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Verify NewGameScreenRoot is spawned
    let root_count = app
        .world_mut()
        .query_filtered::<Entity, With<NewGameScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(root_count, 1, "NewGameScreenRoot must exist");

    // Verify Notebook container is spawned with 3 tabs
    let mut nb_query = app.world_mut().query::<&M3Notebook>();
    let nb = nb_query
        .iter(app.world())
        .next()
        .expect("M3Notebook must be present in the new game screen");
    assert_eq!(nb.tab_count, 3, "Notebook must have exactly 3 tabs");
    assert_eq!(
        nb.active_tab, 0,
        "Initial active tab must be 0 (Map Selection)"
    );

    // Verify all OMSI maps are spawned in the 30% scrolled list
    let (maps_len, vehs_len) = {
        let cat = app.world().resource::<GameCatalogue>();
        (cat.maps.len(), cat.vehicles.len())
    };

    let map_cards_count = app
        .world_mut()
        .query_filtered::<Entity, With<SelectMapBtn>>()
        .iter(app.world())
        .count();
    assert_eq!(
        map_cards_count, maps_len,
        "All OMSI maps must exist as selectable options in the left frame"
    );
    assert!(
        map_cards_count >= 40,
        "Expected at least 40 OMSI maps in the catalogue"
    );

    // Verify 70% Map Detail frame is spawned
    let map_detail_count = app
        .world_mut()
        .query_filtered::<Entity, With<MapDetailPanelRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(map_detail_count, 1, "70% Map Detail frame must exist");

    // Verify all playable vehicles are spawned in the 30% scrolled list
    let vehicle_cards_count = app
        .world_mut()
        .query_filtered::<Entity, With<SelectVehicleBtn>>()
        .iter(app.world())
        .count();
    assert_eq!(
        vehicle_cards_count, vehs_len,
        "All playable vehicles must exist as selectable options"
    );
    assert!(
        vehicle_cards_count >= 50,
        "Expected dozens of playable vehicles in the fleet"
    );

    // Verify 70% Vehicle Detail frame is spawned
    let veh_detail_count = app
        .world_mut()
        .query_filtered::<Entity, With<VehicleDetailPanelRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(veh_detail_count, 1, "70% Vehicle Detail frame must exist");

    // Verify Weather presets exist
    let weather_count = app
        .world_mut()
        .query_filtered::<Entity, With<SelectWeatherBtn>>()
        .iter(app.world())
        .count();
    assert!(weather_count >= 5, "Must have at least 5 weather presets");

    // Verify Back and Launch buttons exist
    let back_count = app
        .world_mut()
        .query_filtered::<Entity, With<BackToMenuBtn>>()
        .iter(app.world())
        .count();
    assert_eq!(back_count, 1, "Must have BackToMenuBtn");

    let launch_count = app
        .world_mut()
        .query_filtered::<Entity, With<LaunchGameBtn>>()
        .iter(app.world())
        .count();
    assert_eq!(launch_count, 1, "Must have LaunchGameBtn");
}

#[test]
fn test_map_selection_updates_setup_and_summary() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Find the Grundorf map button
    let grundorf_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectMapBtn)>();
        let mut found = None;
        for (e, btn) in query.iter(app.world()) {
            if btn.map_id == "Grundorf" {
                found = Some(e);
                break;
            }
        }
        found.expect("Grundorf map card must exist")
    };

    // Simulate clicking Grundorf
    app.world_mut().send_event(ButtonClicked(grundorf_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert!(
        setup.selected_map.contains("Grundorf"),
        "Selected map must be updated to Grundorf"
    );

    // Verify summary text reflects change
    let mut summary_query = app
        .world_mut()
        .query_filtered::<&Text, With<LaunchSummaryText>>();
    let summary_text = summary_query
        .iter(app.world())
        .next()
        .expect("Summary text exists");
    assert!(
        summary_text.sections[0].value.contains("Grundorf"),
        "Summary text must contain 'Grundorf'"
    );

    // Test clicking an entrypoint chip
    let ep_entity = {
        let mut ep_query = app.world_mut().query::<(Entity, &SelectEntryPointBtn)>();
        let (e, _) = ep_query
            .iter(app.world())
            .next()
            .expect("Entrypoint chip must exist");
        e
    };
    app.world_mut().send_event(ButtonClicked(ep_entity));
    app.update();

    let updated_setup = app.world().resource::<NewGameSetup>();
    assert!(
        !updated_setup.selected_entry_point.is_empty(),
        "Selected entry point must not be empty"
    );
}

#[test]
fn test_vehicle_selection_updates_setup_and_summary() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Find a vehicle button (e.g., MAN_D92 or any available vehicle)
    let (vehicle_entity, vehicle_id) = {
        let mut query = app.world_mut().query::<(Entity, &SelectVehicleBtn)>();
        let mut found = None;
        for (e, btn) in query.iter(app.world()) {
            if btn.vehicle_id.contains("D92") || btn.vehicle_id.contains("SD77") {
                found = Some((e, btn.vehicle_id.clone()));
                break;
            }
        }
        found.unwrap_or_else(|| {
            let (e, btn) = query
                .iter(app.world())
                .next()
                .expect("At least one vehicle");
            (e, btn.vehicle_id.clone())
        })
    };

    // Simulate clicking vehicle
    app.world_mut().send_event(ButtonClicked(vehicle_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert!(
        !setup.selected_vehicle.is_empty(),
        "Selected vehicle should be updated"
    );

    let veh_full_name = {
        let catalogue = app.world().resource::<GameCatalogue>();
        let veh_item = catalogue
            .vehicles
            .iter()
            .find(|v| v.id == vehicle_id)
            .expect("Vehicle in catalogue");
        veh_item.full_name.clone()
    };
    assert_eq!(setup.selected_vehicle, veh_full_name);

    // Verify summary text reflects change
    let mut summary_query = app
        .world_mut()
        .query_filtered::<&Text, With<LaunchSummaryText>>();
    let summary_text = summary_query
        .iter(app.world())
        .next()
        .expect("Summary text exists");
    assert!(
        summary_text.sections[0].value.contains(&veh_full_name),
        "Summary text must contain selected vehicle full name"
    );
}

#[test]
fn test_scroll_buttons_adjust_offset() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Find ScrollDownBtn for Maps
    let scroll_down_entity = {
        let mut query = app.world_mut().query::<(Entity, &ScrollDownBtn)>();
        let mut found = None;
        for (e, btn) in query.iter(app.world()) {
            if btn.kind == ScrollKind::Maps {
                found = Some(e);
                break;
            }
        }
        found.expect("ScrollDownBtn for Maps must exist")
    };

    app.world_mut()
        .send_event(ButtonClicked(scroll_down_entity));
    app.update();

    // Check scroll offset increased
    let scroll_y = {
        let mut container_query = app.world_mut().query::<&ScrollableContainer>();
        let container = container_query
            .iter(app.world())
            .find(|c| c.kind == ScrollKind::Maps)
            .expect("ScrollableContainer exists");
        assert!(
            container.scroll_y > 0.0,
            "Scroll down button should advance scroll_y offset"
        );
        container.scroll_y
    };

    // Check that inner ScrollContent received negative top style
    let mut content_query = app.world_mut().query::<(&ScrollContent, &Style)>();
    let (_, style) = content_query
        .iter(app.world())
        .find(|(c, _)| c.kind == ScrollKind::Maps)
        .expect("ScrollContent exists");
    assert_eq!(style.top, Val::Px(-scroll_y));
}

#[test]
fn test_weather_and_time_adjustment_updates_setup() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Find the Winter Snowfall weather preset
    let snow_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectWeatherBtn)>();
        let mut found = None;
        for (e, btn) in query.iter(app.world()) {
            if btn.preset_name == "Winter Snowfall" {
                found = Some(e);
                break;
            }
        }
        found.expect("Winter Snowfall preset must exist")
    };

    app.world_mut().send_event(ButtonClicked(snow_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(setup.weather_preset, "Winter Snowfall");

    // Adjust hour by +1h
    let plus_hour_entity = {
        let mut query = app.world_mut().query::<(Entity, &AdjustTimeBtn)>();
        let mut found = None;
        for (e, btn) in query.iter(app.world()) {
            if btn.hour_offset == 1 {
                found = Some(e);
                break;
            }
        }
        found.expect("+1h button must exist")
    };

    let hour_before = app.world().resource::<NewGameSetup>().hour;
    app.world_mut().send_event(ButtonClicked(plus_hour_entity));
    app.update();

    let hour_after = app.world().resource::<NewGameSetup>().hour;
    assert_eq!(hour_after, (hour_before + 1) % 24);

    // Verify departure time readout is updated
    let mut time_query = app
        .world_mut()
        .query_filtered::<&Text, With<DepartureTimeDisplayText>>();
    let time_text = time_query
        .iter(app.world())
        .next()
        .expect("Departure time text exists");
    let expected_str = format!("{:02}:30", hour_after);
    assert!(
        time_text.sections[0].value.contains(&expected_str),
        "Departure time display must show updated hour {:02}:30",
        hour_after
    );
}

#[test]
fn test_launch_simulation_transitions_to_in_game() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    let launch_entity = app
        .world_mut()
        .query_filtered::<Entity, With<LaunchGameBtn>>()
        .iter(app.world())
        .next()
        .expect("Launch button must exist");

    app.world_mut().send_event(ButtonClicked(launch_entity));
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame,
        "Clicking Launch button must transition to AppState::InGame"
    );
}

#[test]
fn test_back_button_returns_to_main_menu() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    let back_entity = app
        .world_mut()
        .query_filtered::<Entity, With<BackToMenuBtn>>()
        .iter(app.world())
        .next()
        .expect("Back button must exist");

    app.world_mut().send_event(ButtonClicked(back_entity));
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu,
        "Clicking Back button must transition to AppState::MainMenu"
    );
}

#[test]
fn test_escape_key_returns_to_main_menu() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    app.world_mut()
        .send_event(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu,
        "Pressing Escape must transition to AppState::MainMenu"
    );
}

#[test]
fn test_new_game_screen_cleanup_on_state_exit() {
    let mut app = setup_test_app();

    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    let root_count = app
        .world_mut()
        .query_filtered::<Entity, With<NewGameScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(root_count, 1);

    // Transition out of VehicleSelection
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    app.update();

    let root_count_after = app
        .world_mut()
        .query_filtered::<Entity, With<NewGameScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(
        root_count_after, 0,
        "NewGameScreenRoot entities must be despawned on state exit"
    );
}
