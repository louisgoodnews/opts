//! 30% / 70% Two-frame layout for OMSI Map Selection.

use bevy::prelude::*;
use opts_gui::components::Button;
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
            // Left Frame: 30% Scrolled Map List
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
                spawn_search_bar(left_col, ScrollKind::Maps, "Search 42 maps...", "");

                // Header with total count
                left_col.spawn((
                    (
                        Text::new(format!("OMSI Maps ({})", catalogue.maps.len())),
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
                let total_height = (catalogue.maps.len() as f32) * item_height;
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
                            kind: ScrollKind::Maps,
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
                                            (
                                                Text::new(&map.friendly_name),
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
                                                    "{} World Tiles",
                                                    map.tile_count
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
                        spawn_scrollbar(viewport, theme, ScrollKind::Maps);

                        // Floating Scroll-To-Top button
                        spawn_scroll_to_top_button(viewport, theme, ScrollKind::Maps);
                    });
            });

            // ==========================================
            // Right Frame: 70% Map Description & Metadata
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
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    map: &MapItem,
    setup: &NewGameSetup,
) {
    // Large Headline Title
    parent.spawn((
        (
            Text::new(&map.friendly_name),
            TextFont {
                font_size: FontSize::Px(26.0),
                ..default()
            },
            TextColor(theme.scheme.get(ColorRole::Primary)),
        ),
        TypeStyleToken(TypeScale::Headline),
        ForegroundRole(ColorRole::Primary),
    ));

    // Subtitle with Map ID and Era
    parent.spawn((
        (
            Text::new(format!(
                "Map Directory: maps/{} · Era Timeline: {}",
                map.id, map.years
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

    // Metadata stats badges row
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            flex_wrap: FlexWrap::Wrap,
            margin: UiRect::vertical(Val::Px(4.0)),
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
                    Text::new("Map Scenario Description:"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::Primary),
            ));

            // The description can be long: a fixed-height scrollable text field keeps the
            // entry-point list below it reachable.
            desc_box
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(170.0),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    ScrollableContainer {
                        kind: ScrollKind::MapDescription,
                        scroll_y: 0.0,
                        max_scroll_y: 0.0,
                    },
                ))
                .with_children(|viewport| {
                    viewport
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                padding: UiRect::right(Val::Px(26.0)),
                                ..default()
                            },
                            ScrollContent {
                                kind: ScrollKind::MapDescription,
                            },
                        ))
                        .with_children(|content| {
                            content.spawn((
                                (
                                    Text::new(&map.description),
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
                    spawn_scrollbar(viewport, theme, ScrollKind::MapDescription);
                });
        });

    // Scrollable Entrypoint selection list
    crate::new_game_screen::entrypoint_selector::spawn_entrypoint_scrollable_selection(
        parent, theme, map, setup,
    );

    // Summary of current selection at bottom of tab
    spawn_tab_summary_card(parent, theme, setup);
}

/// Notice shown while the OMSI catalogue is still empty (first run, discovery in progress).
pub fn spawn_catalogue_pending_notice(parent: &mut ChildSpawnerCommands, theme: &MaterialTheme) {
    parent.spawn((
        Text::new(
            "The OMSI catalogue is being built in the background. Reopen this screen in a moment.",
        ),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
    ));
}
