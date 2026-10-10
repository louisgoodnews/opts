//! Systems handling date picker, year spinbox, clock picker, scrollbar thumbs, and scroll actions.

use bevy::prelude::*;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::new_game_screen::clock_picker::{get_clock_angle, to_12h, to_24h};
use crate::new_game_screen::date_picker::{days_in_month, format_calendar_header_date, month_name};
use crate::new_game_screen::types::{
    AdjustMinuteBtn, CalendarHeaderDateText, CalendarMonthYearText, ClockDialHourContainer,
    ClockDialMinuteContainer, ClockDigitalDisplayText, ClockHandPointer, ClockHourTextBtn,
    ClockMinuteTextBtn, ClockPickerMode, LaunchSummaryText, NewGameSetup, NextMonthBtn,
    PrevMonthBtn, ScrollContent, ScrollToTopBtn, ScrollableContainer, ScrollbarThumb,
    ScrollbarTrack, SelectCalendarDayBtn, SelectClockHourBtn, SelectClockMeridianBtn,
    SelectClockMinuteBtn, TabSummaryText, YearDecrementBtn, YearIncrementBtn, YearInputBox,
    YearInputText,
};
use crate::new_game_screen::year_spinbox::{is_valid_year, MAX_VALID_YEAR, MIN_VALID_YEAR};

fn format_summary_string(setup: &NewGameSetup) -> String {
    format!(
        "Map: {} ({}) | Bus: {} ({}) | Weather: {} | {:02}:{:02}",
        setup.selected_map,
        setup.selected_entry_point,
        setup.selected_vehicle,
        setup.selected_livery,
        setup.weather_preset,
        setup.hour,
        setup.minute
    )
}

/// System handling scrollbar track clicks and floating scroll-to-top button.
pub fn handle_scroll_to_top(
    mut events: MessageReader<ButtonClicked>,
    top_btn_query: Query<&ScrollToTopBtn>,
    track_query: Query<&ScrollbarTrack>,
    mut container_query: Query<&mut ScrollableContainer>,
    mut content_query: Query<(&ScrollContent, &mut Node)>,
) {
    for event in events.read() {
        if let Ok(btn) = top_btn_query.get(event.0) {
            for mut container in &mut container_query {
                if container.kind == btn.kind {
                    container.scroll_y = 0.0;
                    for (content, mut style) in &mut content_query {
                        if content.kind == btn.kind {
                            style.top = Val::Px(0.0);
                        }
                    }
                }
            }
        } else if let Ok(track) = track_query.get(event.0) {
            for mut container in &mut container_query {
                if container.kind == track.kind {
                    container.scroll_y = (container.scroll_y + 150.0).min(container.max_scroll_y);
                    for (content, mut style) in &mut content_query {
                        if content.kind == track.kind {
                            style.top = Val::Px(-container.scroll_y);
                        }
                    }
                }
            }
        }
    }
}

/// System synchronizing scrollbar thumbs with current container scroll position.
pub fn update_scrollbar_thumbs(
    container_query: Query<&ScrollableContainer>,
    mut thumb_query: Query<(&ScrollbarThumb, &mut Node)>,
) {
    for container in &container_query {
        for (thumb, mut style) in &mut thumb_query {
            if thumb.kind == container.kind {
                let track_height = 434.0f32;
                let thumb_h = 48.0f32;
                let track_range = (track_height - thumb_h).max(0.0f32);
                let ratio = if container.max_scroll_y > 0.0 {
                    (container.scroll_y / container.max_scroll_y).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                style.top = Val::Px(ratio * track_range);
            }
        }
    }
}

/// System handling Month navigation and Day selection with dynamic visual highlighting.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_calendar_interaction(
    theme: Res<MaterialTheme>,
    mut events: MessageReader<ButtonClicked>,
    prev_query: Query<&PrevMonthBtn>,
    next_query: Query<&NextMonthBtn>,
    day_query: Query<&SelectCalendarDayBtn>,
    mut day_btns_query: Query<(
        Entity,
        &SelectCalendarDayBtn,
        &mut BackgroundColor,
        &Children,
    )>,
    header_entity_query: Query<Entity, With<CalendarHeaderDateText>>,
    month_year_entity_query: Query<Entity, With<CalendarMonthYearText>>,
    summary_entities_query: Query<Entity, Or<(With<LaunchSummaryText>, With<TabSummaryText>)>>,
    mut text_query: Query<&mut Text>,
    mut setup: ResMut<NewGameSetup>,
) {
    let mut date_changed = false;

    for event in events.read() {
        if prev_query.get(event.0).is_ok() {
            if setup.month == 1 {
                setup.year = (setup.year - 1).max(MIN_VALID_YEAR);
                setup.month = 12;
            } else {
                setup.month -= 1;
            }
            let max_d = days_in_month(setup.year, setup.month);
            setup.day = setup.day.min(max_d);
            setup.year_buffer = setup.year.to_string();
            date_changed = true;
        } else if next_query.get(event.0).is_ok() {
            if setup.month == 12 {
                setup.year = (setup.year + 1).min(MAX_VALID_YEAR);
                setup.month = 1;
            } else {
                setup.month += 1;
            }
            let max_d = days_in_month(setup.year, setup.month);
            setup.day = setup.day.min(max_d);
            setup.year_buffer = setup.year.to_string();
            date_changed = true;
        } else if let Ok(day_btn) = day_query.get(event.0) {
            setup.day = day_btn.day;
            date_changed = true;
        }
    }

    if date_changed {
        let new_header = format_calendar_header_date(setup.year, setup.month, setup.day);
        let new_my = month_name(setup.month).to_string();

        for ent in &header_entity_query {
            if let Ok(mut text) = text_query.get_mut(ent) {
                text.0 = new_header.clone();
            }
        }
        for ent in &month_year_entity_query {
            if let Ok(mut text) = text_query.get_mut(ent) {
                text.0 = new_my.clone();
            }
        }

        let primary_color = theme.scheme.get(ColorRole::Primary);
        let on_primary = theme.scheme.get(ColorRole::OnPrimary);
        let on_surface = theme.scheme.get(ColorRole::OnSurface);

        for (_entity, day_btn, mut bg, children) in &mut day_btns_query {
            let is_selected = day_btn.day == setup.day;
            bg.0 = if is_selected {
                primary_color
            } else {
                Color::NONE
            };
            for &child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].style.color =
                        if is_selected { on_primary } else { on_surface };
                }
            }
        }

        let summary_str = format_summary_string(&setup);
        for ent in &summary_entities_query {
            if let Ok(mut text) = text_query.get_mut(ent) {
                text.0 = summary_str.clone();
            }
        }
    }
}

/// System handling the Year Spinbox: increment, decrement, and direct typing.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_year_spinbox(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: MessageReader<ButtonClicked>,
    year_inc_query: Query<&YearIncrementBtn>,
    year_dec_query: Query<&YearDecrementBtn>,
    year_box_query: Query<&YearInputBox>,
    year_input_entity: Query<Entity, With<YearInputText>>,
    header_entity_query: Query<Entity, With<CalendarHeaderDateText>>,
    summary_entities_query: Query<Entity, Or<(With<LaunchSummaryText>, With<TabSummaryText>)>>,
    mut text_query: Query<&mut Text>,
    mut setup: ResMut<NewGameSetup>,
) {
    let mut year_changed = false;

    let num_digits = [
        (KeyCode::Digit0, '0'),
        (KeyCode::Numpad0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Numpad1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Numpad2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Numpad3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Numpad4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Numpad5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Numpad6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Numpad7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Numpad8, '8'),
        (KeyCode::Digit9, '9'),
        (KeyCode::Numpad9, '9'),
    ];

    if setup.year_editing {
        if keyboard.just_pressed(KeyCode::Backspace) {
            setup.year_buffer.pop();
        } else if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Escape) {
            if let Ok(val) = setup.year_buffer.parse::<i32>() {
                if is_valid_year(val) {
                    setup.year = val;
                    year_changed = true;
                }
            }
            setup.year_editing = false;
            setup.year_buffer = setup.year.to_string();
        } else {
            for &(code, ch) in &num_digits {
                if keyboard.just_pressed(code) {
                    if setup.year_buffer.len() >= 4 {
                        setup.year_buffer.clear();
                    }
                    setup.year_buffer.push(ch);
                    if setup.year_buffer.len() == 4 {
                        if let Ok(val) = setup.year_buffer.parse::<i32>() {
                            if is_valid_year(val) {
                                setup.year = val;
                                year_changed = true;
                            }
                        }
                    }
                    break;
                }
            }
        }
        for ent in &year_input_entity {
            if let Ok(mut text) = text_query.get_mut(ent) {
                text.0 = format!("{}|", setup.year_buffer);
            }
        }
    } else {
        for &(code, ch) in &num_digits {
            if keyboard.just_pressed(code) && !year_input_entity.is_empty() {
                setup.year_editing = true;
                setup.year_buffer.clear();
                setup.year_buffer.push(ch);
                for ent in &year_input_entity {
                    if let Ok(mut text) = text_query.get_mut(ent) {
                        text.0 = format!("{}|", setup.year_buffer);
                    }
                }
                break;
            }
        }
    }

    for event in events.read() {
        if year_inc_query.get(event.0).is_ok() {
            setup.year = (setup.year + 1).min(MAX_VALID_YEAR);
            setup.year_buffer = setup.year.to_string();
            year_changed = true;
        } else if year_dec_query.get(event.0).is_ok() {
            setup.year = (setup.year - 1).max(MIN_VALID_YEAR);
            setup.year_buffer = setup.year.to_string();
            year_changed = true;
        } else if year_box_query.get(event.0).is_ok() {
            setup.year_editing = true;
            setup.year_buffer.clear();
        }
    }

    if year_changed {
        let max_d = days_in_month(setup.year, setup.month);
        setup.day = setup.day.min(max_d);
        let new_header = format_calendar_header_date(setup.year, setup.month, setup.day);
        for ent in &header_entity_query {
            if let Ok(mut text) = text_query.get_mut(ent) {
                text.0 = new_header.clone();
            }
        }
        if !setup.year_editing {
            for ent in &year_input_entity {
                if let Ok(mut text) = text_query.get_mut(ent) {
                    text.0 = setup.year.to_string();
                }
            }
        }
        let summary_str = format_summary_string(&setup);
        for ent in &summary_entities_query {
            if let Ok(mut text) = text_query.get_mut(ent) {
                text.0 = summary_str.clone();
            }
        }
    }
}

/// System handling Clock Time Picker interaction: Hours/Minutes modes and input clicks.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_clock_picker_interaction(
    mut events: MessageReader<ButtonClicked>,
    hour_text_query: Query<&ClockHourTextBtn>,
    minute_text_query: Query<&ClockMinuteTextBtn>,
    hour_btn_query: Query<&SelectClockHourBtn>,
    minute_btn_query: Query<&SelectClockMinuteBtn>,
    meridian_query: Query<&SelectClockMeridianBtn>,
    min_step_query: Query<&AdjustMinuteBtn>,
    mut setup: ResMut<NewGameSetup>,
) {
    for event in events.read() {
        if hour_text_query.get(event.0).is_ok() {
            setup.clock_mode = ClockPickerMode::Hours;
        } else if minute_text_query.get(event.0).is_ok() {
            setup.clock_mode = ClockPickerMode::Minutes;
        } else if let Ok(h_btn) = hour_btn_query.get(event.0) {
            let (_, is_pm) = to_12h(setup.hour);
            setup.hour = to_24h(h_btn.hour_12, is_pm);
            setup.clock_mode = ClockPickerMode::Minutes;
        } else if let Ok(m_btn) = minute_btn_query.get(event.0) {
            setup.minute = m_btn.minute;
        } else if let Ok(mer_btn) = meridian_query.get(event.0) {
            let (h12, _) = to_12h(setup.hour);
            setup.hour = to_24h(h12, mer_btn.is_pm);
        } else if let Ok(step_btn) = min_step_query.get(event.0) {
            let new_min = (setup.minute as i32 + step_btn.minute_offset).rem_euclid(60);
            setup.minute = new_min as u32;
        }
    }
}

/// System updating Clock Time Picker visual elements: Header readouts, Dial modes, and Hand pointer.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_clock_picker_visuals(
    theme: Res<MaterialTheme>,
    setup: Res<NewGameSetup>,
    hour_text_bundle: Query<&Children, With<ClockHourTextBtn>>,
    min_text_bundle: Query<&Children, With<ClockMinuteTextBtn>>,
    mut hour_dial_query: Query<
        &mut Node,
        (
            With<ClockDialHourContainer>,
            Without<ClockDialMinuteContainer>,
            Without<ClockHandPointer>,
        ),
    >,
    mut min_dial_query: Query<
        &mut Node,
        (
            With<ClockDialMinuteContainer>,
            Without<ClockDialHourContainer>,
            Without<ClockHandPointer>,
        ),
    >,
    mut pointer_query: Query<(&mut Node, &mut Transform), With<ClockHandPointer>>,
    mut hour_btns_query: Query<
        (&SelectClockHourBtn, &mut BackgroundColor, &Children),
        Without<SelectClockMinuteBtn>,
    >,
    mut min_btns_query: Query<
        (&SelectClockMinuteBtn, &mut BackgroundColor, &Children),
        Without<SelectClockHourBtn>,
    >,
    digital_display_query: Query<Entity, With<ClockDigitalDisplayText>>,
    summary_entities_query: Query<Entity, Or<(With<LaunchSummaryText>, With<TabSummaryText>)>>,
    mut text_query: Query<&mut Text>,
) {
    if !setup.is_changed() {
        return;
    }

    let teal_accent = Color::srgb(0.0, 0.588, 0.533);
    let active_white = Color::WHITE;
    let muted_white = Color::srgba(1.0, 1.0, 1.0, 0.65);
    let (h12, _) = to_12h(setup.hour);

    // 0. Update Digital Readout Display
    let time_str = format!("{h12}:{:02}", setup.minute);
    for ent in &digital_display_query {
        if let Ok(mut text) = text_query.get_mut(ent) {
            text.0 = time_str.clone();
        }
    }

    // 1. Update Header Readout text
    for children in &hour_text_bundle {
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.0 = format!("{h12}");
                text.sections[0].style.color = if setup.clock_mode == ClockPickerMode::Hours {
                    active_white
                } else {
                    muted_white
                };
            }
        }
    }
    for children in &min_text_bundle {
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.0 = format!("{:02}", setup.minute);
                text.sections[0].style.color = if setup.clock_mode == ClockPickerMode::Minutes {
                    active_white
                } else {
                    muted_white
                };
            }
        }
    }

    // 2. Toggle Radial Dial container visibility
    for mut style in &mut hour_dial_query {
        style.display = if setup.clock_mode == ClockPickerMode::Hours {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut style in &mut min_dial_query {
        style.display = if setup.clock_mode == ClockPickerMode::Minutes {
            Display::Flex
        } else {
            Display::None
        };
    }

    // 3. Update pointer hand orientation
    let center = 105.0f32;
    let radius = 74.0f32;
    let angle = get_clock_angle(setup.clock_mode, h12, setup.minute);
    let mid_x = center + (radius / 2.0) * angle.cos();
    let mid_y = center + (radius / 2.0) * angle.sin();

    for (mut style, mut transform) in &mut pointer_query {
        style.left = Val::Px(mid_x - (radius / 2.0));
        style.top = Val::Px(mid_y - 1.0);
        transform.rotation = Quat::from_rotation_z(angle);
    }

    // 4. Update Hour Dial Highlights
    for (h_btn, mut bg, children) in &mut hour_btns_query {
        let active = h_btn.hour_12 == h12;
        bg.0 = if active { teal_accent } else { Color::NONE };
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.sections[0].style.color = if active {
                    Color::WHITE
                } else {
                    theme.scheme.get(ColorRole::OnSurface)
                };
            }
        }
    }

    // 5. Update Minute Dial Highlights
    for (m_btn, mut bg, children) in &mut min_btns_query {
        let active = setup.minute / 5 == m_btn.minute / 5;
        bg.0 = if active { teal_accent } else { Color::NONE };
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.sections[0].style.color = if active {
                    Color::WHITE
                } else {
                    theme.scheme.get(ColorRole::OnSurface)
                };
            }
        }
    }

    let summary_str = format_summary_string(&setup);
    for ent in &summary_entities_query {
        if let Ok(mut text) = text_query.get_mut(ent) {
            text.0 = summary_str.clone();
        }
    }
}
