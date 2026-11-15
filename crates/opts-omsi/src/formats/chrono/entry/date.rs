//! Calendar dates of `Chrono.cfg`: the `yyyymmdd` value, its parts and the day-number conversion.

/// A calendar date as Chrono writes it (`yyyymmdd`); ordered like the dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ChronoDate(u32);

impl ChronoDate {
    /// The date `year-month-day` (the year is clamped into `0..=9999`).
    #[must_use]
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        let y = u32::try_from(year.clamp(0, 9999)).unwrap_or(0);
        Self(y * 10_000 + month.min(99) * 100 + day.min(99))
    }

    /// Parses `yyyymmdd` (digits only; `None` for anything else, e.g. a blank value).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        t.parse().ok().map(Self)
    }

    /// The date `days` days after 1970-01-01 (negative: before).
    #[must_use]
    pub fn from_unix_days(days: i64) -> Self {
        let (y, m, d) = civil_from_days(days);
        Self::from_ymd(i32::try_from(y).unwrap_or(0), m, d)
    }

    /// The value as written (`19891110`).
    #[must_use]
    pub fn yyyymmdd(self) -> u32 {
        self.0
    }

    /// `(year, month, day)`.
    #[must_use]
    pub fn ymd(self) -> (i32, u32, u32) {
        (
            i32::try_from(self.0 / 10_000).unwrap_or(0),
            self.0 / 100 % 100,
            self.0 % 100,
        )
    }
}

impl std::fmt::Display for ChronoDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

/// Proleptic Gregorian date of a day number counted from 1970-01-01.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}
