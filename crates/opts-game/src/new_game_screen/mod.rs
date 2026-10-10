//! Interactive Start New Game menu screen utilizing the Material Design 3 Notebook widget.
//!
//! # Architecture & Tabs
//! - **Tab 1: Map Selection:** 30% / 70% Two-frame layout with scrollable list of all OMSI maps and rich metadata details.
//! - **Tab 2: Vehicle Selection:** 30% / 70% Two-frame layout with scrollable list of all playable vehicles and specs.
//! - **Tab 3: Weather and Datetime Selection:** Weather environmental presets and date/time departure setup.
//! - Launch summary bar enabling immediate transition to active simulation ([`AppState::InGame`]).

pub mod card;
pub mod clock_controls;
pub mod clock_picker;
pub mod date_picker;
pub mod entrypoint_selector;
pub mod keyboard_layout;
pub mod live_weather;
pub mod map_panel;
pub mod picker_systems;
pub mod repaint_selector;
pub mod scroll_bar;
pub mod scroll_systems;
pub mod search_bar;
pub mod search_systems;
pub mod setvars_discovery;
pub mod setvars_modal;
pub mod systems;
pub mod types;
pub mod vehicle_panel;
pub mod weather_panel;
pub mod year_spinbox;

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::menu::glass::GlassSurface;

pub use keyboard_layout::SystemKeyboardLayout;
pub use map_panel::spawn_map_selection_tab;
pub use picker_systems::{
    handle_calendar_interaction, handle_clock_picker_interaction, handle_scroll_to_top,
    handle_year_spinbox, update_clock_picker_visuals, update_scrollbar_thumbs,
};
pub use search_systems::{
    filter_search_cards, handle_clear_search, handle_cursor_blink, handle_search_focus,
    handle_search_typing,
};
pub use systems::{
    cleanup_new_game_screen, escape_to_menu_system, handle_back_button,
    handle_entrypoint_selection, handle_launch_button, handle_livery_selection,
    handle_map_selection, handle_mouse_scroll, handle_scroll_buttons, handle_time_adjustment,
    handle_vehicle_selection, handle_weather_selection,
};
pub use types::{
    AdjustMinuteBtn, AdjustTimeBtn, ApplyLiveWeatherBtn, BackToMenuBtn, CalendarHeaderDateText,
    CalendarMonthYearText, CatalogueSearchState, ClearSearchBtn, ClockDigitalDisplayText,
    CloseSetvarsModalBtn, DepartureTimeDisplayText, DeviceLocationWeather, EntryPointCardItem,
    GameCatalogue, LaunchGameBtn, LaunchSummaryText, MapCardItem, MapDetailPanelRoot, MapItem,
    NewGameScreenRoot, NewGameSetup, NextMonthBtn, OpenSetvarsModalBtn, PrevMonthBtn,
    RefreshWeatherBtn, RepaintCardItem, ResetSetvarsBtn, ScrollContent, ScrollDownBtn, ScrollKind,
    ScrollToTopBtn, ScrollUpBtn, ScrollableContainer, ScrollbarThumb, ScrollbarTrack,
    SelectCalendarDayBtn, SelectClockHourBtn, SelectClockMeridianBtn, SelectEntryPointBtn,
    SelectLiveryBtn, SelectMapBtn, SelectVehicleBtn, SelectWeatherBtn, SetvarStepBtn,
    SetvarsModalRoot, TabSummaryText, VehicleCardItem, VehicleDetailPanelRoot, VehicleItem,
    WeatherCardItem, WeatherDetailPanelRoot, YearDecrementBtn, YearIncrementBtn, YearInputBox,
    YearInputText,
};
pub use vehicle_panel::spawn_vehicle_selection_tab;
pub use weather_panel::spawn_weather_datetime_tab;
pub use year_spinbox::spawn_year_spinbox;

/// Plugin managing the interactive Start New Game screen.
#[derive(Default)]
pub struct NewGameMenuPlugin;

impl Plugin for NewGameMenuPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<GameCatalogue>() {
            app.init_resource::<GameCatalogue>();
        }

        if !app.world().contains_resource::<NewGameSetup>() {
            app.init_resource::<NewGameSetup>();
        }

        if !app.world().contains_resource::<CatalogueSearchState>() {
            app.init_resource::<CatalogueSearchState>();
        }

        if !app.world().contains_resource::<SystemKeyboardLayout>() {
            app.insert_resource(SystemKeyboardLayout::detect());
        }

        if !app.world().contains_resource::<DeviceLocationWeather>() {
            app.init_resource::<DeviceLocationWeather>();
        }

        app.add_systems(OnEnter(AppState::VehicleSelection), spawn_new_game_screen)
            .add_systems(OnExit(AppState::VehicleSelection), cleanup_new_game_screen)
            .add_systems(
                Update,
                (
                    handle_back_button,
                    handle_launch_button,
                    handle_map_selection,
                    handle_entrypoint_selection,
                    handle_vehicle_selection,
                    handle_livery_selection,
                    handle_scroll_buttons,
                    handle_mouse_scroll,
                    handle_weather_selection,
                    handle_time_adjustment,
                )
                    .run_if(in_state(AppState::VehicleSelection))
                    .run_if(opts_bevy::reporting::not_modal_active),
            )
            .add_systems(
                Update,
                (
                    handle_search_focus,
                    handle_clear_search,
                    handle_cursor_blink,
                    handle_search_typing,
                    filter_search_cards,
                    handle_scroll_to_top,
                    update_scrollbar_thumbs,
                    handle_calendar_interaction,
                    handle_year_spinbox,
                    handle_clock_picker_interaction,
                    update_clock_picker_visuals
                        .after(handle_clock_picker_interaction)
                        .after(handle_time_adjustment),
                    setvars_modal::handle_setvars_modal_interactions,
                    escape_to_menu_system,
                )
                    .run_if(in_state(AppState::VehicleSelection))
                    .run_if(opts_bevy::reporting::not_modal_active),
            );
    }
}

/// Spawns the complete Start New Game screen featuring the 3-tab Notebook widget.
pub fn spawn_new_game_screen(
    mut commands: Commands,
    theme: Res<MaterialTheme>,
    catalogue: Res<GameCatalogue>,
    setup: Res<NewGameSetup>,
    live_weather: Res<DeviceLocationWeather>,
) {
    // 1. Fullscreen Root Container (flush to window borders)
    let root = commands
        .spawn((
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::ZERO,
                    row_gap: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(theme.scheme.get(ColorRole::Surface)),
            ),
            NewGameScreenRoot,
        ))
        .id();

    // 2. Header Bar with Title and Back Navigation
    commands.entity(root).with_children(|header_parent| {
        header_parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(52.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect {
                        bottom: Val::Px(1.0),
                        ..default()
                    },
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            ))
            .with_children(|bar| {
                bar.spawn(
                    (Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        ..default()
                    }),
                )
                .with_children(|title_box| {
                    title_box.spawn((
                        (
                            Text::new("Start New Game"),
                            TextFont {
                                font_size: FontSize::Px(22.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::Primary)),
                        ),
                        TypeStyleToken(TypeScale::HeadlineSmall),
                        ForegroundRole(ColorRole::Primary),
                    ));

                    title_box.spawn((
                        (
                            Text::new(
                                "Configure World Map, Vehicle Fleet, and Environmental Simulation",
                            ),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                        ),
                        TypeStyleToken(TypeScale::LabelMedium),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));
                });

                // Back Button
                bar.spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                        BackgroundColor(theme.scheme.get(ColorRole::SecondaryContainer)),
                    ),
                    M3Button::default(),
                    BackToMenuBtn,
                    GlassSurface::new(ColorRole::SecondaryContainer, 0.70),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    Focusable { tab_index: 0 },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        (
                            Text::new("< Back to Main Menu"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSecondaryContainer)),
                        ),
                        TypeStyleToken(TypeScale::LabelMedium),
                        ForegroundRole(ColorRole::OnSecondaryContainer),
                    ));
                });
            });
    });

    // 3. Tabbed Notebook Widget (3 Tabs)
    let (notebook_root, panels) = NotebookBuilder::new()
        .with_tabs(vec![
            "Map Selection".into(),
            "Vehicle Selection".into(),
            "Weather & Datetime".into(),
        ])
        .with_active(0)
        .spawn_with_panels(&mut commands);

    commands.entity(notebook_root).insert(Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        height: Val::Auto,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_height: Val::Px(0.0),
        overflow: Overflow::clip(),
        ..default()
    });

    commands.entity(root).add_child(notebook_root);

    // Tab 1: Map Selection Panel (30% / 70% Two Frames)
    if let Some(&map_panel) = panels.first() {
        commands.entity(map_panel).with_children(|tab| {
            spawn_map_selection_tab(tab, &theme, &catalogue, &setup);
        });
    }

    // Tab 2: Vehicle Selection Panel (30% / 70% Two Frames)
    if let Some(&veh_panel) = panels.get(1) {
        commands.entity(veh_panel).with_children(|tab| {
            spawn_vehicle_selection_tab(tab, &theme, &catalogue, &setup);
        });
    }

    // Tab 3: Weather & Datetime Selection Panel
    if let Some(&weather_panel) = panels.get(2) {
        commands.entity(weather_panel).with_children(|tab| {
            spawn_weather_datetime_tab(tab, &theme, &setup, &live_weather);
        });
    }

    // 4. Bottom Action & Launch Bar (Pinned to viewport bottom)
    commands.entity(root).with_children(|footer_parent| {
        footer_parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(56.0),
                    min_height: Val::Px(56.0),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect {
                        top: Val::Px(1.0),
                        ..default()
                    },
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            ))
            .with_children(|bar| {
                let summary_text = format!(
                    "Map: {} ({}) | Bus: {} ({}) | Weather: {} | {:02}:{:02}",
                    setup.selected_map,
                    setup.selected_entry_point,
                    setup.selected_vehicle,
                    setup.selected_livery,
                    setup.weather_preset,
                    setup.hour,
                    setup.minute
                );

                bar.spawn((
                    (
                        Text::new(summary_text),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::OnSurface)),
                    ),
                    LaunchSummaryText,
                    TypeStyleToken(TypeScale::TitleSmall),
                    ForegroundRole(ColorRole::OnSurface),
                ));

                // Launch Simulation Primary Button
                bar.spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(theme.scheme.get(ColorRole::Primary)),
                        BackgroundColor(theme.scheme.get(ColorRole::Primary)),
                    ),
                    M3Button::default(),
                    LaunchGameBtn,
                    GlassSurface::new(ColorRole::Primary, 0.90),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    Focusable { tab_index: 1 },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        (
                            Text::new("Start Simulation"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnPrimary)),
                        ),
                        TypeStyleToken(TypeScale::TitleSmall),
                        ForegroundRole(ColorRole::OnPrimary),
                    ));
                });
            });
    });
}
