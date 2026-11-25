//! The logic of *Plan a drive* without any UI: steps, validity, the quick times, the date and time
//! arithmetic of the fixed-size steppers and the one-line summary.
//!
//! The specification is `docs/src/roadmap/phase-9/wizard-and-loading.md`.

use bevy::prelude::*;

#[cfg(test)]
use super::types::{GameCatalogue, NewGameSetup};
#[cfg(test)]
use opts_bevy::CalendarDate;

use opts_new_game_types::date;
mod era;
mod quick;
mod validation;

pub use date::{clamp_date, days_in_month, set_setup_date, setup_date, step_field, today, Today};
pub use era::{default_map_date, default_start_clock, era_hint, MapEra};
pub use quick::{
    clock_text, matching_quick_time, quick_time_of, round_to_five, QuickTime, QUICK_TIMES,
};
pub use validation::{ensure_valid, validate, Invalid};

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

pub use crate::shell::calendar::{month_key, Field, YEARS};

#[cfg(test)]
mod tests;
