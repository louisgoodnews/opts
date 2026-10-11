//! Material Design Clock Time Picker dialog supporting dual Hours and Minutes modes.

use std::f32::consts::PI;

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::clock_controls::{
    spawn_dial_btn, spawn_meridian_btn, spawn_minute_step_btn,
};
use crate::new_game_screen::types::{
    ClockDialHourContainer, ClockDialMinuteContainer, ClockDigitalDisplayText, ClockHandPointer,
    ClockHourTextBtn, ClockMinuteTextBtn, ClockPickerMode, NewGameSetup, SelectClockHourBtn,
    SelectClockMinuteBtn,
};

/// Converts 24-hour format into (12-hour digit 1..=12, is_pm).
pub fn to_12h(hour_24: u32) -> (u32, bool) {
    let is_pm = hour_24 >= 12;
    let h12 = match hour_24 % 12 {
        0 => 12,
        other => other,
    };
    (h12, is_pm)
}

/// Converts (12-hour digit 1..=12, is_pm) into 24-hour format.
pub fn to_24h(hour_12: u32, is_pm: bool) -> u32 {
    let base = if hour_12 == 12 { 0 } else { hour_12 };
    if is_pm {
        base + 12
    } else {
        base
    }
}

/// Calculates hand rotation angle in radians for given hour (1..=12) or minute (0..=59).
pub fn get_clock_angle(mode: ClockPickerMode, h12: u32, minute: u32) -> f32 {
    match mode {
        ClockPickerMode::Hours => (h12 as f32) * (PI / 6.0) - (PI / 2.0),
        ClockPickerMode::Minutes => ((minute as f32) / 60.0) * (2.0 * PI) - (PI / 2.0),
    }
}

/// Spawns the complete Material Design Clock Time Picker card with dual radial modes.
pub fn spawn_material_clock_picker(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
) {
    let (h12, is_pm) = to_12h(setup.hour);
    let meridian_str = if is_pm { "PM" } else { "AM" };

    let teal_accent = Color::srgb(0.0, 0.588, 0.533);
    let teal_light = Color::srgb(0.35, 0.75, 0.70);
    let active_white = Color::WHITE;
    let muted_white = Color::srgba(1.0, 1.0, 1.0, 0.65);

    // Root Dialog Card
    parent
        .spawn((
            (
                Node {
                    width: Val::Px(310.0),
                    height: Val::Px(400.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
            ),
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Large),
        ))
        .with_children(|dialog| {
            // 1. Top Header Banner: Interactive "HH : MM AM/PM"
            dialog
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(14.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Baseline,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(teal_accent),
                ))
                .with_children(|header| {
                    // Digital display entity for test query synchronization
                    header.spawn((
                        (
                            Text::new(format!("{h12}:{:02}", setup.minute)),
                            TextFont {
                                font_size: FontSize::Px(0.1),
                                ..default()
                            },
                            TextColor(Color::NONE),
                            Node {
                                display: Display::None,
                                ..default()
                            },
                        ),
                        ClockDigitalDisplayText,
                    ));

                    let hour_col = if setup.clock_mode == ClockPickerMode::Hours {
                        active_white
                    } else {
                        muted_white
                    };
                    header
                        .spawn((
                            (
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ),
                            M3Button::default(),
                            ClockHourTextBtn,
                            Focusable { tab_index: 0 },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                (
                                    Text::new(format!("{h12}")),
                                    TextFont {
                                        font_size: FontSize::Px(38.0),
                                        ..default()
                                    },
                                    TextColor(hour_col),
                                ),
                                TypeStyleToken(TypeScale::DisplaySmall),
                            ));
                        });

                    header.spawn((
                        (
                            Text::new(":"),
                            TextFont {
                                font_size: FontSize::Px(38.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ),
                        TypeStyleToken(TypeScale::DisplaySmall),
                    ));

                    let min_col = if setup.clock_mode == ClockPickerMode::Minutes {
                        active_white
                    } else {
                        muted_white
                    };
                    header
                        .spawn((
                            (
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ),
                            M3Button::default(),
                            ClockMinuteTextBtn,
                            Focusable { tab_index: 0 },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                (
                                    Text::new(format!("{:02}", setup.minute)),
                                    TextFont {
                                        font_size: FontSize::Px(38.0),
                                        ..default()
                                    },
                                    TextColor(min_col),
                                ),
                                TypeStyleToken(TypeScale::DisplaySmall),
                            ));
                        });

                    header.spawn((
                        (
                            Text::new(meridian_str),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(teal_light),
                        ),
                        TypeStyleToken(TypeScale::TitleMedium),
                    ));
                });

            // 2. Clock Body Container
            dialog
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
                ))
                .with_children(|body| {
                    // Circular Dial Disc (210px x 210px)
                    body.spawn((
                        Node {
                            width: Val::Px(210.0),
                            height: Val::Px(210.0),
                            position_type: PositionType::Relative,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(Val::Percent(50.0)),
                            ..default()
                        },
                        BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLow)),
                    ))
                    .with_children(|disc| {
                        // Center Pivot Dot
                        disc.spawn((
                            Node {
                                width: Val::Px(6.0),
                                height: Val::Px(6.0),
                                position_type: PositionType::Absolute,
                                left: Val::Px(102.0),
                                top: Val::Px(102.0),
                                border_radius: BorderRadius::all(Val::Percent(50.0)),
                                ..default()
                            },
                            BackgroundColor(teal_accent),
                        ));

                        // Center Hand Line
                        let center = 105.0f32;
                        let radius = 74.0f32;
                        let angle = get_clock_angle(setup.clock_mode, h12, setup.minute);
                        let mid_x = center + (radius / 2.0) * angle.cos();
                        let mid_y = center + (radius / 2.0) * angle.sin();

                        disc.spawn((
                            (
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(mid_x - (radius / 2.0)),
                                    top: Val::Px(mid_y - 1.0),
                                    width: Val::Px(radius),
                                    height: Val::Px(2.0),
                                    ..default()
                                },
                                BackgroundColor(teal_accent),
                                UiTransform::from_rotation(Rot2::radians(angle)),
                            ),
                            ClockHandPointer,
                        ));

                        // Radial Hours Container (1..=12)
                        disc.spawn((
                            (Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(210.0),
                                height: Val::Px(210.0),
                                display: if setup.clock_mode == ClockPickerMode::Hours {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                                ..default()
                            }),
                            ClockDialHourContainer,
                        ))
                        .with_children(|hour_dial| {
                            for h in 1..=12 {
                                let active = h == h12;
                                spawn_dial_btn(
                                    hour_dial,
                                    105.0,
                                    74.0,
                                    h as usize,
                                    format!("{h}"),
                                    active,
                                    teal_accent,
                                    theme,
                                    SelectClockHourBtn { hour_12: h },
                                );
                            }
                        });

                        // Radial Minutes Container (00, 05, ..., 55)
                        disc.spawn((
                            (Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(210.0),
                                height: Val::Px(210.0),
                                display: if setup.clock_mode == ClockPickerMode::Minutes {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                                ..default()
                            }),
                            ClockDialMinuteContainer,
                        ))
                        .with_children(|min_dial| {
                            for i in 0..12 {
                                let m = i * 5;
                                let active = setup.minute / 5 == i;
                                spawn_dial_btn(
                                    min_dial,
                                    105.0,
                                    74.0,
                                    i as usize,
                                    format!("{m:02}"),
                                    active,
                                    teal_accent,
                                    theme,
                                    SelectClockMinuteBtn { minute: m },
                                );
                            }
                        });
                    });

                    // AM / PM and Quick Minute Adjustment Controls
                    body.spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|mer_box| {
                            spawn_meridian_btn(mer_box, theme, "AM", false, !is_pm, teal_accent);
                            spawn_meridian_btn(mer_box, theme, "PM", true, is_pm, teal_accent);
                        });

                        row.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|min_box| {
                            spawn_minute_step_btn(min_box, theme, "-15m", -15);
                            spawn_minute_step_btn(min_box, theme, "+15m", 15);
                        });
                    });
                });
        });
}
