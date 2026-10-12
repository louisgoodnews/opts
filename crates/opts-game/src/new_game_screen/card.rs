//! Reusable selection card widget, stat chips, and scroll buttons for new game panels.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;

/// Spawns a selectable item card with title, subtitle, and metadata specs.
pub fn spawn_selection_card<T: Component>(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    title: &str,
    subtitle: &str,
    specs: &str,
    marker: T,
    is_selected: bool,
) {
    parent
        .spawn((
            (
                Button,
                Node {
                    width: Val::Px(280.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
                    margin: UiRect::vertical(Val::Px(6.0)),
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
                (
                    Text::new(title),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(if is_selected {
                        theme.scheme.get(ColorRole::OnPrimaryContainer)
                    } else {
                        theme.scheme.get(ColorRole::OnSurface)
                    }),
                ),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::OnSurface),
            ));

            card.spawn((
                (
                    Text::new(subtitle),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));

            card.spawn((
                (
                    Text::new(specs),
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

/// Spawns a labeled stat badge chip.
pub fn spawn_stat_chip(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    label: &str,
    value: &str,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
        ))
        .with_children(|chip| {
            chip.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
            chip.spawn((
                (
                    Text::new(value),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::Primary),
            ));
        });
}

/// Spawns a list scroll control button.
pub fn spawn_scroll_btn<T: Component>(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    arrow: &str,
    marker: T,
) {
    parent
        .spawn((
            (
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
            ),
            M3Button::default(),
            marker,
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
        ))
        .with_children(|b| {
            b.spawn((
                (
                    Text::new(arrow),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurface)),
                ),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurface),
            ));
        });
}

/// Spawns a unified bottom selection summary card for notebook panels.
pub fn spawn_tab_summary_card(
    parent: &mut ChildSpawnerCommands,
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
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
        ))
        .with_children(|bar| {
            bar.spawn((
                (
                    Text::new(summary_text),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurface)),
                ),
                crate::new_game_screen::types::TabSummaryText,
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurface),
            ));
        });
}
