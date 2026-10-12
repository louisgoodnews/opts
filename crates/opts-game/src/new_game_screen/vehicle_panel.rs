//! 30% / 70% Two-frame layout for OMSI Vehicle Selection.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::card::{spawn_stat_chip, spawn_tab_summary_card};
use crate::new_game_screen::scroll_bar::{spawn_scroll_to_top_button, spawn_scrollbar};
use crate::new_game_screen::search_bar::spawn_search_bar;
use crate::new_game_screen::types::{
    GameCatalogue, NewGameSetup, ScrollContent, ScrollKind, ScrollableContainer, SelectVehicleBtn,
    VehicleCardItem, VehicleDetailPanelRoot, VehicleItem,
};

/// Spawns the Vehicle Selection tab with a 30% scrollable left frame and a 70% details right frame.
pub fn spawn_vehicle_selection_tab(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) {
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|row| {
            // ==========================================
            // Left Frame: 30% Scrolled Vehicle List
            // ==========================================
            row.spawn((
                Node {
                    width: Val::Percent(30.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::right(Val::Px(1.0)),
                    padding: UiRect::right(Val::Px(12.0)),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            ))
            .with_children(|left_col| {
                // Top Search Bar
                spawn_search_bar(left_col, ScrollKind::Vehicles, "Search 956 vehicles...", "");

                // Header with total count and scroll controls
                // Header with total count
                left_col.spawn((
                    (
                        Text::new(format!("Fleet Models ({})", catalogue.vehicles.len())),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::Primary)),
                    ),
                    TypeStyleToken(TypeScale::TitleMedium),
                    ForegroundRole(ColorRole::Primary),
                ));

                // Scroll viewport
                let item_height = 58.0;
                let total_height = (catalogue.vehicles.len() as f32) * item_height;
                let viewport_height = 470.0;
                let max_scroll = (total_height - viewport_height).max(0.0);

                left_col
                    .spawn((
                        (Node {
                            flex_grow: 1.0,
                            width: Val::Percent(100.0),
                            height: Val::Px(viewport_height),
                            overflow: Overflow::clip_y(),
                            flex_direction: FlexDirection::Column,
                            position_type: PositionType::Relative,
                            ..default()
                        }),
                        ScrollableContainer {
                            kind: ScrollKind::Vehicles,
                            scroll_y: 0.0,
                            max_scroll_y: max_scroll,
                        },
                    ))
                    .with_children(|viewport| {
                        viewport
                            .spawn((
                                (Node {
                                    flex_direction: FlexDirection::Column,
                                    width: Val::Percent(100.0),
                                    padding: UiRect::right(Val::Px(24.0)),
                                    row_gap: Val::Px(6.0),
                                    position_type: PositionType::Relative,
                                    top: Val::Px(0.0),
                                    ..default()
                                }),
                                ScrollContent {
                                    kind: ScrollKind::Vehicles,
                                },
                            ))
                            .with_children(|list| {
                                for veh in &catalogue.vehicles {
                                    let is_selected = veh.model == setup.selected_vehicle
                                        || veh.full_name == setup.selected_vehicle
                                        || veh.id == setup.selected_vehicle;

                                    let search_str = format!(
                                        "{} {} {} {}",
                                        veh.manufacturer,
                                        veh.model,
                                        veh.full_name,
                                        veh.default_livery
                                    )
                                    .to_lowercase();

                                    list.spawn((
                                        (
                                            Button,
                                            Node {
                                                width: Val::Percent(100.0),
                                                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                                flex_direction: FlexDirection::Column,
                                                border: UiRect::all(Val::Px(if is_selected {
                                                    2.0
                                                } else {
                                                    1.0
                                                })),
                                                ..default()
                                            },
                                            BorderColor::all(if is_selected {
                                                theme.scheme.get(ColorRole::Primary)
                                            } else {
                                                theme.scheme.get(ColorRole::OutlineVariant)
                                            }),
                                            BackgroundColor(if is_selected {
                                                theme.scheme.get(ColorRole::PrimaryContainer)
                                            } else {
                                                theme.scheme.get(ColorRole::SurfaceContainerLow)
                                            }),
                                        ),
                                        M3Button::default(),
                                        SelectVehicleBtn {
                                            vehicle_id: veh.id.clone(),
                                        },
                                        VehicleCardItem {
                                            vehicle_id: veh.id.clone(),
                                            search_text: search_str,
                                        },
                                        GlassSurface::new(
                                            if is_selected {
                                                ColorRole::PrimaryContainer
                                            } else {
                                                ColorRole::SurfaceContainerLow
                                            },
                                            if is_selected { 0.85 } else { 0.50 },
                                        ),
                                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                        Focusable { tab_index: 0 },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            (
                                                Text::new(&veh.full_name),
                                                TextFont {
                                                    font_size: FontSize::Px(13.0),
                                                    ..default()
                                                },
                                                TextColor(if is_selected {
                                                    theme.scheme.get(ColorRole::OnPrimaryContainer)
                                                } else {
                                                    theme.scheme.get(ColorRole::OnSurface)
                                                }),
                                            ),
                                            TypeStyleToken(TypeScale::BodyMedium),
                                            ForegroundRole(ColorRole::OnSurface),
                                        ));

                                        btn.spawn((
                                            (
                                                Text::new(format!(
                                                    "{} · {}",
                                                    veh.manufacturer, veh.default_livery
                                                )),
                                                TextFont {
                                                    font_size: FontSize::Px(11.0),
                                                    ..default()
                                                },
                                                TextColor(theme.scheme.get(ColorRole::Outline)),
                                            ),
                                            TypeStyleToken(TypeScale::Label),
                                            ForegroundRole(ColorRole::Outline),
                                        ));
                                    });
                                }
                            });

                        // Right edge scrollbar indicator
                        spawn_scrollbar(viewport, theme, ScrollKind::Vehicles);

                        // Floating Scroll-To-Top button
                        spawn_scroll_to_top_button(viewport, theme, ScrollKind::Vehicles);
                    });
            });

            // ==========================================
            // Right Frame: 70% Vehicle Description & Specs
            // ==========================================
            row.spawn((
                (
                    Node {
                        width: Val::Percent(70.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLow)),
                ),
                VehicleDetailPanelRoot,
                GlassSurface::new(ColorRole::SurfaceContainerLow, 0.40),
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            ))
            .with_children(|right_col| {
                let current_veh = catalogue
                    .vehicles
                    .iter()
                    .find(|v| {
                        v.model == setup.selected_vehicle
                            || v.full_name == setup.selected_vehicle
                            || v.id == setup.selected_vehicle
                    })
                    .or_else(|| catalogue.vehicles.first());

                if let Some(current_veh) = current_veh {
                    populate_vehicle_details_inner(right_col, theme, current_veh, setup);
                } else {
                    crate::new_game_screen::map_panel::spawn_catalogue_pending_notice(
                        right_col, theme,
                    );
                }
            });
        });
}

/// Populates the inner content of the 70% vehicle detail frame.
pub fn populate_vehicle_details_inner(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    veh: &VehicleItem,
    setup: &NewGameSetup,
) {
    // Large Headline Title
    parent.spawn((
        (
            Text::new(&veh.full_name),
            TextFont {
                font_size: FontSize::Px(26.0),
                ..default()
            },
            TextColor(theme.scheme.get(ColorRole::Primary)),
        ),
        TypeStyleToken(TypeScale::Headline),
        ForegroundRole(ColorRole::Primary),
    ));

    // Subtitle with Manufacturer and Model Code
    parent.spawn((
        (
            Text::new(format!(
                "Fleet Manufacturer: {} · Model Architecture: {}",
                veh.manufacturer, veh.model
            )),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
        ),
        TypeStyleToken(TypeScale::Label),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));

    // Technical specification badges row 1
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|badges| {
            spawn_stat_chip(badges, theme, "Powertrain", &veh.engine_power);
            spawn_stat_chip(badges, theme, "Transmission", &veh.transmission);
            spawn_stat_chip(badges, theme, "Capacity", &veh.capacity);
            spawn_stat_chip(badges, theme, "Overall Length", &veh.length_m);
        });

    // Technical specification badges row 2
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|badges| {
            spawn_stat_chip(badges, theme, "Top Speed", &veh.max_speed_kmh);
            spawn_stat_chip(badges, theme, "Passenger Doors", &veh.passenger_doors);
            spawn_stat_chip(badges, theme, "Assigned Depot", "Spandau Depot");
            spawn_stat_chip(badges, theme, "Fleet Status", "Operational");
        });

    // Multi-line Description Box
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                width: Val::Percent(100.0),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
        ))
        .with_children(|desc_box| {
            desc_box.spawn((
                (
                    Text::new("Vehicle Profile & Technical Specifications:"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::Primary),
            ));

            desc_box.spawn((
                (
                    Text::new(&veh.description),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurface)),
                ),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurface),
            ));
        });

    // Scrollable Repaints selection and Configure Setvars trigger
    crate::new_game_screen::repaint_selector::spawn_repaint_scrollable_selection(
        parent, theme, veh, setup,
    );

    // Summary of current selection at bottom of tab
    spawn_tab_summary_card(parent, theme, setup);
}
