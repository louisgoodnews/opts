//! The stepper arithmetic: date and time steps, the valid date and the value texts.

use opts_bevy::environment::days_in_month;
use opts_bevy::CalendarDate;

use super::field::{Field, YEARS};

/// Changes a date field by `delta` steps: days and months wrap, years clamp to [`YEARS`], and
/// the day is kept inside the month (31 January + 1 month = 28/29 February). Time fields are
/// ignored.
pub fn step_date(date: &mut CalendarDate, field: Field, delta: i32) {
    match field {
        Field::Day => {
            let n = days_in_month(date.year, date.month).max(1) as i32;
            date.day = ((date.day as i32 - 1 + delta).rem_euclid(n) + 1) as u32;
        }
        Field::Month => {
            date.month = ((date.month as i32 - 1 + delta).rem_euclid(12) + 1) as u32;
        }
        Field::Year => date.year = (date.year + delta).clamp(*YEARS.start(), *YEARS.end()),
        Field::Hour | Field::Minute => {}
    }
    *date = clamp_date(*date);
}

/// Changes a time field by `delta` steps: hours wrap, minutes move by 5 on the 5-minute grid
/// (by 1 when `fine`) and wrap. Date fields are ignored.
pub fn step_time(hour: &mut u32, minute: &mut u32, field: Field, delta: i32, fine: bool) {
    match field {
        Field::Hour => *hour = (*hour as i32 + delta).rem_euclid(24) as u32,
        Field::Minute => {
            let step = if fine { 1 } else { 5 };
            let m = *minute as i32 + delta * step;
            *minute = if fine {
                m.rem_euclid(60)
            } else {
                (m.div_euclid(5) * 5).rem_euclid(60)
            } as u32;
        }
        Field::Day | Field::Month | Field::Year => {}
    }
    *hour = (*hour).min(23);
    *minute = (*minute).min(59);
}

/// A valid date inside [`YEARS`].
pub fn clamp_date(date: CalendarDate) -> CalendarDate {
    CalendarDate::clamped(
        date.year.clamp(*YEARS.start(), *YEARS.end()),
        date.month,
        date.day,
    )
}

/// Message key of a month name.
pub fn month_key(month: u32) -> &'static str {
    const KEYS: [&str; 12] = [
        "wiz-month-1",
        "wiz-month-2",
        "wiz-month-3",
        "wiz-month-4",
        "wiz-month-5",
        "wiz-month-6",
        "wiz-month-7",
        "wiz-month-8",
        "wiz-month-9",
        "wiz-month-10",
        "wiz-month-11",
        "wiz-month-12",
    ];
    KEYS[(month.clamp(1, 12) - 1) as usize]
}

/// The value text of a date field (`month_name` turns a month into its localised name).
pub fn date_value(date: CalendarDate, field: Field, month_name: impl Fn(u32) -> String) -> String {
    match field {
        Field::Day => date.day.to_string(),
        Field::Month => month_name(date.month),
        Field::Year => date.year.to_string(),
        Field::Hour | Field::Minute => String::new(),
    }
}

/// The value text of a time field.
pub fn time_value(hour: u32, minute: u32, field: Field) -> String {
    match field {
        Field::Hour => format!("{hour:02}"),
        Field::Minute => format!("{minute:02}"),
        _ => String::new(),
    }
}
