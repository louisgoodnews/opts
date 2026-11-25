//! Challenge definitions: a stable id, the number of stops the run must serve, its rules and its
//! pass criteria.

use crate::rules::{ScoringRules, MAX_COLLISIONS, MAX_MISSED_STOPS, MIN_ON_TIME_RATIO, TIGHT_WINDOW_S};

/// What a run must achieve to pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassCriteria {
    /// Share of the expected stops that must be served on time (0.0 to 1.0).
    pub min_on_time_ratio: f64,
    /// Most collisions allowed.
    pub max_collisions: u32,
    /// Most missed stops allowed.
    pub max_missed: u32,
}

impl Default for PassCriteria {
    fn default() -> Self {
        Self {
            min_on_time_ratio: MIN_ON_TIME_RATIO,
            max_collisions: MAX_COLLISIONS,
            max_missed: MAX_MISSED_STOPS,
        }
    }
}

/// One challenge: what the run is scored with and what it is judged against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Challenge {
    /// Stable id, used in logs and in stored results (not a display name).
    pub id: &'static str,
    /// Stops the timetable schedules for the run; the on-time ratio divides by this.
    pub expected_stops: u32,
    /// Points table.
    pub rules: ScoringRules,
    /// Pass criteria.
    pub criteria: PassCriteria,
}

/// The base challenge: punctuality against the timetable, with the default window.
pub fn punctual_run(expected_stops: u32) -> Challenge {
    Challenge {
        id: "punctual-run",
        expected_stops,
        rules: ScoringRules::default(),
        criteria: PassCriteria::default(),
    }
}

/// The tight-schedule challenge: the same rules with a narrower on-time window.
pub fn tight_schedule(expected_stops: u32) -> Challenge {
    let mut challenge = punctual_run(expected_stops);
    challenge.id = "tight-schedule";
    challenge.rules.punctual_window_s = TIGHT_WINDOW_S;
    challenge
}
