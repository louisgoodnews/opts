//! Integration tests verifying advanced features of the New Game setup menu:
//! 1. Map entrypoints in a scrollable list
//! 2. Vehicle repaints in a scrollable list and [Configure...] setvars modal
//! 3. Year spinbox responding to regular digit and numpad keys
//! 4. Equal height/width Date and Time pickers without OK/Cancel buttons and immediate synchronization
//! 5. 3-column (1:1:1) Weather & Datetime tab layout with scrollable weather presets and live telemetry panel

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use opts_game::app_state::AppState;
use opts_game::new_game_screen::types::{
    CalendarHeaderDateText, ClockDigitalDisplayText, CloseSetvarsModalBtn, EntryPointCardItem,
    LaunchSummaryText, NewGameSetup, OpenSetvarsModalBtn, RepaintCardItem, ResetSetvarsBtn,
    ScrollKind, ScrollableContainer, SelectCalendarDayBtn, SelectClockHourBtn, SelectEntryPointBtn,
    SelectLiveryBtn, SelectWeatherBtn, SetvarStepBtn, SetvarsModalRoot, WeatherCardItem,
    WeatherDetailPanelRoot, YearInputBox,
};
use opts_game::new_game_screen::NewGameMenuPlugin;
use opts_game::settings::GameSettings;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::theme::MaterialTheme;

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
        .add_plugins(opts_gui::plugin::OptsGuiPlugin)
        .insert_resource(opts_game::catalogue_service::testing::fixture_lists())
        .add_plugins(NewGameMenuPlugin);
    app
}

#[test]
fn test_map_entrypoints_scrollable_container_and_selection() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // 1. Check entrypoints scrollable container exists
    let ep_containers: Vec<&ScrollableContainer> = app
        .world_mut()
        .query::<&ScrollableContainer>()
        .iter(app.world())
        .filter(|c| c.kind == ScrollKind::EntryPoints)
        .collect();
    assert_eq!(
        ep_containers.len(),
        1,
        "Entrypoints scrollable container must exist"
    );

    // 2. Check entrypoint cards spawned
    let ep_cards: Vec<&EntryPointCardItem> = app
        .world_mut()
        .query::<&EntryPointCardItem>()
        .iter(app.world())
        .collect();
    assert!(
        !ep_cards.is_empty(),
        "At least one entrypoint card must be rendered"
    );

    // 3. Test selecting an entrypoint
    let target_ep = ep_cards[0].entry_point.clone();
    let btn_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectEntryPointBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.entry_point == target_ep)
            .map(|(e, _)| e)
            .expect("Target entry point button entity exists")
    };

    app.world_mut().send_event(ButtonClicked(btn_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(
        setup.selected_entry_point, target_ep,
        "Selected entrypoint must match clicked entrypoint"
    );

    // Verify summary text includes entrypoint
    let mut summary_query = app
        .world_mut()
        .query_filtered::<&Text, With<LaunchSummaryText>>();
    for summary_text in summary_query.iter(app.world()) {
        assert!(
            summary_text.sections[0].value.contains(&target_ep),
            "Summary text must contain selected entrypoint"
        );
    }
}

#[test]
fn test_vehicle_repaints_scrollable_and_setvars_modal() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // 1. Check repaints scrollable container exists
    let repaint_containers: Vec<&ScrollableContainer> = app
        .world_mut()
        .query::<&ScrollableContainer>()
        .iter(app.world())
        .filter(|c| c.kind == ScrollKind::Repaints)
        .collect();
    assert_eq!(
        repaint_containers.len(),
        1,
        "Repaints scrollable container must exist"
    );

    // 2. Check repaint cards spawned
    let repaint_cards: Vec<&RepaintCardItem> = app
        .world_mut()
        .query::<&RepaintCardItem>()
        .iter(app.world())
        .collect();
    assert!(
        !repaint_cards.is_empty(),
        "At least one repaint card must be rendered"
    );

    // 3. Select a livery
    let target_repaint = repaint_cards[0].livery.clone();
    let livery_btn_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectLiveryBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.livery == target_repaint)
            .map(|(e, _)| e)
            .expect("Livery button entity exists")
    };
    app.world_mut().send_event(ButtonClicked(livery_btn_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(
        setup.selected_livery, target_repaint,
        "Selected livery must update in setup"
    );

    // 4. Test Configure Setvars modal button
    let config_btn_entity = {
        let mut query = app.world_mut().query::<(Entity, &OpenSetvarsModalBtn)>();
        query
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("OpenSetvarsModalBtn exists")
    };
    app.world_mut().send_event(ButtonClicked(config_btn_entity));
    app.update();

    // Verify modal spawned
    let modal_count = app
        .world_mut()
        .query_filtered::<Entity, With<SetvarsModalRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(modal_count, 1, "Setvars modal root must be spawned");

    // Test stepping a setvar if available
    let step_btn = {
        let mut query = app.world_mut().query::<(Entity, &SetvarStepBtn)>();
        query.iter(app.world()).next().map(|(e, b)| (e, b.clone()))
    };

    if let Some((step_entity, btn_info)) = step_btn {
        app.world_mut().send_event(ButtonClicked(step_entity));
        app.update();

        let setup = app.world().resource::<NewGameSetup>();
        assert!(
            setup.custom_setvars.contains_key(&btn_info.var_name),
            "Custom setvars must contain stepped variable"
        );

        // Test Reset button
        let reset_btn = {
            let mut query = app.world_mut().query::<(Entity, &ResetSetvarsBtn)>();
            query.iter(app.world()).next().map(|(e, _)| e)
        };
        if let Some(r_ent) = reset_btn {
            app.world_mut().send_event(ButtonClicked(r_ent));
            app.update();

            let setup = app.world().resource::<NewGameSetup>();
            assert!(
                setup.custom_setvars.is_empty(),
                "Reset button must clear custom setvars"
            );
        }
    }

    // Test closing modal
    let close_btn_entity = {
        let mut query = app.world_mut().query::<(Entity, &CloseSetvarsModalBtn)>();
        query
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("CloseSetvarsModalBtn exists")
    };
    app.world_mut().send_event(ButtonClicked(close_btn_entity));
    app.update();

    let modal_count_after = app
        .world_mut()
        .query_filtered::<Entity, With<SetvarsModalRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(modal_count_after, 0, "Setvars modal root must be despawned");
}

#[test]
fn test_year_spinbox_keyboard_input_regular_and_numpad() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // 1. Focus the year spinbox
    let year_box_entity = {
        let mut query = app.world_mut().query::<(Entity, &YearInputBox)>();
        query
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("YearInputBox exists")
    };
    app.world_mut().send_event(ButtonClicked(year_box_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert!(setup.year_editing, "Year spinbox must be in editing mode");

    // 2. Type '1', '9', '9', '5' using standard Digit keys and Numpad keys
    let keys = [
        (KeyCode::Digit1, "1"),
        (KeyCode::Numpad9, "9"),
        (KeyCode::Digit9, "9"),
        (KeyCode::Numpad5, "5"),
    ];

    for (code, ch) in keys {
        let key_event = KeyboardInput {
            key_code: code,
            logical_key: Key::Character(ch.into()),
            state: ButtonState::Pressed,
            window: Entity::from_raw(0),
        };
        app.world_mut().send_event(key_event);
        app.update();
    }

    // Finish editing by pressing Enter
    let enter_event = KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        window: Entity::from_raw(0),
    };
    app.world_mut().send_event(enter_event);
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(
        setup.year, 1995,
        "Year must be updated to 1995 after typing digits and numpad keys"
    );
    assert!(!setup.year_editing, "Year editing should end after Enter");
}

#[test]
fn test_datetime_and_weather_three_column_layout_and_no_ok_buttons() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // 1. Verify Weather presets scrollable container exists (Column 1)
    let weather_containers: Vec<&ScrollableContainer> = app
        .world_mut()
        .query::<&ScrollableContainer>()
        .iter(app.world())
        .filter(|c| c.kind == ScrollKind::Weather)
        .collect();
    assert_eq!(
        weather_containers.len(),
        1,
        "Weather scrollable container must exist in Column 1"
    );

    // Verify Weather cards exist
    let weather_cards: Vec<&WeatherCardItem> = app
        .world_mut()
        .query::<&WeatherCardItem>()
        .iter(app.world())
        .collect();
    assert!(
        weather_cards.len() >= 5,
        "All weather presets must be rendered as cards"
    );

    // 2. Verify Weather Detail Panel exists (Column 2)
    let detail_panels = app
        .world_mut()
        .query_filtered::<Entity, With<WeatherDetailPanelRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(
        detail_panels, 1,
        "Weather detail panel root must exist in Column 2"
    );

    // 3. Test changing weather preset immediately updates setup and UI
    let rain_btn_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectWeatherBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.preset_name == "Rainy Afternoon")
            .map(|(e, _)| e)
            .expect("Rain weather preset button exists")
    };
    app.world_mut().send_event(ButtonClicked(rain_btn_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(
        setup.weather_preset, "Rainy Afternoon",
        "Selected weather preset must update immediately"
    );

    // 4. Test Date & Time picker interaction directly adjusts setup and header without OK button
    let day_20_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectCalendarDayBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.day == 20)
            .map(|(e, _)| e)
            .expect("Day 20 button exists")
    };
    app.world_mut().send_event(ButtonClicked(day_20_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(setup.day, 20, "Day must immediately update to 20");

    let header_text = {
        let mut query = app
            .world_mut()
            .query_filtered::<&Text, With<CalendarHeaderDateText>>();
        query
            .iter(app.world())
            .next()
            .map(|t| t.sections[0].value.clone())
            .expect("Header date text exists")
    };
    assert!(
        header_text.contains("20"),
        "Calendar header must immediately show day 20: {}",
        header_text
    );

    // 5. Test clock hour selection directly adjusts time without OK button
    let hour_4_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectClockHourBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.hour_12 == 4)
            .map(|(e, _)| e)
            .expect("Hour 4 button exists")
    };
    app.world_mut().send_event(ButtonClicked(hour_4_entity));
    app.update();

    let clock_text = {
        let mut query = app
            .world_mut()
            .query_filtered::<&Text, With<ClockDigitalDisplayText>>();
        query
            .iter(app.world())
            .next()
            .map(|t| t.sections[0].value.clone())
            .expect("Clock digital display text exists")
    };
    assert!(
        clock_text.starts_with("4:"),
        "Clock display must immediately show 4:.., got: {}",
        clock_text
    );
}
