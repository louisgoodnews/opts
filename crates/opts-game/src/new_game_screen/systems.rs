//! Event-handling systems and dynamic updates for the Start New Game screen.

use bevy::prelude::*;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::new_game_screen::live_weather::populate_weather_details_inner;
use crate::new_game_screen::map_panel::populate_map_details_inner;
use crate::new_game_screen::types::{
    AdjustTimeBtn, ApplyLiveWeatherBtn, BackToMenuBtn, DepartureTimeDisplayText,
    DeviceLocationWeather, GameCatalogue, LaunchGameBtn, LaunchSummaryText, MapDetailPanelRoot,
    NewGameScreenRoot, NewGameSetup, RefreshWeatherBtn, SelectEntryPointBtn, SelectLiveryBtn,
    SelectMapBtn, SelectVehicleBtn, SelectWeatherBtn, TabSummaryText, VehicleDetailPanelRoot,
    WeatherDetailPanelRoot,
};
use crate::new_game_screen::vehicle_panel::populate_vehicle_details_inner;

/// System returning to the main menu when the back button is clicked.
pub fn handle_back_button(
    mut events: MessageReader<ButtonClicked>,
    back_query: Query<&BackToMenuBtn>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        if back_query.contains(event.0) {
            tracing::info!("Returning to Main Menu from Start New Game");
            next_state.set(AppState::MainMenu);
        }
    }
}

/// System launching simulation when the launch button is clicked.
pub fn handle_launch_button(
    mut events: MessageReader<ButtonClicked>,
    launch_query: Query<&LaunchGameBtn>,
    setup: Res<NewGameSetup>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        if launch_query.contains(event.0) {
            tracing::info!(
                map = ?setup.selected_map,
                entry_point = ?setup.selected_entry_point,
                vehicle = ?setup.selected_vehicle,
                weather = ?setup.weather_preset,
                time = format!("{:02}:{:02}", setup.hour, setup.minute),
                "Launching simulation session with configured parameters"
            );
            next_state.set(AppState::InGame);
        }
    }
}

/// System updating selected map, updating list highlights, and repopulating the 70% right frame.
#[allow(clippy::too_many_arguments)]
pub fn handle_map_selection(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    map_btn_query: Query<&SelectMapBtn>,
    mut card_query: Query<(&SelectMapBtn, &mut BorderColor, &mut BackgroundColor)>,
    detail_panel_query: Query<Entity, With<MapDetailPanelRoot>>,
    catalogue: Res<GameCatalogue>,
    theme: Res<MaterialTheme>,
    mut setup: ResMut<NewGameSetup>,
    mut summary_query: Query<&mut Text, (With<LaunchSummaryText>, Without<TabSummaryText>)>,
    mut tab_summary_query: Query<&mut Text, (With<TabSummaryText>, Without<LaunchSummaryText>)>,
) {
    let mut selected_id = None;
    for event in events.read() {
        if let Ok(btn) = map_btn_query.get(event.0) {
            selected_id = Some(btn.map_id.clone());
        }
    }

    if let Some(map_id) = selected_id {
        if let Some(map) = catalogue
            .maps
            .iter()
            .find(|m| m.id == map_id || m.name == map_id || m.friendly_name == map_id)
        {
            setup.selected_map = map.friendly_name.clone();
            if let Some(first_ep) = map.entry_points.first() {
                setup.selected_entry_point = first_ep.clone();
            }
            tracing::info!(map = map.friendly_name, "Selected simulation map");

            // Update border & background on left list cards
            for (btn, mut border, mut bg) in &mut card_query {
                let is_selected = btn.map_id == map.id;
                *border = BorderColor(if is_selected {
                    theme.scheme.get(ColorRole::Primary)
                } else {
                    theme.scheme.get(ColorRole::OutlineVariant)
                });
                *bg = BackgroundColor(if is_selected {
                    theme.scheme.get(ColorRole::PrimaryContainer)
                } else {
                    theme.scheme.get(ColorRole::SurfaceContainerLow)
                });
            }

            // Repopulate 70% right frame details
            for detail_entity in &detail_panel_query {
                commands.entity(detail_entity).despawn_descendants();
                commands.entity(detail_entity).with_children(|right_col| {
                    populate_map_details_inner(right_col, &theme, map, &setup);
                });
            }

            update_all_summaries(&setup, &mut summary_query, &mut tab_summary_query);
        }
    }
}

/// System updating selected spawn entrypoint and chip styling.
pub fn handle_entrypoint_selection(
    mut events: MessageReader<ButtonClicked>,
    ep_btn_query: Query<&SelectEntryPointBtn>,
    mut chip_query: Query<(&SelectEntryPointBtn, &mut BorderColor, &mut BackgroundColor)>,
    theme: Res<MaterialTheme>,
    mut setup: ResMut<NewGameSetup>,
    mut summary_query: Query<&mut Text, (With<LaunchSummaryText>, Without<TabSummaryText>)>,
    mut tab_summary_query: Query<&mut Text, (With<TabSummaryText>, Without<LaunchSummaryText>)>,
) {
    let mut selected_ep = None;
    for event in events.read() {
        if let Ok(btn) = ep_btn_query.get(event.0) {
            selected_ep = Some(btn.entry_point.clone());
        }
    }

    if let Some(ep) = selected_ep {
        setup.selected_entry_point = ep.clone();
        tracing::info!(entrypoint = ep, "Selected starting spawn terminal");

        for (btn, mut border, mut bg) in &mut chip_query {
            let is_selected = btn.entry_point == ep;
            *border = BorderColor(if is_selected {
                theme.scheme.get(ColorRole::Primary)
            } else {
                theme.scheme.get(ColorRole::OutlineVariant)
            });
            *bg = BackgroundColor(if is_selected {
                theme.scheme.get(ColorRole::PrimaryContainer)
            } else {
                theme.scheme.get(ColorRole::SurfaceContainerHigh)
            });
        }

        update_all_summaries(&setup, &mut summary_query, &mut tab_summary_query);
    }
}

/// System updating selected vehicle, updating list highlights, and repopulating 70% right frame.
#[allow(clippy::too_many_arguments)]
pub fn handle_vehicle_selection(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    veh_btn_query: Query<&SelectVehicleBtn>,
    mut card_query: Query<(&SelectVehicleBtn, &mut BorderColor, &mut BackgroundColor)>,
    detail_panel_query: Query<Entity, With<VehicleDetailPanelRoot>>,
    catalogue: Res<GameCatalogue>,
    theme: Res<MaterialTheme>,
    mut setup: ResMut<NewGameSetup>,
    mut summary_query: Query<&mut Text, (With<LaunchSummaryText>, Without<TabSummaryText>)>,
    mut tab_summary_query: Query<&mut Text, (With<TabSummaryText>, Without<LaunchSummaryText>)>,
) {
    let mut selected_id = None;
    for event in events.read() {
        if let Ok(btn) = veh_btn_query.get(event.0) {
            selected_id = Some(btn.vehicle_id.clone());
        }
    }

    if let Some(veh_id) = selected_id {
        if let Some(veh) = catalogue
            .vehicles
            .iter()
            .find(|v| v.id == veh_id || v.model == veh_id || v.full_name == veh_id)
        {
            setup.selected_vehicle = veh.full_name.clone();
            setup.selected_livery = veh.default_livery.clone();
            tracing::info!(vehicle = veh.full_name, "Selected simulation vehicle");

            for (btn, mut border, mut bg) in &mut card_query {
                let is_selected = btn.vehicle_id == veh.id;
                *border = BorderColor(if is_selected {
                    theme.scheme.get(ColorRole::Primary)
                } else {
                    theme.scheme.get(ColorRole::OutlineVariant)
                });
                *bg = BackgroundColor(if is_selected {
                    theme.scheme.get(ColorRole::PrimaryContainer)
                } else {
                    theme.scheme.get(ColorRole::SurfaceContainerLow)
                });
            }

            for detail_entity in &detail_panel_query {
                commands.entity(detail_entity).despawn_descendants();
                commands.entity(detail_entity).with_children(|right_col| {
                    populate_vehicle_details_inner(right_col, &theme, veh, &setup);
                });
            }

            update_all_summaries(&setup, &mut summary_query, &mut tab_summary_query);
        }
    }
}

pub use crate::new_game_screen::scroll_systems::{handle_mouse_scroll, handle_scroll_buttons};

/// System updating selected vehicle repaint / livery.
pub fn handle_livery_selection(
    mut events: MessageReader<ButtonClicked>,
    query: Query<&SelectLiveryBtn>,
    mut card_query: Query<(&SelectLiveryBtn, &mut BorderColor, &mut BackgroundColor)>,
    theme: Res<MaterialTheme>,
    mut setup: ResMut<NewGameSetup>,
    mut summary_query: Query<&mut Text, (With<LaunchSummaryText>, Without<TabSummaryText>)>,
    mut tab_summary_query: Query<&mut Text, (With<TabSummaryText>, Without<LaunchSummaryText>)>,
) {
    let mut changed = false;
    for event in events.read() {
        if let Ok(btn) = query.get(event.0) {
            setup.selected_livery = btn.livery.clone();
            tracing::info!(livery = btn.livery, "Selected vehicle livery");
            changed = true;
        }
    }

    if changed {
        for (btn, mut border, mut bg) in &mut card_query {
            let is_selected = btn.livery == setup.selected_livery;
            *border = BorderColor(if is_selected {
                theme.scheme.get(ColorRole::Primary)
            } else {
                theme.scheme.get(ColorRole::OutlineVariant)
            });
            *bg = BackgroundColor(if is_selected {
                theme.scheme.get(ColorRole::PrimaryContainer)
            } else {
                theme.scheme.get(ColorRole::SurfaceContainerLow)
            });
        }
        update_all_summaries(&setup, &mut summary_query, &mut tab_summary_query);
    }
}

/// System updating selected weather preset and highlighting button.
#[allow(clippy::too_many_arguments)]
pub fn handle_weather_selection(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    query: Query<&SelectWeatherBtn>,
    mut preset_query: Query<(&SelectWeatherBtn, &mut BorderColor, &mut BackgroundColor)>,
    refresh_query: Query<&RefreshWeatherBtn>,
    apply_live_query: Query<&ApplyLiveWeatherBtn>,
    detail_panel_query: Query<Entity, With<WeatherDetailPanelRoot>>,
    theme: Res<MaterialTheme>,
    mut setup: ResMut<NewGameSetup>,
    live_weather: Res<DeviceLocationWeather>,
    mut summary_query: Query<&mut Text, (With<LaunchSummaryText>, Without<TabSummaryText>)>,
    mut tab_summary_query: Query<&mut Text, (With<TabSummaryText>, Without<LaunchSummaryText>)>,
) {
    let mut changed = false;
    for event in events.read() {
        if let Ok(btn) = query.get(event.0) {
            setup.weather_preset = btn.preset_name.to_string();
            tracing::info!(weather = btn.preset_name, "Selected weather preset");
            changed = true;
        } else if refresh_query.get(event.0).is_ok() {
            tracing::info!("Refreshed live device-location weather");
            changed = true;
        } else if apply_live_query.get(event.0).is_ok() {
            setup.weather_preset = "Real-Time Local Weather".to_string();
            tracing::info!("Applied real-time local weather to setup");
            changed = true;
        }
    }

    if changed {
        for (btn, mut border, mut bg) in &mut preset_query {
            let is_selected = btn.preset_name == setup.weather_preset;
            *border = BorderColor(if is_selected {
                theme.scheme.get(ColorRole::Primary)
            } else {
                theme.scheme.get(ColorRole::OutlineVariant)
            });
            *bg = BackgroundColor(if is_selected {
                theme.scheme.get(ColorRole::PrimaryContainer)
            } else {
                theme.scheme.get(ColorRole::SurfaceContainerLow)
            });
        }

        for detail_entity in &detail_panel_query {
            commands.entity(detail_entity).despawn_descendants();
            commands.entity(detail_entity).with_children(|center_col| {
                populate_weather_details_inner(center_col, &theme, &setup, &live_weather);
            });
        }

        update_all_summaries(&setup, &mut summary_query, &mut tab_summary_query);
    }
}

/// System adjusting departure time and updating display text.
#[allow(clippy::type_complexity)]
pub fn handle_time_adjustment(
    mut events: MessageReader<ButtonClicked>,
    query: Query<&AdjustTimeBtn>,
    mut setup: ResMut<NewGameSetup>,
    mut departure_query: Query<
        &mut Text,
        (
            With<DepartureTimeDisplayText>,
            Without<LaunchSummaryText>,
            Without<TabSummaryText>,
        ),
    >,
    mut summary_query: Query<
        &mut Text,
        (
            With<LaunchSummaryText>,
            Without<DepartureTimeDisplayText>,
            Without<TabSummaryText>,
        ),
    >,
    mut tab_summary_query: Query<
        &mut Text,
        (
            With<TabSummaryText>,
            Without<DepartureTimeDisplayText>,
            Without<LaunchSummaryText>,
        ),
    >,
) {
    let mut changed = false;
    for event in events.read() {
        if let Ok(btn) = query.get(event.0) {
            let mut new_hour = (setup.hour as i32) + btn.hour_offset;
            while new_hour < 0 {
                new_hour += 24;
            }
            setup.hour = (new_hour % 24) as u32;
            tracing::info!(hour = setup.hour, "Adjusted simulation start hour");
            changed = true;
        }
    }

    if changed {
        for mut text in departure_query.iter_mut() {
            text.0 = format!(
                "Departure Time: {:02}:{:02} (Date: {:04}-{:02}-{:02})",
                setup.hour, setup.minute, setup.year, setup.month, setup.day
            );
        }
        update_all_summaries(&setup, &mut summary_query, &mut tab_summary_query);
    }
}

/// Helper synchronizing both footer summary text and all tab summary cards.
pub fn update_all_summaries<
    F1: bevy::ecs::query::QueryFilter,
    F2: bevy::ecs::query::QueryFilter,
>(
    setup: &NewGameSetup,
    launch_summary_query: &mut Query<&mut Text, F1>,
    tab_summary_query: &mut Query<&mut Text, F2>,
) {
    let summary_str = format!(
        "Map: {} ({}) | Bus: {} ({}) | Weather: {} | {:02}:{:02}",
        setup.selected_map,
        setup.selected_entry_point,
        setup.selected_vehicle,
        setup.selected_livery,
        setup.weather_preset,
        setup.hour,
        setup.minute
    );

    for mut text in launch_summary_query.iter_mut() {
        text.0 = summary_str.clone();
    }
    for mut text in tab_summary_query.iter_mut() {
        text.0 = summary_str.clone();
    }
}

/// Helper synchronizing footer summary text.
pub fn update_summary_text<F: bevy::ecs::query::QueryFilter>(
    setup: &NewGameSetup,
    summary_query: &mut Query<&mut Text, F>,
) {
    for mut text in summary_query.iter_mut() {
        text.0 = format!(
            "Map: {} ({}) | Bus: {} ({}) | Weather: {} | {:02}:{:02}",
            setup.selected_map,
            setup.selected_entry_point,
            setup.selected_vehicle,
            setup.selected_livery,
            setup.weather_preset,
            setup.hour,
            setup.minute
        );
    }
}

/// System cleaning up new game menu entities on state exit.
pub fn cleanup_new_game_screen(
    mut commands: Commands,
    query: Query<Entity, With<NewGameScreenRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Escape key listener returning to the Main Menu.
pub fn escape_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}
