//! Layout and widget spawning helpers for Tab 3: Weather and Datetime Selection.
//! Reworked into a 3-column layout (1:1:1) featuring scrollable weather presets,
//! live device-location weather telemetry, and aligned date/time pickers.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::card::spawn_tab_summary_card;
use crate::new_game_screen::clock_picker::spawn_material_clock_picker;
use crate::new_game_screen::date_picker::spawn_material_date_picker;
use crate::new_game_screen::live_weather::{populate_weather_details_inner, ALL_WEATHER_PRESETS};
use crate::new_game_screen::scroll_bar::{spawn_scroll_to_top_button, spawn_scrollbar};
use crate::new_game_screen::types::{
    AdjustTimeBtn, DepartureTimeDisplayText, DeviceLocationWeather, NewGameSetup, ScrollContent,
    ScrollKind, ScrollableContainer, SelectWeatherBtn, WeatherCardItem, WeatherDetailPanelRoot,
};

/// Populates Tab 3: Weather & Datetime Selection in a 3-column architecture (1:1:1 ratio).
pub fn spawn_weather_datetime_tab(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
    live_weather: &DeviceLocationWeather,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            ..default()
        })
        .with_children(|main_col| {
            // 3-Column Container
            main_col
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(16.0),
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|cols_row| {
                    // =========================================================
                    // Column 1 (1/3): Scrollable Weather Presets Selection
                    // =========================================================
                    cols_row
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(27.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                                background_color: BackgroundColor(
                                    theme.scheme.get(ColorRole::SurfaceContainerLow),
                                ),
                                ..default()
                            },
                            GlassSurface::new(ColorRole::SurfaceContainerLow, 0.45),
                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                        ))
                        .with_children(|col1| {
                            col1.spawn((
                                TextBundle::from_section(
                                    "Atmospheric Presets (Scrollable):",
                                    TextStyle {
                                        font_size: 15.0,
                                        color: theme.scheme.get(ColorRole::Primary),
                                        ..default()
                                    },
                                ),
                                TypeStyleToken(TypeScale::TitleSmall),
                                ForegroundRole(ColorRole::Primary),
                            ));

                            let item_height = 56.0;
                            let viewport_height = 420.0;
                            let total_height = ALL_WEATHER_PRESETS.len() as f32 * item_height;
                            let max_scroll_y = (total_height - viewport_height).max(0.0);

                            col1.spawn((
                                NodeBundle {
                                    style: Style {
                                        width: Val::Percent(100.0),
                                        height: Val::Px(viewport_height),
                                        position_type: PositionType::Relative,
                                        overflow: Overflow::clip_y(),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                                    background_color: BackgroundColor(
                                        theme.scheme.get(ColorRole::SurfaceContainerLowest),
                                    ),
                                    ..default()
                                },
                                RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                            ))
                            .with_children(|viewport| {
                                viewport
                                    .spawn((
                                        NodeBundle {
                                            style: Style {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                position_type: PositionType::Relative,
                                                overflow: Overflow::clip_y(),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                        ScrollableContainer {
                                            kind: ScrollKind::Weather,
                                            scroll_y: 0.0,
                                            max_scroll_y,
                                        },
                                    ))
                                    .with_children(|scroll_container| {
                                        scroll_container
                                            .spawn((
                                                NodeBundle {
                                                    style: Style {
                                                        width: Val::Percent(100.0),
                                                        flex_direction: FlexDirection::Column,
                                                        row_gap: Val::Px(6.0),
                                                        position_type: PositionType::Absolute,
                                                        left: Val::Px(0.0),
                                                        top: Val::Px(0.0),
                                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0))
                                                            .with_right(Val::Px(24.0)),
                                                        ..default()
                                                    },
                                                    ..default()
                                                },
                                                ScrollContent {
                                                    kind: ScrollKind::Weather,
                                                },
                                            ))
                                            .with_children(|content| {
                                                for preset in ALL_WEATHER_PRESETS {
                                                    let is_active = setup.weather_preset == preset.id || setup.weather_preset == preset.name;
                                                    let border_c = if is_active {
                                                        theme.scheme.get(ColorRole::Primary)
                                                    } else {
                                                        theme.scheme.get(ColorRole::OutlineVariant)
                                                    };
                                                    let bg_c = if is_active {
                                                        theme.scheme.get(ColorRole::PrimaryContainer)
                                                    } else {
                                                        theme.scheme.get(ColorRole::SurfaceContainerLow)
                                                    };

                                                    content
                                                        .spawn((
                                                            ButtonBundle {
                                                                style: Style {
                                                                    width: Val::Percent(100.0),
                                                                    padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                                                                    flex_direction: FlexDirection::Row,
                                                                    align_items: AlignItems::Center,
                                                                    justify_content: JustifyContent::SpaceBetween,
                                                                    border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                                                                    ..default()
                                                                },
                                                                border_color: BorderColor(border_c),
                                                                background_color: BackgroundColor(bg_c),
                                                                ..default()
                                                            },
                                                            M3Button::default(),
                                                            SelectWeatherBtn {
                                                                preset_name: preset.id,
                                                            },
                                                            WeatherCardItem {
                                                                preset_name: preset.id.to_string(),
                                                            },
                                                            GlassSurface::new(
                                                                if is_active {
                                                                    ColorRole::PrimaryContainer
                                                                } else {
                                                                    ColorRole::SurfaceContainerLow
                                                                },
                                                                if is_active { 0.85 } else { 0.50 },
                                                            ),
                                                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                                            Focusable { tab_index: 0 },
                                                        ))
                                                        .with_children(|btn| {
                                                            btn.spawn(NodeBundle {
                                                                style: Style {
                                                                    flex_direction: FlexDirection::Row,
                                                                    align_items: AlignItems::Center,
                                                                    column_gap: Val::Px(8.0),
                                                                    ..default()
                                                                },
                                                                ..default()
                                                            })
                                                            .with_children(|title_box| {
                                                                title_box.spawn((
                                                                    TextBundle::from_section(
                                                                        preset.name,
                                                                        TextStyle {
                                                                            font_size: 13.0,
                                                                            color: if is_active {
                                                                                theme.scheme.get(ColorRole::OnPrimaryContainer)
                                                                            } else {
                                                                                theme.scheme.get(ColorRole::OnSurface)
                                                                            },
                                                                            ..default()
                                                                        },
                                                                    ),
                                                                    TypeStyleToken(TypeScale::LabelMedium),
                                                                ));
                                                            });

                                                            btn.spawn((
                                                                TextBundle::from_section(
                                                                    preset.temp_str,
                                                                    TextStyle {
                                                                        font_size: 11.5,
                                                                        color: if is_active {
                                                                            theme.scheme.get(ColorRole::Primary)
                                                                        } else {
                                                                            theme.scheme.get(ColorRole::Outline)
                                                                        },
                                                                        ..default()
                                                                    },
                                                                ),
                                                                TypeStyleToken(TypeScale::LabelSmall),
                                                            ));
                                                        });
                                                }
                                            });
                                    });

                                spawn_scrollbar(viewport, theme, ScrollKind::Weather);
                                spawn_scroll_to_top_button(viewport, theme, ScrollKind::Weather);
                            });
                        });

                    // =========================================================
                    // Column 2 (31%): Description & Live Device Location Telemetry
                    // =========================================================
                    cols_row
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(31.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(12.0),
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    overflow: Overflow::clip_y(),
                                    ..default()
                                },
                                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                                background_color: BackgroundColor(
                                    theme.scheme.get(ColorRole::SurfaceContainerLow),
                                ),
                                ..default()
                            },
                            WeatherDetailPanelRoot,
                            GlassSurface::new(ColorRole::SurfaceContainerLow, 0.40),
                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                        ))
                        .with_children(|center_col| {
                            populate_weather_details_inner(center_col, theme, setup, live_weather);
                        });

                    // =========================================================
                    // Column 3 (42%): Aligned Date & Time Pickers + Steppers
                    // =========================================================
                    cols_row
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(42.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    row_gap: Val::Px(10.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    overflow: Overflow::clip_y(),
                                    ..default()
                                },
                                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                                background_color: BackgroundColor(
                                    theme.scheme.get(ColorRole::SurfaceContainerLow),
                                ),
                                ..default()
                            },
                            GlassSurface::new(ColorRole::SurfaceContainerLow, 0.40),
                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                        ))
                        .with_children(|col3| {
                            // Quick Departure Readout
                            let departure_str = format!(
                                "Departure Time: {:02}:{:02} (Date: {:04}-{:02}-{:02})",
                                setup.hour, setup.minute, setup.year, setup.month, setup.day
                            );
                            col3.spawn((
                                TextBundle::from_section(
                                    departure_str,
                                    TextStyle {
                                        font_size: 13.0,
                                        color: theme.scheme.get(ColorRole::Secondary),
                                        ..default()
                                    },
                                ),
                                DepartureTimeDisplayText,
                                TypeStyleToken(TypeScale::TitleSmall),
                                ForegroundRole(ColorRole::Secondary),
                            ));

                            // Stepper Row
                            col3.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|steppers| {
                                for (offset, label) in [(-2, "-2h"), (-1, "-1h"), (1, "+1h"), (2, "+2h")] {
                                    steppers
                                        .spawn((
                                            ButtonBundle {
                                                style: Style {
                                                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                                                background_color: BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
                                                ..default()
                                            },
                                            M3Button::default(),
                                            AdjustTimeBtn { hour_offset: offset },
                                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                            Focusable { tab_index: 0 },
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                TextBundle::from_section(
                                                    label,
                                                    TextStyle {
                                                        font_size: 11.0,
                                                        color: theme.scheme.get(ColorRole::OnSurface),
                                                        ..default()
                                                    },
                                                ),
                                                TypeStyleToken(TypeScale::LabelSmall),
                                                ForegroundRole(ColorRole::OnSurface),
                                            ));
                                        });
                                }
                            });

                            // Date & Time Pickers Container (Aligned side-by-side or wrapped)
                            col3.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Start,
                                    column_gap: Val::Px(10.0),
                                    flex_wrap: FlexWrap::Wrap,
                                    overflow: Overflow::clip_y(),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|pickers_node| {
                                spawn_material_date_picker(pickers_node, theme, setup);
                                spawn_material_clock_picker(pickers_node, theme, setup);
                            });
                        });
                });

            spawn_tab_summary_card(main_col, theme, setup);
        });
}
