//! The counts of one run, and the records of changes under observation.

use opts_traffic::AgentId;

/// One change under observation: the changer, the follower it left behind in the target lane,
/// and the worst braking of both within the window.
pub struct Watch {
    pub changer: AgentId,
    pub follower: Option<AgentId>,
    pub t0: f64,
    pub follower_decel: f64,
    pub changer_decel: f64,
    pub recovered_at: Option<f64>,
    pub desired: f64,
    pub speed_at_t0: f64,
}

/// Per-agent announcement state: when the indicator came on, and whether the move already
/// happened while it stays on (the glide keeps it on; that is not a new announcement).
#[derive(Default, Clone, Copy)]
pub struct Announce {
    pub since: Option<f64>,
    pub moved: bool,
}

#[derive(Default)]
pub struct Tally {
    pub situations_with_change: u64,
    pub changes: u64,
    pub indicator_checked: u64,
    pub indicator_short: u64,
    pub shortest_indicator: Option<f64>,
    pub follower_checked: u64,
    pub follower_within: u64,
    pub worst_follower_decel: f64,
    pub unsafe_changes: u64,
    pub recovery_checked: u64,
    pub recovery_late: u64,
    pub late_example: Option<String>,
}

impl Tally {
    /// Records one announced move: `shown` is the indicator time before it, `None` when the
    /// move came without an announcement.
    pub fn record_indicator(&mut self, shown: Option<f64>, min_s: f64) {
        self.indicator_checked += 1;
        match shown {
            Some(t) => {
                self.shortest_indicator = Some(self.shortest_indicator.map_or(t, |m| m.min(t)));
                if t < min_s {
                    self.indicator_short += 1;
                }
            }
            None => self.indicator_short += 1,
        }
    }

    /// Adds one finished watch, judged against the follower and unsafe limits.
    pub fn record_watch(
        &mut self,
        watch: &Watch,
        seed: u64,
        follower_limit: f64,
        unsafe_limit: f64,
    ) {
        self.recovery_checked += 1;
        if watch.follower.is_some() {
            self.follower_checked += 1;
            self.worst_follower_decel = self.worst_follower_decel.max(watch.follower_decel);
            if watch.follower_decel <= follower_limit {
                self.follower_within += 1;
            }
        }
        if watch.follower_decel.max(watch.changer_decel) > unsafe_limit {
            self.unsafe_changes += 1;
        }
        if watch.recovered_at.is_none() {
            self.recovery_late += 1;
            if self.late_example.is_none() {
                self.late_example = Some(format!(
                    "seed {seed}, change at t={:.1} s, changer speed {:.2} m/s, desired {:.1} m/s",
                    watch.t0, watch.speed_at_t0, watch.desired
                ));
            }
        }
    }
}
