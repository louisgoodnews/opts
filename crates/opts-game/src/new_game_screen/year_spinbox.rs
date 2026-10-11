//! Year spinbox widget with direct numeric typing and increment/decrement steppers.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{
    NewGameSetup, YearDecrementBtn, YearIncrementBtn, YearInputBox, YearInputText,
};

/// Valid minimum OMSI simulation departure year.
pub const MIN_VALID_YEAR: i32 = 1850;
/// Valid maximum OMSI simulation departure year.
pub const MAX_VALID_YEAR: i32 = 2050;

/// Validates whether a given year falls within the supported simulation range.
pub fn is_valid_year(year: i32) -> bool {
    (MIN_VALID_YEAR..=MAX_VALID_YEAR).contains(&year)
}

/// Spawns the year spinbox with direct clickable text input and ▲/▼ increment/decrement steppers.
pub fn spawn_year_spinbox(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest)),
        ))
        .with_children(|spinbox| {
            // Clickable text input
            spinbox
                .spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                            min_width: Val::Px(48.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ),
                    M3Button::default(),
                    YearInputBox,
                    Focusable { tab_index: 0 },
                ))
                .with_children(|input| {
                    input.spawn((
                        (
                            Text::new(format!("{}", setup.year)),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSurface)),
                        ),
                        YearInputText,
                        TypeStyleToken(TypeScale::BodyMedium),
                    ));
                });

            // Vertical Steppers: ▲ and ▼
            spinbox
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        border: UiRect {
                            left: Val::Px(1.0),
                            ..default()
                        },
                        ..default()
                    },
                    BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                ))
                .with_children(|steppers| {
                    // ▲ Increment
                    steppers
                        .spawn((
                            (
                                Button,
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(11.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ),
                            M3Button::default(),
                            YearIncrementBtn,
                            Focusable { tab_index: 0 },
                        ))
                        .with_children(|b| {
                            b.spawn((
                                (
                                    Text::new("+"),
                                    TextFont {
                                        font_size: FontSize::Px(9.0),
                                        ..default()
                                    },
                                    TextColor(theme.scheme.get(ColorRole::Primary)),
                                ),
                                TypeStyleToken(TypeScale::LabelSmall),
                            ));
                        });

                    // ▼ Decrement
                    steppers
                        .spawn((
                            (
                                Button,
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(11.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ),
                            M3Button::default(),
                            YearDecrementBtn,
                            Focusable { tab_index: 0 },
                        ))
                        .with_children(|b| {
                            b.spawn((
                                (
                                    Text::new("-"),
                                    TextFont {
                                        font_size: FontSize::Px(9.0),
                                        ..default()
                                    },
                                    TextColor(theme.scheme.get(ColorRole::Primary)),
                                ),
                                TypeStyleToken(TypeScale::LabelSmall),
                            ));
                        });
                });
        });
}
