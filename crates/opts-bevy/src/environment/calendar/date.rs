//! The calendar: proleptic Gregorian dates, their day of the year, weekday and day bits.

use super::*;

/// A calendar date (proleptic Gregorian).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CalendarDate {
    /// Year.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Day of the month, 1–31.
    pub day: u32,
}

impl Default for CalendarDate {
    fn default() -> Self {
        Self::new(2026, 6, 29)
    }
}

impl fmt::Display for CalendarDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl CalendarDate {
    /// A date from its parts, as given (see [`Self::clamped`]).
    #[must_use]
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// A valid date: the month brought into 1–12 and the day into the month (31 February
    /// becomes the last day of February).
    #[must_use]
    pub fn clamped(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            day: day.clamp(1, days_in_month(year, month)),
        }
    }

    /// Whether the parts form a real date.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
    }

    /// Parses `YYYY-MM-DD`; `None` for anything else or an impossible date.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().splitn(3, '-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        let d = Self::new(year, month, day);
        d.is_valid().then_some(d)
    }

    /// Day of the year, 1–366.
    #[must_use]
    pub fn day_of_year(&self) -> u32 {
        const START: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let m = (self.month.clamp(1, 12) - 1) as usize;
        START[m] + self.day + u32::from(is_leap_year(self.year) && m >= 2)
    }

    /// Days since 1970-01-01 (negative before).
    #[must_use]
    pub fn days_since_epoch(&self) -> i64 {
        // Howard Hinnant's `days_from_civil`.
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01.
    #[must_use]
    pub fn from_days_since_epoch(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Self { year, month, day }
    }

    /// The date `days` later (earlier when negative).
    #[must_use]
    pub fn add_days(&self, days: i64) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch() + days)
    }

    /// ISO day of the week: 1 = Monday … 7 = Sunday.
    #[must_use]
    pub fn weekday(&self) -> u32 {
        // 1970-01-01 was a Thursday (4).
        ((self.days_since_epoch() + 3).rem_euclid(7) + 1) as u32
    }

    /// The `[set_day_of_week]` bit of the day (Monday–Friday, Saturday or Sunday; public
    /// holidays are not known ❓).
    #[must_use]
    pub fn day_bit(&self) -> u8 {
        match self.weekday() {
            6 => day::SATURDAY,
            7 => day::SUNDAY,
            _ => day::WEEKDAY,
        }
    }
}

/// Whether `year` is a leap year.
#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days of a month (0 for a month outside 1–12).
#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}
