//! The logic of *Plan a drive* without any UI: steps, validity, the quick times, the date and time
//! arithmetic of the fixed-size steppers and the one-line summary.
//!
//! The specification is `docs/src/roadmap/phase-9/wizard-and-loading.md`.

use bevy::prelude::*;

use super::types::{GameCatalogue, NewGameSetup};

/// The four steps of the wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlannerStep {
    /// Choose the map and the start point.
    #[default]
    Where,
    /// Choose the vehicle, its livery and equipment.
    What,
    /// Choose date, time and weather.
    When,
    /// Look at everything and drive.
    Review,
}

impl PlannerStep {
    /// All steps in order.
    pub const ALL: [Self; 4] = [Self::Where, Self::What, Self::When, Self::Review];

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
    if !setup.no_vehicle
        && !catalogue
            .vehicles
            .iter()
            .any(|v| v.full_name == setup.selected_vehicle)
    {
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

/// Supported years.
pub const YEARS: std::ops::RangeInclusive<i32> = 1900..=2100;

/// Number of days of a month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// A part of the date or time the steppers change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    /// Day of the month.
    Day,
    /// Month.
    Month,
    /// Year.
    Year,
    /// Hour.
    Hour,
    /// Minute.
    Minute,
}

/// Changes one field by `delta` steps (minutes move by 5, `fine` by 1), wrapping hours, minutes,
/// months and days, clamping years, and keeping the date valid.
pub fn step_field(setup: &mut NewGameSetup, field: Field, delta: i32, fine: bool) {
    match field {
        Field::Day => {
            let n = days_in_month(setup.year, setup.month) as i32;
            setup.day = ((setup.day as i32 - 1 + delta).rem_euclid(n) + 1) as u32;
        }
        Field::Month => {
            setup.month = ((setup.month as i32 - 1 + delta).rem_euclid(12) + 1) as u32;
        }
        Field::Year => {
            setup.year = (setup.year + delta).clamp(*YEARS.start(), *YEARS.end());
        }
        Field::Hour => setup.hour = (setup.hour as i32 + delta).rem_euclid(24) as u32,
        Field::Minute => {
            let step = if fine { 1 } else { 5 };
            let m = setup.minute as i32 + delta * step;
            setup.minute = if fine {
                m.rem_euclid(60)
            } else {
                (m.div_euclid(5) * 5).rem_euclid(60)
            } as u32;
        }
    }
    clamp_date(setup);
}

/// Brings the day into the month (31 February becomes the last day of February).
pub fn clamp_date(setup: &mut NewGameSetup) {
    setup.year = setup.year.clamp(*YEARS.start(), *YEARS.end());
    setup.month = setup.month.clamp(1, 12);
    setup.day = setup.day.clamp(1, days_in_month(setup.year, setup.month));
    setup.hour = setup.hour.min(23);
    setup.minute = setup.minute.min(59);
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
mod tests {
    use super::*;
    use crate::new_game_screen::types::{MapItem, VehicleItem};

    fn catalogue() -> GameCatalogue {
        GameCatalogue {
            maps: vec![MapItem {
                id: "m".into(),
                name: "m".into(),
                friendly_name: "Berlin".into(),
                description: String::new(),
                tile_count: 4,
                standard_depot: String::new(),
                years: "1986 - 1994".into(),
                entry_points: vec!["A".into(), "B".into()],
            }],
            vehicles: vec![VehicleItem::named("v", "MAN", "SD77")],
        }
    }

    fn setup() -> NewGameSetup {
        NewGameSetup {
            selected_map: "Berlin".into(),
            selected_entry_point: "A".into(),
            selected_vehicle: "MAN SD77".into(),
            ..NewGameSetup::default()
        }
    }

    #[test]
    fn steps_walk_forwards_and_backwards() {
        assert_eq!(PlannerStep::Where.prev(), None);
        assert_eq!(PlannerStep::Where.next(), Some(PlannerStep::What));
        assert_eq!(PlannerStep::Review.next(), None);
        assert_eq!(PlannerStep::Review.prev(), Some(PlannerStep::When));
    }

    #[test]
    fn validity_names_the_first_problem_in_step_order() {
        let c = catalogue();
        assert_eq!(validate(&setup(), &c), Ok(()));
        let mut s = setup();
        s.selected_map = "Nowhere".into();
        s.selected_vehicle = "Nothing".into();
        assert_eq!(validate(&s, &c), Err(Invalid::NoMap));
        let mut s = setup();
        s.selected_entry_point = "Z".into();
        assert_eq!(validate(&s, &c), Err(Invalid::NoStartPoint));
        let mut s = setup();
        s.selected_vehicle = "x".into();
        assert_eq!(validate(&s, &c), Err(Invalid::NoVehicle));
    }

    #[test]
    fn an_invalid_selection_is_repaired_from_the_catalogue() {
        let c = catalogue();
        let mut s = NewGameSetup::default();
        ensure_valid(&mut s, &c);
        assert_eq!(validate(&s, &c), Ok(()));
        assert_eq!(s.selected_map, "Berlin");
        assert_eq!(s.selected_entry_point, "A");
        let mut keep = setup();
        keep.selected_entry_point = "B".into();
        ensure_valid(&mut keep, &c);
        assert_eq!(
            keep.selected_entry_point, "B",
            "a valid choice is not touched"
        );
    }

    #[test]
    fn a_map_without_start_points_needs_none() {
        let mut c = catalogue();
        c.maps[0].entry_points.clear();
        let mut s = setup();
        s.selected_entry_point = String::new();
        assert_eq!(validate(&s, &c), Ok(()));
    }

    #[test]
    fn now_is_rounded_to_five_minutes_and_wraps() {
        assert_eq!(round_to_five(8, 32), (8, 30));
        assert_eq!(round_to_five(8, 33), (8, 35));
        assert_eq!(round_to_five(23, 58), (0, 0));
        let now = &QUICK_TIMES[0];
        assert_eq!(quick_time_of(now, (14, 17)), (14, 15));
        assert_eq!(quick_time_of(&QUICK_TIMES[3], (1, 1)), (18, 0));
    }

    #[test]
    fn exactly_one_chip_matches_a_fixed_time() {
        let mut s = setup();
        s.hour = 12;
        s.minute = 0;
        assert_eq!(matching_quick_time(&s), Some(2));
        s.minute = 5;
        assert_eq!(matching_quick_time(&s), None);
    }

    #[test]
    fn the_date_stays_valid_when_a_month_changes() {
        let mut s = setup();
        s.year = 1989;
        s.month = 1;
        s.day = 31;
        step_field(&mut s, Field::Month, 1, false);
        assert_eq!((s.month, s.day), (2, 28));
        s.year = 1988;
        s.month = 1;
        s.day = 31;
        step_field(&mut s, Field::Month, 1, false);
        assert_eq!((s.month, s.day), (2, 29));
        s.month = 12;
        step_field(&mut s, Field::Month, 1, false);
        assert_eq!(s.month, 1);
    }

    #[test]
    fn days_hours_and_minutes_wrap_and_years_clamp() {
        let mut s = setup();
        s.month = 4;
        s.day = 30;
        step_field(&mut s, Field::Day, 1, false);
        assert_eq!(s.day, 1);
        s.hour = 23;
        step_field(&mut s, Field::Hour, 1, false);
        assert_eq!(s.hour, 0);
        s.minute = 55;
        step_field(&mut s, Field::Minute, 1, false);
        assert_eq!(s.minute, 0);
        s.minute = 3;
        step_field(&mut s, Field::Minute, 1, true);
        assert_eq!(s.minute, 4);
        s.minute = 0;
        step_field(&mut s, Field::Minute, -1, true);
        assert_eq!(s.minute, 59);
        s.year = 2100;
        step_field(&mut s, Field::Year, 5, false);
        assert_eq!(s.year, 2100);
        s.year = 1900;
        step_field(&mut s, Field::Year, -5, false);
        assert_eq!(s.year, 1900);
    }

    #[test]
    fn the_era_hint_appears_only_outside_the_years_of_the_map() {
        assert_eq!(era_hint("1986 - 1994", 2020), Some((1986, 1994)));
        assert_eq!(era_hint("1986 - 1994", 1990), None);
        assert_eq!(era_hint("", 2020), None);
        assert_eq!(era_hint("ab", 2020), None);
    }
}
