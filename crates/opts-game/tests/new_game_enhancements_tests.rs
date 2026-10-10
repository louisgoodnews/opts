//! Integration tests verifying searchbars, Material date/time pickers, scrollbars,
//! floating scroll-to-top buttons, selection summaries on all tabs, and clean entrypoints.

use bevy::prelude::*;
use opts_game::app_state::AppState;
use opts_game::new_game_screen::types::{
    AdjustMinuteBtn, CalendarHeaderDateText, CatalogueSearchState, ClockDigitalDisplayText,
    LaunchSummaryText, MapCardItem, NewGameSetup, NextMonthBtn, ScrollKind, ScrollToTopBtn,
    ScrollableContainer, ScrollbarThumb, ScrollbarTrack, SearchInputBox, SelectCalendarDayBtn,
    SelectClockHourBtn, SelectClockMeridianBtn, SelectMapBtn, TabSummaryText,
};
use opts_game::new_game_screen::NewGameMenuPlugin;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::theme::MaterialTheme;

use opts_game::settings::GameSettings;

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

fn catalogue_from_fixture() -> opts_game::new_game_screen::types::GameCatalogue {
    opts_game::catalogue_service::testing::fixture_lists()
}

#[test]
fn test_map_entrypoints_come_from_discovered_global_cfg() {
    let catalogue = catalogue_from_fixture();

    let spandau = catalogue
        .maps
        .iter()
        .find(|m| m.id == "Berlin-Spandau")
        .expect("Berlin-Spandau in catalogue");
    assert_eq!(spandau.entry_points.len(), 2, "{:?}", spandau.entry_points);
    assert!(spandau.entry_points.iter().any(|ep| ep.contains("Rathaus")));
    for ep in &spandau.entry_points {
        assert!(
            !ep.chars().all(|c| c.is_ascii_digit()),
            "Entrypoint name cannot be just digits: {ep}"
        );
    }

    let grundorf = catalogue
        .maps
        .iter()
        .find(|m| m.id == "Grundorf")
        .expect("Grundorf in catalogue");
    assert!(
        grundorf
            .entry_points
            .iter()
            .any(|ep| ep.contains("Bauernhof")),
        "{:?}",
        grundorf.entry_points
    );
}

#[test]
fn test_searchbars_and_filtering_maps_and_vehicles() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Check searchbar UI components spawned
    let map_search_boxes = app
        .world_mut()
        .query_filtered::<&SearchInputBox, ()>()
        .iter(app.world())
        .filter(|b| b.kind == ScrollKind::Maps)
        .count();
    assert_eq!(map_search_boxes, 1, "Map search box must be spawned");

    let veh_search_boxes = app
        .world_mut()
        .query_filtered::<&SearchInputBox, ()>()
        .iter(app.world())
        .filter(|b| b.kind == ScrollKind::Vehicles)
        .count();
    assert_eq!(veh_search_boxes, 1, "Vehicle search box must be spawned");

    // Filter maps by "Spandau"
    {
        let mut search = app.world_mut().resource_mut::<CatalogueSearchState>();
        search.map_query = "Spandau".to_string();
    }
    app.update();

    // Verify filtering applied
    let mut map_query = app.world_mut().query::<(&MapCardItem, &Node)>();
    let mut matching_visible = 0;
    let mut non_matching_hidden = 0;
    for (card, style) in map_query.iter(app.world()) {
        if card.search_text.contains("spandau") {
            assert_eq!(style.display, Display::Flex);
            matching_visible += 1;
        } else {
            assert_eq!(style.display, Display::None);
            non_matching_hidden += 1;
        }
    }
    assert!(matching_visible > 0, "Spandau map must be visible");
    assert!(non_matching_hidden > 0, "Non-matching maps must be hidden");

    // Clear map search and verify all become visible
    {
        let mut search = app.world_mut().resource_mut::<CatalogueSearchState>();
        search.map_query.clear();
    }
    app.update();

    let mut map_query = app.world_mut().query::<(&MapCardItem, &Node)>();
    for (_card, style) in map_query.iter(app.world()) {
        assert_eq!(
            style.display,
            Display::Flex,
            "All maps visible after clearing search"
        );
    }
}

#[test]
fn test_scrollbars_and_scroll_to_top_button() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Verify scrollbar track and thumb exist
    let track_count = app
        .world_mut()
        .query_filtered::<&ScrollbarTrack, ()>()
        .iter(app.world())
        .count();
    assert!(
        track_count >= 2,
        "Scrollbar tracks must exist for Maps and Vehicles"
    );

    let thumb_count = app
        .world_mut()
        .query_filtered::<&ScrollbarThumb, ()>()
        .iter(app.world())
        .count();
    assert!(
        thumb_count >= 2,
        "Scrollbar thumbs must exist for Maps and Vehicles"
    );

    // Verify scroll to top button
    let fab_entity = {
        let mut query = app.world_mut().query::<(Entity, &ScrollToTopBtn)>();
        query
            .iter(app.world())
            .find(|(_, btn)| btn.kind == ScrollKind::Maps)
            .map(|(e, _)| e)
            .expect("Scroll to top button for maps must exist")
    };

    // Simulate scrolled container
    {
        let mut query = app.world_mut().query::<&mut ScrollableContainer>();
        for mut c in query.iter_mut(app.world_mut()) {
            if c.kind == ScrollKind::Maps {
                c.scroll_y = 250.0;
            }
        }
    }

    // Click floating action button
    app.world_mut().write_message(ButtonClicked(fab_entity));
    app.update();

    // Verify container scroll reset to 0.0
    let mut query = app.world_mut().query::<&ScrollableContainer>();
    for c in query.iter(app.world()) {
        if c.kind == ScrollKind::Maps {
            assert_eq!(c.scroll_y, 0.0, "Scroll to top must reset scroll_y to 0.0");
        }
    }
}

#[test]
fn test_material_date_picker_screenshot_1_interaction() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Verify header formatted date exists
    let header_entity = {
        let mut query = app.world_mut().query::<(Entity, &CalendarHeaderDateText)>();
        query
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("Header date text exists")
    };
    let initial_header_text = app.world().get::<Text>(header_entity).unwrap().0.clone();
    assert!(
        !initial_header_text.is_empty(),
        "Header date string must not be empty"
    );

    // Click a day button (e.g. day 15)
    let day_15_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectCalendarDayBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.day == 15)
            .map(|(e, _)| e)
            .expect("Day 15 button exists")
    };
    app.world_mut().write_message(ButtonClicked(day_15_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(setup.day, 15, "Selected day must update to 15");

    let updated_header_text = app.world().get::<Text>(header_entity).unwrap().0.clone();
    assert!(
        updated_header_text.contains("15"),
        "Header must reflect selected day 15: {}",
        updated_header_text
    );

    // Click NextMonthBtn
    let next_month_entity = {
        let mut query = app.world_mut().query::<(Entity, &NextMonthBtn)>();
        query
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("NextMonthBtn exists")
    };
    let month_before = app.world().resource::<NewGameSetup>().month;
    app.world_mut()
        .write_message(ButtonClicked(next_month_entity));
    app.update();

    let month_after = app.world().resource::<NewGameSetup>().month;
    assert_eq!(month_after, (month_before % 12) + 1, "Month must advance");
}

#[test]
fn test_material_clock_picker_screenshot_2_interaction() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Verify digital display text exists
    let clock_entity = {
        let mut query = app
            .world_mut()
            .query::<(Entity, &ClockDigitalDisplayText)>();
        query
            .iter(app.world())
            .next()
            .map(|(e, _)| e)
            .expect("Clock digital display exists")
    };

    // Click hour 7 button
    let hour_7_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectClockHourBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.hour_12 == 7)
            .map(|(e, _)| e)
            .expect("Hour 7 button exists")
    };
    app.world_mut().write_message(ButtonClicked(hour_7_entity));
    app.update();

    let clock_text = app.world().get::<Text>(clock_entity).unwrap().0.clone();
    assert!(
        clock_text.starts_with("7:"),
        "Clock display must show 7:.., got: {}",
        clock_text
    );

    // Click PM meridian button
    let pm_entity = {
        let mut query = app.world_mut().query::<(Entity, &SelectClockMeridianBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.is_pm)
            .map(|(e, _)| e)
            .expect("PM button exists")
    };
    app.world_mut().write_message(ButtonClicked(pm_entity));
    app.update();

    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(setup.hour, 19, "7 PM in 24h format must be 19");

    // Click minute adjustment (+15m)
    let min_plus_15 = {
        let mut query = app.world_mut().query::<(Entity, &AdjustMinuteBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.minute_offset == 15)
            .map(|(e, _)| e)
            .expect("+15m button exists")
    };
    let min_before = app.world().resource::<NewGameSetup>().minute;
    app.world_mut().write_message(ButtonClicked(min_plus_15));
    app.update();

    let min_after = app.world().resource::<NewGameSetup>().minute;
    assert_eq!(
        min_after,
        (min_before + 15) % 60,
        "Minute must advance by 15m"
    );
}

#[test]
fn test_tab_summary_cards_synchronize_across_all_tabs() {
    let mut app = setup_test_app();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    app.update();

    // Verify TabSummaryText entities exist (at least 3 across the panels)
    let tab_summary_count = app
        .world_mut()
        .query_filtered::<Entity, With<TabSummaryText>>()
        .iter(app.world())
        .count();
    assert!(
        tab_summary_count >= 3,
        "TabSummaryText cards must exist in all tabs (found {})",
        tab_summary_count
    );

    // Change map
    let spandau_btn = {
        let mut query = app.world_mut().query::<(Entity, &SelectMapBtn)>();
        query
            .iter(app.world())
            .find(|(_, b)| b.map_id == "Berlin-Spandau")
            .map(|(e, _)| e)
            .expect("Berlin-Spandau button exists")
    };
    app.world_mut().write_message(ButtonClicked(spandau_btn));
    app.update();

    // Verify all tab summaries reflect "Berlin-Spandau"
    let mut tab_query = app
        .world_mut()
        .query_filtered::<&Text, With<TabSummaryText>>();
    for summary in tab_query.iter(app.world()) {
        assert!(
            summary.0.contains("Berlin-Spandau"),
            "Tab summary must be synchronized with selected map: {}",
            summary.0
        );
    }

    // Verify global footer summary also reflects "Berlin-Spandau"
    let mut footer_query = app
        .world_mut()
        .query_filtered::<&Text, With<LaunchSummaryText>>();
    for footer in footer_query.iter(app.world()) {
        assert!(
            footer.0.contains("Berlin-Spandau"),
            "Footer summary must be synchronized with selected map: {}",
            footer.0
        );
    }
}
