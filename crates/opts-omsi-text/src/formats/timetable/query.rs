//! Queries on [`Timetables`]: stops, trips, tours, station links and the schedule of a run.

use super::{
    station_times, ActiveSchedule, Interpolation, ScheduledStop, StnLink, Timetables, TtBusStop,
    TtTour, TtTrip,
};

impl Timetables {
    /// The link from station `from` to station `to`, or failing that the one the other way
    /// round (a road between two stops is the same road).
    #[must_use]
    pub fn link(&self, from: u32, to: u32) -> Option<&StnLink> {
        self.link_by_pair
            .get(&(from, to))
            .or_else(|| self.link_by_pair.get(&(to, from)))
            .and_then(|&i| self.links.get(i))
    }

    /// Whether the folder held nothing usable.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The stop with station id `id`.
    #[must_use]
    pub fn stop(&self, id: u32) -> Option<&TtBusStop> {
        self.stop_by_id.get(&id).and_then(|&i| self.stops.get(i))
    }

    /// A trip file by name (case-insensitive).
    #[must_use]
    pub fn trip(&self, name: &str) -> Option<&TtTrip> {
        self.trips.get(&name.to_ascii_lowercase())
    }

    /// The tour `tour` of line file `line_file` (case-insensitive stem).
    #[must_use]
    pub fn tour(&self, line_file: &str, tour: usize) -> Option<&TtTour> {
        self.lines
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(line_file))
            .and_then(|(_, l)| l.tours.get(tour))
    }

    /// The schedule of run `run` of tour `tour` in `line_file`: the trip's stations with the
    /// times its timing profile gives, counted from the run's start.
    #[must_use]
    pub fn schedule(&self, line_file: &str, tour: usize, run: usize) -> Option<ActiveSchedule> {
        self.schedule_with(line_file, tour, run, Interpolation::default())
    }

    /// The lengths of the links between the stations of a trip, `n - 1` of them, when every
    /// consecutive pair has a link (`StnLinks.cfg`); `None` otherwise.
    #[must_use]
    pub fn segment_lengths(&self, trip: &TtTrip) -> Option<Vec<f32>> {
        let segs: Vec<f32> = trip
            .stations
            .windows(2)
            .map(|w| self.link(w[0].id, w[1].id).map_or(0.0, |l| l.length_m))
            .collect();
        (!segs.is_empty() && segs.iter().all(|&d| d > 0.0)).then_some(segs)
    }

    /// [`Timetables::schedule`] with the interpolation chosen (the experiment of the corpus test
    /// compares the two).
    #[must_use]
    pub fn schedule_with(
        &self,
        line_file: &str,
        tour: usize,
        run: usize,
        mode: Interpolation,
    ) -> Option<ActiveSchedule> {
        let (stem, _) = self
            .lines
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(line_file))?;
        let t = self.tour(line_file, tour)?;
        let r = t.trips.get(run)?;
        let trip = self.trip(&r.trip)?;
        let start_s = r.start_min * 60.0;
        let profile = trip
            .profiles
            .get(r.profile)
            .or_else(|| trip.profiles.first());
        let segments = match mode {
            Interpolation::ByIndex => None,
            Interpolation::ByDistance => self.segment_lengths(trip),
        };
        // The distance along the run, whenever the links are complete (whatever the mode).
        let cumulated: Option<Vec<f32>> = self.segment_lengths(trip).map(|segs| {
            std::iter::once(0.0)
                .chain(segs.iter().scan(0.0f32, |acc, d| {
                    *acc += d;
                    Some(*acc)
                }))
                .collect()
        });
        let times = profile.map_or_else(
            || vec![(0.0, 0.0); trip.stations.len()],
            |p| station_times(p, trip.stations.len(), segments.as_deref()),
        );
        let stops = trip
            .stations
            .iter()
            .zip(times)
            .enumerate()
            .map(|(k, (st, (a, d)))| ScheduledStop {
                station_id: st.id,
                name: self
                    .stop(st.id)
                    .map_or_else(|| st.name.clone(), |s| s.name.clone()),
                arrival_s: start_s + a * 60.0,
                departure_s: start_s + d * 60.0,
                distance_m: cumulated.as_ref().and_then(|c| c.get(k).copied()),
            })
            .collect();
        Some(ActiveSchedule {
            line_file: stem.clone(),
            tour: t.name.clone(),
            trip: r.trip.clone(),
            title: trip.title.clone(),
            line: trip.line.clone(),
            start_s,
            stops,
        })
    }
}
