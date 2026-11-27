//! The holiday calendar and the day kind a date selects.

/// A public holiday.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicHoliday {
    /// The day, `yyyymmdd`.
    pub date: u32,
    /// The name as the file gives it (localised per language file).
    pub name: String,
}

/// A school holiday period, both ends included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolHoliday {
    /// First day, `yyyymmdd`.
    pub start: u32,
    /// Last day, `yyyymmdd`.
    pub end: u32,
    /// The name, e.g. `Sommer`.
    pub name: String,
}

/// The day kind a date selects for the `[set_day_of_week]` schedules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayKind {
    /// Monday to Friday.
    Weekday,
    /// Saturday.
    Saturday,
    /// Sunday, or a public holiday.
    Sunday,
}

/// The holidays of a map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Holidays {
    /// Public holidays in file order.
    pub public: Vec<PublicHoliday>,
    /// School holiday periods in file order.
    pub school: Vec<SchoolHoliday>,
}

impl Holidays {
    /// Whether `date` is a public holiday.
    #[must_use]
    pub fn is_public(&self, date: u32) -> bool {
        self.public.iter().any(|h| h.date == date)
    }

    /// Whether `date` lies in a school holiday period (both ends included).
    #[must_use]
    pub fn is_school(&self, date: u32) -> bool {
        self.school.iter().any(|h| (h.start..=h.end).contains(&date))
    }

    /// The day kind of `date`. A public holiday selects the Sunday schedule: the density
    /// profiles treat holidays as Sundays too (`lists/density.rs`), but the file does not say
    /// it; that is the open ❓ of the design, recorded here.
    #[must_use]
    pub fn day_kind(&self, date: u32) -> DayKind {
        if self.is_public(date) {
            return DayKind::Sunday;
        }
        match weekday(date) {
            6 => DayKind::Saturday,
            7 => DayKind::Sunday,
            _ => DayKind::Weekday,
        }
    }
}

/// The ISO weekday of `date` (`yyyymmdd`): `1` Monday … `7` Sunday. Proleptic Gregorian, from
/// the day count since 1970-01-01 (a Thursday).
#[must_use]
pub fn weekday(date: u32) -> u32 {
    let (mut y, m, d) = (
        i64::from(date / 10_000),
        i64::from(date / 100 % 100),
        i64::from(date % 100),
    );
    if m <= 2 {
        y -= 1;
    }
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days + 3).rem_euclid(7) as u32 + 1
}
