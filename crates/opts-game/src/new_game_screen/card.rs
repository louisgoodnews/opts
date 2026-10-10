//! Reusable selection card widget, stat chips, and scroll buttons for new game panels.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;

/// Spawns a selectable item card with title, subtitle, and metadata specs.
pub fn spawn_selection_card<T: Component>(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    title: &str,
    subtitle: &str,
    specs: &str,
    marker: T,
    is_selected: bool,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(280.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
                    margin: UiRect::vertical(Val::Px(6.0)),
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
            marker,
            GlassSurface::new(
                if is_selected {
                    ColorRole::PrimaryContainer
                } else {
                    ColorRole::SurfaceContainerLow
                },
                if is_selected { 0.85 } else { 0.60 },
            ),
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            Focusable { tab_index: 0 },
        ))
        .with_children(|card| {
            card.spawn((
                TextBundle::from_section(
                    title,
                    TextStyle {
                        font_size: 15.0,
                        color: if is_selected {
                            theme.scheme.get(ColorRole::OnPrimaryContainer)
                        } else {
                            theme.scheme.get(ColorRole::OnSurface)
                        },
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::OnSurface),
            ));

            card.spawn((
                TextBundle::from_section(
                    subtitle,
                    TextStyle {
                        font_size: 12.0,
                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodySmall),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));

            card.spawn((
                TextBundle::from_section(
                    specs,
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

/// Spawns a labeled stat badge chip.
pub fn spawn_stat_chip(parent: &mut ChildBuilder, theme: &MaterialTheme, label: &str, value: &str) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
            background_color: BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
            ..default()
        })
        .with_children(|chip| {
            chip.spawn((
                TextBundle::from_section(
                    label,
                    TextStyle {
                        font_size: 10.0,
                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::LabelSmall),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
            chip.spawn((
                TextBundle::from_section(
                    value,
                    TextStyle {
                        font_size: 12.0,
                        color: theme.scheme.get(ColorRole::Primary),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::Primary),
            ));
        });
}

/// Spawns a list scroll control button.
pub fn spawn_scroll_btn<T: Component>(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    arrow: &str,
    marker: T,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                background_color: BackgroundColor(
                    theme.scheme.get(ColorRole::SurfaceContainerHigh),
                ),
                ..default()
            },
            M3Button::default(),
            marker,
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
        ))
        .with_children(|b| {
            b.spawn((
                TextBundle::from_section(
                    arrow,
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

/// Spawns a unified bottom selection summary card for notebook panels.
pub fn spawn_tab_summary_card(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    setup: &crate::new_game_screen::types::NewGameSetup,
) {
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

    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
            border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
            background_color: BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
            ..default()
        })
        .with_children(|bar| {
            bar.spawn((
                TextBundle::from_section(
                    summary_text,
                    TextStyle {
                        font_size: 13.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                crate::new_game_screen::types::TabSummaryText,
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::OnSurface),
            ));
        });
}
