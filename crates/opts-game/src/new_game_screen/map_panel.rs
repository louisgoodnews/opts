//! 30% / 70% Two-frame layout for OMSI Map Selection.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::card::{spawn_stat_chip, spawn_tab_summary_card};
use crate::new_game_screen::scroll_bar::{spawn_scroll_to_top_button, spawn_scrollbar};
use crate::new_game_screen::search_bar::spawn_search_bar;
use crate::new_game_screen::types::{
    GameCatalogue, MapCardItem, MapDetailPanelRoot, MapItem, NewGameSetup, ScrollContent,
    ScrollKind, ScrollableContainer, SelectMapBtn,
};

/// Spawns the Map Selection tab with a 30% scrollable left frame and a 70% details right frame.
pub fn spawn_map_selection_tab(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                column_gap: Val::Px(16.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            // ==========================================
            // Left Frame: 30% Scrolled Map List
            // ==========================================
            row.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(30.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::right(Val::Px(1.0)),
                    padding: UiRect::right(Val::Px(12.0)),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                ..default()
            })
            .with_children(|left_col| {
                // Top Search Bar
                spawn_search_bar(
                    left_col,
                    theme,
                    ScrollKind::Maps,
                    "Search 42 maps...",
                    "",
                    false,
                );

                // Header with total count
                left_col.spawn((
                    TextBundle::from_section(
                        format!("OMSI Maps ({})", catalogue.maps.len()),
                        TextStyle {
                            font_size: 14.0,
                            color: theme.scheme.get(ColorRole::Primary),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::TitleMedium),
                    ForegroundRole(ColorRole::Primary),
                ));

                // Scroll viewport
                let item_height = 58.0;
                let total_height = (catalogue.maps.len() as f32) * item_height;
                let viewport_height = 470.0;
                let max_scroll = (total_height - viewport_height).max(0.0);

                left_col
                    .spawn((
                        NodeBundle {
                            style: Style {
                                flex_grow: 1.0,
                                width: Val::Percent(100.0),
                                height: Val::Px(viewport_height),
                                overflow: Overflow::clip_y(),
                                flex_direction: FlexDirection::Column,
                                position_type: PositionType::Relative,
                                ..default()
                            },
                            ..default()
                        },
                        ScrollableContainer {
                            kind: ScrollKind::Maps,
                            scroll_y: 0.0,
                            max_scroll_y: max_scroll,
                        },
                    ))
                    .with_children(|viewport| {
                        viewport
                            .spawn((
                                NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Column,
                                        width: Val::Percent(100.0),
                                        padding: UiRect::right(Val::Px(24.0)),
                                        row_gap: Val::Px(6.0),
                                        position_type: PositionType::Relative,
                                        top: Val::Px(0.0),
                                        ..default()
                                    },
                                    ..default()
                                },
                                ScrollContent {
                                    kind: ScrollKind::Maps,
                                },
                            ))
                            .with_children(|list| {
                                for map in &catalogue.maps {
                                    let is_selected = map.id == setup.selected_map
                                        || map.name == setup.selected_map
                                        || map.friendly_name == setup.selected_map;

                                    let search_str = format!(
                                        "{} {} {}",
                                        map.friendly_name, map.name, map.standard_depot
                                    )
                                    .to_lowercase();

                                    list.spawn((
                                        ButtonBundle {
                                            style: Style {
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
                                            border_color: BorderColor(if is_selected {
                                                theme.scheme.get(ColorRole::Primary)
                                            } else {
                                                theme.scheme.get(ColorRole::OutlineVariant)
                                            }),
                                            background_color: BackgroundColor(if is_selected {
                                                theme.scheme.get(ColorRole::PrimaryContainer)
                                            } else {
                                                theme.scheme.get(ColorRole::SurfaceContainerLow)
                                            }),
                                            ..default()
                                        },
                                        M3Button::default(),
                                        SelectMapBtn {
                                            map_id: map.id.clone(),
                                        },
                                        MapCardItem {
                                            map_id: map.id.clone(),
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
                                            TextBundle::from_section(
                                                &map.friendly_name,
                                                TextStyle {
                                                    font_size: 13.0,
                                                    color: if is_selected {
                                                        theme
                                                            .scheme
                                                            .get(ColorRole::OnPrimaryContainer)
                                                    } else {
                                                        theme.scheme.get(ColorRole::OnSurface)
                                                    },
                                                    ..default()
                                                },
                                            ),
                                            TypeStyleToken(TypeScale::LabelLarge),
                                            ForegroundRole(ColorRole::OnSurface),
                                        ));

                                        btn.spawn((
                                            TextBundle::from_section(
                                                format!("{} World Tiles", map.tile_count),
                                                TextStyle {
                                                    font_size: 11.0,
                                                    color: theme.scheme.get(ColorRole::Outline),
                                                    ..default()
                                                },
                                            ),
                                            TypeStyleToken(TypeScale::LabelSmall),
                                            ForegroundRole(ColorRole::Outline),
                                        ));
                                    });
                                }
                            });

                        // Right edge scrollbar indicator
                        spawn_scrollbar(viewport, theme, ScrollKind::Maps);

                        // Floating Scroll-To-Top button
                        spawn_scroll_to_top_button(viewport, theme, ScrollKind::Maps);
                    });
            });

            // ==========================================
            // Right Frame: 70% Map Description & Metadata
            // ==========================================
            row.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(70.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    background_color: BackgroundColor(
                        theme.scheme.get(ColorRole::SurfaceContainerLow),
                    ),
                    ..default()
                },
                MapDetailPanelRoot,
                GlassSurface::new(ColorRole::SurfaceContainerLow, 0.40),
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            ))
            .with_children(|right_col| {
                let current_map = catalogue
                    .maps
                    .iter()
                    .find(|m| {
                        m.id == setup.selected_map
                            || m.name == setup.selected_map
                            || m.friendly_name == setup.selected_map
                    })
                    .or_else(|| catalogue.maps.first());

                if let Some(current_map) = current_map {
                    populate_map_details_inner(right_col, theme, current_map, setup);
                } else {
                    spawn_catalogue_pending_notice(right_col, theme);
                }
            });
        });
}

/// Populates the inner content of the 70% map detail frame.
pub fn populate_map_details_inner(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    map: &MapItem,
    setup: &NewGameSetup,
) {
    // Large Headline Title
    parent.spawn((
        TextBundle::from_section(
            &map.friendly_name,
            TextStyle {
                font_size: 26.0,
                color: theme.scheme.get(ColorRole::Primary),
                ..default()
            },
        ),
        TypeStyleToken(TypeScale::HeadlineMedium),
        ForegroundRole(ColorRole::Primary),
    ));

    // Subtitle with Map ID and Era
    parent.spawn((
        TextBundle::from_section(
            format!(
                "Map Directory: maps/{} · Era Timeline: {}",
                map.id, map.years
            ),
            TextStyle {
                font_size: 12.0,
                color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                ..default()
            },
        ),
        TypeStyleToken(TypeScale::BodySmall),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));

    // Metadata stats badges row
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                flex_wrap: FlexWrap::Wrap,
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|badges| {
            spawn_stat_chip(badges, theme, "World Tiles", &format!("{}", map.tile_count));
            spawn_stat_chip(badges, theme, "Standard Depot", &map.standard_depot);
            spawn_stat_chip(
                badges,
                theme,
                "Entrypoints",
                &format!("{}", map.entry_points.len()),
            );
        });

    // Multi-line Description Box
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                width: Val::Percent(100.0),
                row_gap: Val::Px(6.0),
                ..default()
            },
            border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
            background_color: BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
            ..default()
        })
        .with_children(|desc_box| {
            desc_box.spawn((
                TextBundle::from_section(
                    "Map Scenario Description:",
                    TextStyle {
                        font_size: 13.0,
                        color: theme.scheme.get(ColorRole::Primary),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::LabelLarge),
                ForegroundRole(ColorRole::Primary),
            ));

            desc_box.spawn((
                TextBundle::from_section(
                    &map.description,
                    TextStyle {
                        font_size: 12.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodySmall),
                ForegroundRole(ColorRole::OnSurface),
            ));
        });

    // Scrollable Entrypoint selection list
    crate::new_game_screen::entrypoint_selector::spawn_entrypoint_scrollable_selection(
        parent, theme, map, setup,
    );

    // Summary of current selection at bottom of tab
    spawn_tab_summary_card(parent, theme, setup);
}

/// Notice shown while the OMSI catalogue is still empty (first run, discovery in progress).
pub fn spawn_catalogue_pending_notice(parent: &mut ChildBuilder, theme: &MaterialTheme) {
    parent.spawn(TextBundle::from_section(
        "The OMSI catalogue is being built in the background. Reopen this screen in a moment.",
        TextStyle {
            font_size: 16.0,
            color: theme.scheme.get(ColorRole::OnSurfaceVariant),
            ..default()
        },
    ));
}
