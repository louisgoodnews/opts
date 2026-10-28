//! The logic of *Plan a drive* without any UI: steps, validity, the quick times, the date and time
//! arithmetic of the fixed-size steppers and the one-line summary.
//!
//! The specification is `docs/src/roadmap/phase-9/wizard-and-loading.md`.

use bevy::prelude::*;

use opts_bevy::CalendarDate;

use super::types::{GameCatalogue, NewGameSetup};
use crate::shell::calendar;

/// The steps of the wizard; *Trip* is shown only when the plan has runs to offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlannerStep {
    /// Choose the map and the start point.
    #[default]
    Where,
    /// Choose the vehicle, its livery and equipment.
    What,
    /// Choose date, time and weather.
    When,
    /// Optionally choose a timetable run to drive (M-0145): its first stop replaces the start
    /// point.
    Trip,
    /// Look at everything and drive.
    Review,
}

impl PlannerStep {
    /// All steps in order.
    pub const ALL: [Self; 5] = [
        Self::Where,
        Self::What,
        Self::When,
        Self::Trip,
        Self::Review,
    ];

    /// Position in [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// The next step, if any.
    pub fn next(self) -> Option<Self> {
        Self::ALL.get(self.index() + 1).copied()
    }

    /// The previous step, if any.
    pub fn prev(self) -> Option<Self> {
        self.index().checked_sub(1).map(|i| Self::ALL[i])
    }

    /// Stable id fragment (`where`, `what`, ...), used in `UiId`s.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Where => "where",
            Self::What => "what",
            Self::When => "when",
            Self::Trip => "trip",
            Self::Review => "review",
        }
    }
}

/// The wizard's own state.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PlannerState {
    /// The step being shown.
    pub step: PlannerStep,
    /// The step whose body currently exists on screen.
    pub built: Option<PlannerStep>,
    /// Notes about remembered choices that no longer exist (shown once in the step).
    pub notes: Vec<Note>,
    /// The date was chosen by the user or remembered from the last drive: choosing a map keeps
    /// it instead of setting the map's default date ([`default_map_date`]).
    pub date_chosen: bool,
    /// The time of day was chosen by the user or remembered from the last drive: choosing a
    /// map keeps it instead of setting the map's default time ([`default_start_clock`]).
    pub time_chosen: bool,
    /// Where the time of day shown comes from while it is still a default.
    pub time_source: Option<TimeSource>,
    /// The plan has timetable runs to offer: the step *Trip* is shown.
    pub trips_offered: bool,
}

impl PlannerState {
    /// Whether `step` is part of the wizard now.
    #[must_use]
    pub fn shows(&self, step: PlannerStep) -> bool {
        step != PlannerStep::Trip || self.trips_offered
    }

    /// The steps shown, in order.
    #[must_use]
    pub fn shown_steps(&self) -> Vec<PlannerStep> {
        PlannerStep::ALL
            .into_iter()
            .filter(|s| self.shows(*s))
            .collect()
    }

    /// The shown step after the current one.
    #[must_use]
    pub fn next_step(&self) -> Option<PlannerStep> {
        let mut s = self.step.next();
        while let Some(step) = s.filter(|s| !self.shows(*s)) {
            s = step.next();
        }
        s
    }

    /// The shown step before the current one.
    #[must_use]
    pub fn prev_step(&self) -> Option<PlannerStep> {
        let mut s = self.step.prev();
        while let Some(step) = s.filter(|s| !self.shows(*s)) {
            s = step.prev();
        }
        s
    }
}

/// Where a default time of day comes from (shown next to the time in step When).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    /// The map's saved clock (`laststn.osn` `[time]`).
    Map,
    /// The device's current local time.
    Now,
}

/// A one-line remark the wizard shows on a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// The remembered map is not installed any more.
    MapMissing,
    /// The remembered vehicle is not installed any more.
    VehicleMissing,
}

/// Why a drive cannot start (the reason shown on the disabled control).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// No map is chosen or it is not in the catalogue.
    NoMap,
    /// The chosen map has start points but none is chosen.
    NoStartPoint,
    /// No vehicle is chosen or it is not in the catalogue.
    NoVehicle,
}

/// Whether the choices make a valid drive.
///
/// # Errors
/// The first problem found, in the order of the steps.
pub fn validate(setup: &NewGameSetup, catalogue: &GameCatalogue) -> Result<(), Invalid> {
    let Some(map) = catalogue
        .maps
        .iter()
        .find(|m| m.friendly_name == setup.selected_map)
    else {
        return Err(Invalid::NoMap);
    };
    if !map.entry_points.is_empty() && !map.entry_points.contains(&setup.selected_entry_point) {
        return Err(Invalid::NoStartPoint);
    }
    // The key is the display name, or the id when two vehicles share that name.
    if !setup.no_vehicle && catalogue.find_vehicle(&setup.selected_vehicle).is_none() {
        return Err(Invalid::NoVehicle);
    }
    Ok(())
}

/// Makes the selection valid when the catalogue allows it: an unknown map becomes the first
/// map (with its first start point), an unknown start point the map's first, an unknown vehicle
/// the preferred default (`MAN_SD77`) or the first. Nothing happens while the lists are empty.
pub fn ensure_valid(setup: &mut NewGameSetup, catalogue: &GameCatalogue) {
    if let Some(map) = catalogue.map_by_name(&setup.selected_map) {
        if !map.entry_points.is_empty() && !map.entry_points.contains(&setup.selected_entry_point) {
            setup.selected_entry_point.clone_from(&map.entry_points[0]);
        }
    } else if let Some(map) = catalogue.maps.first() {
        setup.selected_map.clone_from(&map.friendly_name);
        setup.selected_entry_point = map.entry_points.first().cloned().unwrap_or_default();
    }
    if !setup.no_vehicle && catalogue.find_vehicle(&setup.selected_vehicle).is_none() {
        if let Some(v) = catalogue
            .vehicles
            .iter()
            .find(|v| v.id.eq_ignore_ascii_case("MAN_SD77"))
            .or_else(|| catalogue.vehicles.first())
        {
            setup.selected_vehicle = catalogue.vehicle_key(v);
            setup.selected_livery.clear();
        }
    }
}

/// A quick time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickTime {
    /// Message key of the chip label.
    pub key: &'static str,
    /// Hour, or `None` for *Now* (the wall clock).
    pub hour: Option<u32>,
    /// Minute (ignored for *Now*).
    pub minute: u32,
}

/// The quick chips of step *When*, in order.
pub const QUICK_TIMES: [QuickTime; 5] = [
    QuickTime {
        key: "wiz-when-now",
        hour: None,
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-morning",
        hour: Some(8),
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-noon",
        hour: Some(12),
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-evening",
        hour: Some(18),
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-night",
        hour: Some(22),
        minute: 0,
    },
];

/// Rounds a time of day to 5 minutes (a result of 24:00 wraps to 00:00).
pub fn round_to_five(hour: u32, minute: u32) -> (u32, u32) {
    let total = ((hour * 60 + minute + 2) / 5) * 5;
    ((total / 60) % 24, total % 60)
}

/// The time of day a chip sets, given the wall clock for *Now*.
pub fn quick_time_of(chip: &QuickTime, wall: (u32, u32)) -> (u32, u32) {
    match chip.hour {
        Some(h) => (h, chip.minute),
        None => round_to_five(wall.0, wall.1),
    }
}

/// The index of the chip that matches the current time exactly (never *Now*, which moves).
pub fn matching_quick_time(setup: &NewGameSetup) -> Option<usize> {
    QUICK_TIMES
        .iter()
        .position(|q| q.hour == Some(setup.hour) && q.minute == setup.minute)
}

pub use crate::shell::calendar::{month_key, Field, YEARS};

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

/// What a map's `global.cfg` says about its time: `[years]` and `[realyearoffset]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapEra {
    /// `[years]`: the first and last year the map is set in.
    pub years: Option<(i32, i32)>,
    /// `[realyearoffset]`: years added to today's date (0 when absent).
    pub real_year_offset: i32,
    /// The time of day (hour, minute) of the map's `laststn.osn` `[time]` block, if it has one.
    pub start_clock: Option<(u32, u32)>,
}

impl MapEra {
    /// The era of a parsed `global.cfg`.
    pub fn of(manifest: &opts_core::formats::map::MapManifest) -> Self {
        Self {
            years: manifest.years.map(|(a, b)| (a.min(b), a.max(b))),
            real_year_offset: manifest.real_year_offset.unwrap_or(0),
            start_clock: None,
        }
    }

    /// Reads `maps/<folder>/global.cfg` and the start of `laststn.osn` below the installation;
    /// `None` when neither tells anything.
    pub fn read(root: &std::path::Path, folder: &str) -> Option<Self> {
        let dir = root.join("maps").join(folder);
        let era = std::fs::read(dir.join("global.cfg"))
            .ok()
            .and_then(|bytes| opts_core::formats::map::MapManifest::parse(&bytes).ok())
            .map(|m| Self::of(&m));
        let start_clock = read_start_clock(&dir);
        if era.is_none() && start_clock.is_none() {
            return None;
        }
        Some(Self {
            start_clock,
            ..era.unwrap_or_default()
        })
    }
}

/// The time of day of `<map dir>/laststn.osn` (`[time]`): only the first few KB are read.
fn read_start_clock(map_dir: &std::path::Path) -> Option<(u32, u32)> {
    use opts_core::formats::map::{OsnTime, OSN_TIME_PREFIX_BYTES};
    use std::io::Read;
    let mut prefix = Vec::new();
    std::fs::File::open(map_dir.join("laststn.osn"))
        .ok()?
        .take(OSN_TIME_PREFIX_BYTES as u64)
        .read_to_end(&mut prefix)
        .ok()?;
    OsnTime::parse(&prefix).map(|t| t.clock())
}

/// The time of day a plan starts at when the user has not chosen one: the map's saved clock,
/// else `now`; with the source for the label.
pub fn default_start_clock(era: &MapEra, now: (u32, u32)) -> ((u32, u32), TimeSource) {
    match era.start_clock {
        Some(clock) => (clock, TimeSource::Map),
        None => (now, TimeSource::Now),
    }
}

/// The date a map starts on when the user has not chosen one: today shifted by
/// `[realyearoffset]` years (29 February becomes 28 February in a common year), clamped into
/// `[years]` as `a-01-01 ..= b-12-31`. A map without `[years]` keeps the shifted date.
pub fn default_map_date(today: CalendarDate, era: &MapEra) -> CalendarDate {
    let shifted = CalendarDate::clamped(today.year + era.real_year_offset, today.month, today.day);
    let date = match era.years {
        Some((a, b)) => shifted.clamp(CalendarDate::new(a, 1, 1), CalendarDate::new(b, 12, 31)),
        None => shifted,
    };
    calendar::clamp_date(date)
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

/// `HH:MM`.
pub fn clock_text(setup: &NewGameSetup) -> String {
    format!("{:02}:{:02}", setup.hour, setup.minute)
}

/// The map's years suggestion: `Some((from, to))` when the chosen year lies outside them.
pub fn era_hint(years: &str, year: i32) -> Option<(i32, i32)> {
    let nums: Vec<i32> = years
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| p.len() == 4)
        .filter_map(|p| p.parse().ok())
        .collect();
    let from = *nums.iter().min()?;
    let to = *nums.iter().max()?;
    (year < from || year > to).then_some((from, to))
}

#[cfg(test)]
mod tests;
