//! UI button builders and widgets for the Material Design Clock Time Picker.

use std::f32::consts::PI;

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{AdjustMinuteBtn, SelectClockMeridianBtn};

/// Spawns a radial number button on the clock face dial.
#[allow(clippy::too_many_arguments)]
pub fn spawn_dial_btn<T: Component>(
    parent: &mut ChildSpawnerCommands,
    center: f32,
    radius: f32,
    step_idx: usize,
    label: String,
    is_active: bool,
    teal: Color,
    theme: &MaterialTheme,
    marker: T,
) {
    let angle = (step_idx as f32) * (PI / 6.0) - (PI / 2.0);
    let size = 32.0;
    let left = center + radius * angle.cos() - (size / 2.0);
    let top = center + radius * angle.sin() - (size / 2.0);

    parent
        .spawn((
            (
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(top),
                    width: Val::Px(size),
                    height: Val::Px(size),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(if is_active { teal } else { Color::NONE }),
            ),
            M3Button::default(),
            marker,
            Focusable { tab_index: 0 },
        ))
        .with_children(|btn| {
            btn.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(if is_active {
                        Color::WHITE
                    } else {
                        theme.scheme.get(ColorRole::OnSurface)
                    }),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
            ));
        });
}

/// Spawns an AM or PM toggle pill button.
pub fn spawn_meridian_btn(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    label: &str,
    is_pm: bool,
    is_active: bool,
    teal: Color,
) {
    parent
        .spawn((
            (
                Button,
                Node {
                    width: Val::Px(36.0),
                    height: Val::Px(28.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BorderColor::all(if is_active {
                    teal
                } else {
                    theme.scheme.get(ColorRole::OutlineVariant)
                }),
                BackgroundColor(if is_active { teal } else { Color::NONE }),
            ),
            M3Button::default(),
            SelectClockMeridianBtn { is_pm },
            Focusable { tab_index: 0 },
        ))
        .with_children(|btn| {
            btn.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(if is_active {
                        Color::WHITE
                    } else {
                        theme.scheme.get(ColorRole::OnSurfaceVariant)
                    }),
                ),
                TypeStyleToken(TypeScale::LabelMedium),
            ));
        });
}

/// Spawns a -15m or +15m quick minute offset button.
pub fn spawn_minute_step_btn(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    label: &str,
    offset: i32,
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
                BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest)),
            ),
            M3Button::default(),
            AdjustMinuteBtn {
                minute_offset: offset,
            },
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
            Focusable { tab_index: 0 },
        ))
        .with_children(|btn| {
            btn.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::LabelSmall),
            ));
        });
}

/// Spawns a text action button like CANCEL or OK.
pub fn spawn_clock_action_btn(parent: &mut ChildSpawnerCommands, label: &str, teal: Color) {
    parent
        .spawn((
            (
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            M3Button::default(),
            Focusable { tab_index: 0 },
        ))
        .with_children(|btn| {
            btn.spawn((
                (
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(teal),
                ),
                TypeStyleToken(TypeScale::LabelLarge),
            ));
        });
}
