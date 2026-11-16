//! The train and tram side of the scheduled traffic's counters: runs driven on the rails, their
//! arrivals and departures, as the rail scheduler reports them.

use super::*;

impl ScheduledTraffic {
    /// Whether a train or tram drives `run` now.
    #[must_use]
    pub fn rail_has(&self, run: &RunRef) -> bool {
        self.rail_runs.contains(run)
    }

    /// Whether a road vehicle drives `run` now.
    #[must_use]
    pub fn road_has(&self, run: &RunRef) -> bool {
        self.runs.get(run).is_some_and(|r| r.agent.is_some())
    }

    /// Trains and trams on the road.
    #[must_use]
    pub fn rail_agents(&self) -> usize {
        self.rail_runs.len()
    }

    /// A train or tram was built for `run`.
    pub fn rail_started(&mut self, run: RunRef) {
        self.stats.spawned += 1;
        self.rail_runs.insert(run);
    }

    /// The train or tram of `run` is gone.
    pub fn rail_done(&mut self, run: &RunRef) {
        self.rail_runs.remove(run);
    }

    /// A train or tram stands at a stop of its run at `clock`; `scheduled` is the timetable's
    /// arrival there.
    pub fn rail_arrived(&mut self, station: u32, scheduled: f64, clock: f64) {
        self.stats.arrivals += 1;
        if (clock - scheduled).abs() >= 3600.0 {
            return;
        }
        self.stats.delay_sum_s += clock - scheduled;
        self.stats.log.push((station, scheduled, clock));
        if self.stats.log.len() > 40 {
            self.stats.log.remove(0);
        }
    }

    /// A train or tram left a stop.
    pub fn rail_departed(&mut self) {
        self.stats.departures += 1;
    }

    /// A train or tram dropped a stop it could not reach.
    pub fn rail_missed(&mut self) {
        self.stats.missed += 1;
    }
}
