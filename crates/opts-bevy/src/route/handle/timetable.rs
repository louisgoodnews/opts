//! The timetable side of the selection handle: the followed run, its route, and the delay against the clock.

use super::*;

impl RouteHandle {

    /// The map's timetable files.
    pub fn set_timetables(&self, t: Option<Arc<Timetables>>) {
        self.with_mut(|s| s.timetables = t);
    }

    /// The map's timetable files.
    #[must_use]
    pub fn timetables(&self) -> Option<Arc<Timetables>> {
        self.with(|s| s.timetables.clone())
    }

    /// Follows run `run` of tour `tour` in line file `line_file`: the schedule becomes active,
    /// the stop is the first, and the depot's route of that line and destination is selected when
    /// it has one (so the IBIS can be keyed). Returns the schedule and the route code selected.
    ///
    /// # Errors
    /// [`RouteError::UnknownRun`] when the map has no such run or no timetable files.
    pub fn select_run(
        &self,
        line_file: &str,
        tour: usize,
        run: usize,
    ) -> Result<(ActiveSchedule, Option<i32>), RouteError> {
        let tt = self.timetables().ok_or(RouteError::NoTimetables)?;
        let schedule = tt
            .schedule(line_file, tour, run)
            .ok_or_else(|| RouteError::UnknownRun(format!("{line_file}/{tour}/{run}")))?;
        self.with_mut(|s| {
            s.schedule = Some(schedule.clone());
            s.stop_index = 0;
        });
        let code = self.route_code_for(&schedule);
        if let Some(c) = code {
            let _ = self.select(c);
            // `select` resets the stop; the schedule stays.
        }
        Ok((schedule, code))
    }

    /// Stops following a timetable.
    pub fn clear_schedule(&self) {
        self.with_mut(|s| {
            s.schedule = None;
            s.delay_s = 0.0;
        });
    }

    /// The run followed, if any.
    #[must_use]
    pub fn schedule(&self) -> Option<ActiveSchedule> {
        self.with(|s| s.schedule.clone())
    }

    /// The depot route (route code) that has the schedule's line and destination: the terminus
    /// the trip's title names, on a route whose line text is the trip's line (or whose line
    /// number is it).
    #[must_use]
    pub fn route_code_for(&self, schedule: &ActiveSchedule) -> Option<i32> {
        self.with(|s| {
            let hof = s.hof.as_ref()?;
            let want = flat(&schedule.title);
            let terminus = hof
                .termini
                .iter()
                .find(|t| {
                    !want.is_empty()
                        && (flat(&t.ident) == want || t.strings.iter().any(|x| flat(x) == want))
                })
                .map(|t| t.code)?;
            let line = flat(&schedule.line);
            hof.routes
                .iter()
                .find(|r| {
                    r.terminus_code == terminus
                        && (flat(&r.line_string) == line
                            || (r.route_code / 100).to_string() == line)
                })
                .map(|r| r.route_code)
        })
    }

    /// Sets the clock the delay is measured against (s after midnight) and recomputes the delay
    /// of a followed timetable: now minus the scheduled arrival at the current stop, wrapped to
    /// within 12 hours (0 in free driving).
    pub fn set_clock(&self, seconds_of_day: f32) {
        self.with_mut(|s| {
            let stop = s.stop_index;
            s.delay_s = match &s.schedule {
                Some(sc) => sc.stops.get(stop).map_or(0.0, |x| {
                    // Times wrap at 24 h: a run that starts at 23:50 and a clock at 00:10 are
                    // 20 minutes apart, not 23 h 40 min.
                    let d = seconds_of_day - x.arrival_s;
                    (d + 43_200.0).rem_euclid(86_400.0) - 43_200.0
                }),
                None => 0.0,
            };
        });
    }

    /// Moves to stop `index` of the route.
    pub fn set_stop(&self, index: usize) {
        self.with_mut(|s| s.stop_index = index);
    }

    /// Sets the delay against the timetable (s).
    pub fn set_delay(&self, delay_s: f32) {
        self.with_mut(|s| s.delay_s = delay_s);
    }
}
