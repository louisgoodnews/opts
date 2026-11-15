//! The date and time arithmetic of the planner: the plan's date, the steppers and today.

use bevy::prelude::*;

use opts_bevy::CalendarDate;

use super::super::types::NewGameSetup;
use crate::shell::calendar;
use super::Field;

/// Number of days of a month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    opts_bevy::environment::days_in_month(year, month)
}

/// The date of the plan.
pub fn setup_date(setup: &NewGameSetup) -> CalendarDate {
    CalendarDate::new(setup.year, setup.month, setup.day)
}

/// Sets the date of the plan.
pub fn set_setup_date(setup: &mut NewGameSetup, date: CalendarDate) {
    (setup.year, setup.month, setup.day) = (date.year, date.month, date.day);
}

/// Changes one field by `delta` steps (minutes move by 5, `fine` by 1), wrapping hours, minutes,
/// months and days, clamping years, and keeping the date valid (`shell::calendar`).
pub fn step_field(setup: &mut NewGameSetup, field: Field, delta: i32, fine: bool) {
    let mut date = setup_date(setup);
    calendar::step_date(&mut date, field, delta);
    set_setup_date(setup, date);
    calendar::step_time(&mut setup.hour, &mut setup.minute, field, delta, fine);
    clamp_date(setup);
}

/// Brings the day into the month (31 February becomes the last day of February).
pub fn clamp_date(setup: &mut NewGameSetup) {
    let date = calendar::clamp_date(setup_date(setup));
    set_setup_date(setup, date);
    setup.hour = setup.hour.min(23);
    setup.minute = setup.minute.min(59);
}

/// A fixed "today" for the map default date (screenshots and tests); `None` uses the wall clock.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Today(pub Option<CalendarDate>);

impl Today {
    /// The date this resource stands for.
    pub fn date(&self) -> CalendarDate {
        self.0.unwrap_or_else(today)
    }
}

/// Today's date on the wall clock.
pub fn today() -> CalendarDate {
    use chrono::Datelike;
    let t = chrono::Local::now().date_naive();
    CalendarDate::new(t.year(), t.month(), t.day())
}
