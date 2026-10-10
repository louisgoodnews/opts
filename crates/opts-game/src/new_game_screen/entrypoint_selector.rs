//! Scrollable entrypoint selection widget for the map detail panel.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::scroll_bar::{spawn_scroll_to_top_button, spawn_scrollbar};
use crate::new_game_screen::types::{
    EntryPointCardItem, MapItem, NewGameSetup, ScrollContent, ScrollKind, ScrollableContainer,
    SelectEntryPointBtn,
};

/// Spawns the scrollable entrypoint selection container matching map selection list style.
pub fn spawn_entrypoint_scrollable_selection(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    map: &MapItem,
    setup: &NewGameSetup,
) {
    parent.spawn((
        (
            Text::new("Select Starting Spawn Terminal (Scrollable):"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(theme.scheme.get(ColorRole::Primary)),
        ),
        TypeStyleToken(TypeScale::TitleSmall),
        ForegroundRole(ColorRole::Primary),
    ));

    let item_height = 52.0;
    let viewport_height = 150.0;
    let total_height = map.entry_points.len() as f32 * item_height;
    let max_scroll_y = (total_height - viewport_height).max(0.0);

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(viewport_height),
                position_type: PositionType::Relative,
                overflow: Overflow::clip_y(),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest)),
        ))
        .with_children(|viewport| {
            viewport
                .spawn((
                    (Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip_y(),
                        ..default()
                    }),
                    ScrollableContainer {
                        kind: ScrollKind::EntryPoints,
                        scroll_y: 0.0,
                        max_scroll_y,
                    },
                ))
                .with_children(|scroll_container| {
                    scroll_container
                        .spawn((
                            (Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0))
                                    .with_right(Val::Px(24.0)),
                                ..default()
                            }),
                            ScrollContent {
                                kind: ScrollKind::EntryPoints,
                            },
                        ))
                        .with_children(|content| {
                            for ep in &map.entry_points {
                                let is_selected = ep == &setup.selected_entry_point;
                                let border_c = if is_selected {
                                    theme.scheme.get(ColorRole::Primary)
                                } else {
                                    theme.scheme.get(ColorRole::OutlineVariant)
                                };
                                let bg_c = if is_selected {
                                    theme.scheme.get(ColorRole::PrimaryContainer)
                                } else {
                                    theme.scheme.get(ColorRole::SurfaceContainerLow)
                                };

                                content
                                    .spawn((
                                        (
                                            Button,
                                            Node {
                                                width: Val::Percent(100.0),
                                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::SpaceBetween,
                                                border: UiRect::all(Val::Px(if is_selected {
                                                    2.0
                                                } else {
                                                    1.0
                                                })),
                                                ..default()
                                            },
                                            BorderColor::all(border_c),
                                            BackgroundColor(bg_c),
                                        ),
                                        M3Button::default(),
                                        SelectEntryPointBtn {
                                            entry_point: ep.clone(),
                                        },
                                        EntryPointCardItem {
                                            entry_point: ep.clone(),
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
                                        btn.spawn(Node {
                                            flex_direction: FlexDirection::Row,
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(8.0),
                                            ..default()
                                        })
                                        .with_children(
                                            |row| {
                                                row.spawn((
                                                    (Text::new(opts_gui::icons::DIRECTIONS_BUS), /* TODO text style opts_gui::icons::icon_style(
                                                            16.0,
                                                            theme.scheme.get(ColorRole::Primary),
                                                        ) */),
                                                    TypeStyleToken(TypeScale::LabelMedium),
                                                ));

                                                row.spawn((
                                                    (
                                                        Text::new(ep),
                                                        TextFont {
                                                            font_size: FontSize::Px(12.0),
                                                            ..default()
                                                        },
                                                        TextColor(if is_selected {
                                                            theme
                                                                .scheme
                                                                .get(ColorRole::OnPrimaryContainer)
                                                        } else {
                                                            theme.scheme.get(ColorRole::OnSurface)
                                                        }),
                                                    ),
                                                    TypeStyleToken(TypeScale::LabelMedium),
                                                    ForegroundRole(ColorRole::OnSurface),
                                                ));
                                            },
                                        );

                                        btn.spawn((
                                            (
                                                Text::new(if is_selected {
                                                    "● Active Terminal"
                                                } else {
                                                    "Spawn Terminal"
                                                }),
                                                TextFont {
                                                    font_size: FontSize::Px(11.0),
                                                    ..default()
                                                },
                                                TextColor(if is_selected {
                                                    theme.scheme.get(ColorRole::Primary)
                                                } else {
                                                    theme.scheme.get(ColorRole::Outline)
                                                }),
                                            ),
                                            TypeStyleToken(TypeScale::LabelSmall),
                                            ForegroundRole(ColorRole::Outline),
                                        ));
                                    });
                            }
                        });
                });

            spawn_scrollbar(viewport, theme, ScrollKind::EntryPoints);
            spawn_scroll_to_top_button(viewport, theme, ScrollKind::EntryPoints);
        });
}
