//! Material Design Date Picker dialog matching Screenshot 1.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{
    CalendarHeaderDateText, CalendarMonthYearText, NewGameSetup, NextMonthBtn, PrevMonthBtn,
    SelectCalendarDayBtn,
};
use crate::new_game_screen::year_spinbox::spawn_year_spinbox;

/// Computes the number of days in a given year and month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Computes day of week (0 = Sunday, 1 = Monday, ..., 6 = Saturday) using Sakamoto's algorithm.
pub fn day_of_week(year: i32, month: u32, day: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    let m_idx = ((month - 1) as usize).min(11);
    let dow = (y + y / 4 - y / 100 + y / 400 + t[m_idx] + day as i32) % 7;
    ((dow + 7) % 7) as u32
}

/// Returns full name of a month (1 = January, ..., 12 = December).
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Returns 3-letter abbreviation of weekday (0 = Sun, ..., 6 = Sat).
pub fn weekday_short(dow: u32) -> &'static str {
    match dow {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => "Day",
    }
}

/// Formats the date string for the Material Date Picker header ("Mon, Apr 18").
pub fn format_calendar_header_date(year: i32, month: u32, day: u32) -> String {
    let dow = day_of_week(year, month, day);
    let m_name = month_name(month);
    let short_m = if m_name.len() >= 3 {
        &m_name[..3]
    } else {
        m_name
    };
    format!("{}, {} {}", weekday_short(dow), short_m, day)
}

/// Spawns the complete Material Design Date Picker card matching Screenshot 1.
pub fn spawn_material_date_picker(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
) {
    let header_date = format_calendar_header_date(setup.year, setup.month, setup.day);

    // Root Dialog Card
    parent
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(310.0),
                    height: Val::Px(400.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                background_color: BackgroundColor(
                    theme.scheme.get(ColorRole::SurfaceContainerHigh),
                ),
                ..default()
            },
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Large),
        ))
        .with_children(|dialog| {
            // 1. Top Header Banner (Primary Surface)
            dialog
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    background_color: BackgroundColor(theme.scheme.get(ColorRole::Primary)),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        TextBundle::from_section(
                            "SELECT DATE",
                            TextStyle {
                                font_size: 11.0,
                                color: theme.scheme.get(ColorRole::OnPrimary),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::LabelSmall),
                    ));

                    header
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|date_row| {
                            date_row.spawn((
                                TextBundle::from_section(
                                    header_date,
                                    TextStyle {
                                        font_size: 24.0,
                                        color: theme.scheme.get(ColorRole::OnPrimary),
                                        ..default()
                                    },
                                ),
                                CalendarHeaderDateText,
                                TypeStyleToken(TypeScale::HeadlineSmall),
                            ));

                            date_row.spawn((
                                TextBundle::from_section(
                                    "",
                                    TextStyle {
                                        font_size: 18.0,
                                        color: theme.scheme.get(ColorRole::OnPrimary),
                                        ..default()
                                    },
                                ),
                                TypeStyleToken(TypeScale::TitleMedium),
                            ));
                        });
                });

            // 2. Calendar Body Container
            dialog
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    background_color: BackgroundColor(
                        theme.scheme.get(ColorRole::SurfaceContainerHigh),
                    ),
                    ..default()
                })
                .with_children(|body| {
                    // Month & Year Selector
                    body.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::bottom(Val::Px(4.0)),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|nav_bar| {
                        // Month Selector with Prev/Next buttons
                        nav_bar
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(4.0),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|month_box| {
                                spawn_nav_arrow(month_box, theme, "<", PrevMonthBtn);
                                month_box.spawn((
                                    TextBundle::from_section(
                                        month_name(setup.month),
                                        TextStyle {
                                            font_size: 14.0,
                                            color: theme.scheme.get(ColorRole::OnSurface),
                                            ..default()
                                        },
                                    ),
                                    CalendarMonthYearText,
                                    TypeStyleToken(TypeScale::TitleSmall),
                                ));
                                spawn_nav_arrow(month_box, theme, ">", NextMonthBtn);
                            });

                        // Year Spinbox with typed input and ▲/▼ steppers
                        spawn_year_spinbox(nav_bar, theme, setup);
                    });

                    // Weekday Column Headers: S M T W T F S
                    body.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|week_row| {
                        for letter in ["S", "M", "T", "W", "T", "F", "S"] {
                            week_row.spawn((
                                TextBundle::from_section(
                                    letter,
                                    TextStyle {
                                        font_size: 12.0,
                                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                                        ..default()
                                    },
                                )
                                .with_style(Style {
                                    width: Val::Px(34.0),
                                    justify_content: JustifyContent::Center,
                                    align_content: AlignContent::Center,
                                    ..default()
                                }),
                                TypeStyleToken(TypeScale::LabelMedium),
                            ));
                        }
                    });

                    // 7-Column Day Grid (5 to 6 rows)
                    let first_dow = day_of_week(setup.year, setup.month, 1);
                    let total_days = days_in_month(setup.year, setup.month);

                    body.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|grid| {
                        let mut current_day = 1u32;
                        let mut week_slot = 0u32;

                        for _week in 0..6 {
                            if current_day > total_days {
                                break;
                            }

                            grid.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|row| {
                                for _day_col in 0..7 {
                                    if week_slot < first_dow || current_day > total_days {
                                        row.spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Px(34.0),
                                                height: Val::Px(34.0),
                                                ..default()
                                            },
                                            ..default()
                                        });
                                    } else {
                                        let day_num = current_day;
                                        let is_active = day_num == setup.day;

                                        row.spawn((
                                            ButtonBundle {
                                                style: Style {
                                                    width: Val::Px(34.0),
                                                    height: Val::Px(34.0),
                                                    align_items: AlignItems::Center,
                                                    justify_content: JustifyContent::Center,
                                                    ..default()
                                                },
                                                background_color: BackgroundColor(if is_active {
                                                    theme.scheme.get(ColorRole::Primary)
                                                } else {
                                                    Color::NONE
                                                }),
                                                border_radius: BorderRadius::all(Val::Percent(
                                                    50.0,
                                                )),
                                                ..default()
                                            },
                                            M3Button::default(),
                                            SelectCalendarDayBtn { day: day_num },
                                            Focusable { tab_index: 0 },
                                        ))
                                        .with_children(
                                            |day_cell| {
                                                day_cell.spawn((
                                                    TextBundle::from_section(
                                                        format!("{day_num}"),
                                                        TextStyle {
                                                            font_size: 13.0,
                                                            color: if is_active {
                                                                theme
                                                                    .scheme
                                                                    .get(ColorRole::OnPrimary)
                                                            } else {
                                                                theme
                                                                    .scheme
                                                                    .get(ColorRole::OnSurface)
                                                            },
                                                            ..default()
                                                        },
                                                    ),
                                                    TypeStyleToken(TypeScale::BodyMedium),
                                                ));
                                            },
                                        );

                                        current_day += 1;
                                    }
                                    week_slot += 1;
                                }
                            });
                        }
                    });
                });
        });
}

fn spawn_nav_arrow<T: Component>(
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
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            M3Button::default(),
            marker,
            Focusable { tab_index: 0 },
        ))
        .with_children(|btn| {
            btn.spawn((
                TextBundle::from_section(
                    arrow,
                    TextStyle {
                        font_size: 13.0,
                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodyMedium),
            ));
        });
}
