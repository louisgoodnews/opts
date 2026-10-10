//! Searchbar widget and helper routines for filtering lists in the new game screen.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{ClearSearchBtn, ScrollKind, SearchInputBox, SearchInputText};

/// Spawns a search bar container with search icon, text/placeholder, and clear button.
pub fn spawn_search_bar(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    kind: ScrollKind,
    placeholder: &str,
    current_query: &str,
    is_focused: bool,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(38.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(if is_focused { 2.0 } else { 1.0 })),
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                border_color: BorderColor(if is_focused {
                    theme.scheme.get(ColorRole::Primary)
                } else {
                    theme.scheme.get(ColorRole::OutlineVariant)
                }),
                background_color: BackgroundColor(
                    theme.scheme.get(ColorRole::SurfaceContainerHigh),
                ),
                ..default()
            },
            M3Button::default(),
            SearchInputBox { kind },
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
            Focusable { tab_index: 0 },
        ))
        .with_children(|bar| {
            // Left: Icon + Text
            bar.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    flex_grow: 1.0,
                    ..default()
                },
                ..default()
            })
            .with_children(|left| {
                left.spawn((
                    TextBundle::from_section(
                        opts_gui::icons::SEARCH,
                        opts_gui::icons::icon_style(16.0, theme.scheme.get(ColorRole::Primary)),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::Primary),
                ));

                let display_text = if current_query.is_empty() {
                    placeholder
                } else {
                    current_query
                };

                left.spawn((
                    TextBundle::from_section(
                        display_text,
                        TextStyle {
                            font_size: 13.0,
                            color: if current_query.is_empty() {
                                theme.scheme.get(ColorRole::OnSurfaceVariant)
                            } else {
                                theme.scheme.get(ColorRole::OnSurface)
                            },
                            ..default()
                        },
                    ),
                    SearchInputText { kind },
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(if current_query.is_empty() {
                        ColorRole::OnSurfaceVariant
                    } else {
                        ColorRole::OnSurface
                    }),
                ));
            });

            // Right: Reset / Clear Button (displayed when focused or query is not empty)
            bar.spawn((
                ButtonBundle {
                    style: Style {
                        display: if is_focused || !current_query.is_empty() {
                            Display::Flex
                        } else {
                            Display::None
                        },
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                    background_color: BackgroundColor(
                        theme.scheme.get(ColorRole::SurfaceContainerLow),
                    ),
                    ..default()
                },
                M3Button::default(),
                ClearSearchBtn { kind },
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                Focusable { tab_index: 0 },
            ))
            .with_children(|btn| {
                btn.spawn((
                    TextBundle::from_section(
                        "✖",
                        TextStyle {
                            font_size: 11.0,
                            color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::LabelSmall),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));
            });
        });
}
