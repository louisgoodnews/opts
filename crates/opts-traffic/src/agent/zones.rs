//! Conflict-zone gaps: the zone an agent waits in front of, and which agents meet in a junction.

use super::*;

impl TrafficWorld {
    /// The gap (m) to the zone agent `i` has to wait in front of, if it must give way to another
    /// vehicle: crossing and merging flows take turns by estimated arrival time at the zone (a
    /// vehicle already in the zone always goes first; ties go to the lower id). Vehicles that
    /// stand at a red light do not count as arriving. Zones the agent has entered are never
    /// blocked, so nobody stops in the middle of a junction.
    pub(super) fn conflict_gap(
        &self,
        i: usize,
        look: &[(LaneId, f64)],
        by_lane: &HashMap<LaneId, Vec<usize>>,
        held: &[bool],
    ) -> Option<f64> {
        let a = &self.agents[i];
        // The deadlock breaker lets a vehicle drive on through *other traffic* it waits for
        // (cyclic waits, ghosts); a vehicle outside the traffic (the player's bus) is a body
        // that is really there and is never driven through.
        let ignoring = a.ignore_conflicts_until > self.time;
        let mut best: Option<f64> = None;
        for &(lane_id, offset) in look {
            if offset > CONFLICT_LOOKAHEAD_M {
                break;
            }
            for c in self.graph.conflicts(lane_id) {
                // Distance from my front bumper to the zone entrance.
                let d = c.enter + offset;
                if !(-0.5..=CONFLICT_LOOKAHEAD_M).contains(&d) {
                    continue; // inside the zone already, or too far to matter
                }
                let t_me = d.max(0.0) / a.speed.max(MIN_ARRIVAL_SPEED);
                let prec = precedence(&self.graph, lane_id, c);
                let must_wait = by_lane.get(&c.other).is_some_and(|list| {
                    list.iter().any(|&j| {
                        if j == i {
                            return false;
                        }
                        let b = &self.agents[j];
                        let inside = b.s > c.other_enter && b.s - b.params.length < c.other_exit;
                        // A released vehicle drives on through traffic it was waiting for, but
                        // not through one that is moving through the zone right now: that one
                        // clears it in a moment, and driving through it is a collision, not a
                        // way out of a cyclic wait.
                        if ignoring && !b.foreign && !(inside && b.speed > RELEASE_RESPECT_SPEED) {
                            return false;
                        }
                        let gap_b = c.other_enter - b.s;
                        if !inside && (held[j] || !(0.0..=CONFLICT_LOOKAHEAD_M).contains(&gap_b)) {
                            return false;
                        }
                        let t_b = if inside {
                            0.0
                        } else {
                            gap_b / b.speed.max(MIN_ARRIVAL_SPEED)
                        };
                        match prec {
                            _ if inside => true,
                            // The other has the right of way: give way if it is near enough.
                            Precedence::Second => t_b < GAP_ACCEPTANCE_S,
                            // I have it: the other yields (and a vehicle inside is respected above).
                            Precedence::First => false,
                            Precedence::Equal => t_b < t_me || (t_b == t_me && b.id < a.id),
                        }
                    })
                });
                if must_wait {
                    let gap = (d - CONFLICT_STOP_MARGIN_M).max(0.0);
                    best = Some(best.map_or(gap, |g: f64| g.min(gap)));
                }
            }
        }
        best
    }

    /// Whether agent `i`'s lane (or one of the next ones on its plan) has a conflict zone with the
    /// lane agent `j` is on: the two meet in a junction.
    pub(super) fn shares_conflict(&self, i: usize, j: usize) -> bool {
        let other = self.agents[j].lane;
        let a = &self.agents[i];
        std::iter::once(a.lane)
            .chain(a.plan.iter().copied().take(3))
            .any(|l| self.graph.conflicts(l).iter().any(|c| c.other == other))
    }
}
