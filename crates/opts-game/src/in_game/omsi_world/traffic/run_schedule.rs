//! Timetable logic for scheduled vehicles: which runs are on the road at a clock time and where a
//! run is along its stops.
//!
//! Pure functions over [`opts_core::formats::timetable`] (they live here, not in the engine crate
//! `opts-traffic`, which keeps off the OMSI file types, ADR-0014); the planner turns them into
//! vehicles (`docs/src/architecture/scheduled-traffic.md`). Times are seconds of the day; a run past midnight has
//! times above 86,400, and the clock is looked at both as itself and a day later so that such a
//! run is found after midnight too.

use opts_core::formats::timetable::{ActiveSchedule, Timetables};

/// Seconds of a day.
pub const DAY_S: f64 = 86_400.0;

/// A run of a tour: the key the timetable reader takes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunRef {
    /// Line file stem.
    pub line_file: String,
    /// Index of the tour in the line file.
    pub tour: usize,
    /// Index of the run (trip entry) in the tour.
    pub run: usize,
}

/// Where a run is at a clock time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunPhase {
    /// Before the first departure: `wait_s` until the run starts.
    NotStarted {
        /// Seconds until the first stop's departure.
        wait_s: f64,
    },
    /// Standing at stop `index` (between its arrival and its departure).
    AtStop {
        /// The stop.
        index: usize,
    },
    /// Between stop `from` and `from + 1`, `fraction` of the way by time.
    Between {
        /// The stop left.
        from: usize,
        /// `0..1`.
        fraction: f64,
    },
    /// Past the last arrival.
    Finished,
}

/// The phase of `schedule` at clock time `now_s`.
#[must_use]
pub fn run_phase(schedule: &ActiveSchedule, now_s: f64) -> RunPhase {
    let stops = &schedule.stops;
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return RunPhase::Finished;
    };
    if now_s < f64::from(first.departure_s) {
        return RunPhase::NotStarted {
            wait_s: f64::from(first.departure_s) - now_s,
        };
    }
    if now_s >= f64::from(last.arrival_s) {
        return RunPhase::Finished;
    }
    for (i, w) in stops.windows(2).enumerate() {
        let (a, b) = (&w[0], &w[1]);
        if now_s < f64::from(a.arrival_s) && i > 0 {
            continue;
        }
        if now_s < f64::from(a.departure_s) && now_s >= f64::from(a.arrival_s) {
            return RunPhase::AtStop { index: i };
        }
        if now_s >= f64::from(a.departure_s) && now_s < f64::from(b.arrival_s) {
            let span = f64::from(b.arrival_s - a.departure_s).max(1e-6);
            return RunPhase::Between {
                from: i,
                fraction: ((now_s - f64::from(a.departure_s)) / span).clamp(0.0, 1.0),
            };
        }
        if now_s >= f64::from(b.arrival_s)
            && now_s < f64::from(b.departure_s)
            && i + 2 == stops.len()
        {
            return RunPhase::AtStop { index: i + 1 };
        }
    }
    RunPhase::Finished
}

/// The runs that are on the road at `now_s`, or start within `lead_s`: those whose start lies in
/// `[now - horizon_s, now + lead_s]` (a run never lasts longer than `horizon_s`) and whose
/// schedule has not finished. Each with its schedule. Day offsets: the clock is tried as
/// itself and as `now + 86,400` for runs that cross midnight.
#[must_use]
pub fn active_runs(
    tt: &Timetables,
    now_s: f64,
    lead_s: f64,
    horizon_s: f64,
) -> Vec<(RunRef, ActiveSchedule)> {
    let mut out = Vec::new();
    for (stem, line) in &tt.lines {
        for (t, tour) in line.tours.iter().enumerate() {
            for (r, run) in tour.trips.iter().enumerate() {
                let start = f64::from(run.start_min) * 60.0;
                let clock = [now_s, now_s + DAY_S]
                    .into_iter()
                    .find(|c| start >= c - horizon_s && start <= c + lead_s);
                let Some(clock) = clock else { continue };
                let Some(schedule) = tt.schedule(stem, t, r) else {
                    continue;
                };
                if run_phase(&schedule, clock) == RunPhase::Finished {
                    continue;
                }
                out.push((
                    RunRef {
                        line_file: stem.clone(),
                        tour: t,
                        run: r,
                    },
                    schedule,
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
