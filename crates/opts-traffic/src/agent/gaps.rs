//! Gap and conflict queries: leader, signals, conflict zones.

use super::*;

impl TrafficWorld {
    /// The lanes ahead of agent `i` with the distance from its front bumper to each lane's start
    /// (negative for the current lane), up to [`LOOKAHEAD_M`].
    pub(crate) fn lookahead(&self, i: usize) -> Vec<(LaneId, f64)> {
        let a = &self.agents[i];
        let mut out = vec![(a.lane, -a.s)];
        let Some(lane) = self.graph.lane(a.lane) else {
            return out;
        };
        let mut offset = lane.length() - a.s;
        for p in &a.plan {
            if offset > LOOKAHEAD_M {
                break;
            }
            out.push((*p, offset));
            offset += self.graph.lane(*p).map_or(0.0, |l| l.length());
        }
        out
    }

    /// The nearest vehicle ahead of agent `i` along its lookahead: `(net gap, its speed)`.
    pub(crate) fn leader(
        &self,
        i: usize,
        look: &[(LaneId, f64)],
        by_lane: &HashMap<LaneId, Vec<usize>>,
    ) -> Option<(f64, f64)> {
        let a = &self.agents[i];
        for &(lane, off) in look {
            let Some(list) = by_lane.get(&lane) else {
                continue;
            };
            let o = if lane == a.lane {
                let pos = list.iter().position(|&j| j == i)?;
                list.get(pos + 1)
            } else {
                list.first()
            };
            if let Some(&j) = o {
                let o = &self.agents[j];
                return Some((off + o.s - o.params.length, o.speed));
            }
        }
        None
    }

    /// The gap (m) to the start of the lane whose light agent `i` has to stop for, if any:
    /// red and red + yellow stop it, yellow only when it can stop at a comfortable deceleration.
    pub(super) fn signal_gap(&self, i: usize, look: &[(LaneId, f64)]) -> Option<f64> {
        let a = &self.agents[i];
        for &(lane_id, off) in look {
            if off < 0.5 {
                continue;
            }
            let Some(key) = self.graph.lane(lane_id).and_then(|l| l.signal) else {
                continue;
            };
            let Some(program) = self.signals.get(&key) else {
                continue;
            };
            let reach = if program.approach > 0.0 {
                program.approach
            } else {
                DEFAULT_APPROACH_M
            };
            if off > reach.max(DEFAULT_APPROACH_M) {
                continue;
            }
            let stop = match program.aspect_at(self.clock()) {
                Aspect::Stop => true,
                Aspect::Yellow => off > a.speed * a.speed / (2.0 * YELLOW_COMFORT_DECEL),
                Aspect::Go | Aspect::Off => false,
            };
            if stop {
                return Some((off - SIGNAL_STOP_MARGIN_M).max(0.0));
            }
        }
        None
    }

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

    /// Agents per lane, rearmost first; ties broken by id so the order is deterministic.
    pub(crate) fn agents_by_lane(&self) -> HashMap<LaneId, Vec<usize>> {
        let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
        for (i, a) in self.agents.iter().enumerate() {
            by_lane.entry(a.lane).or_default().push(i);
        }
        for list in by_lane.values_mut() {
            list.sort_by(|&x, &y| {
                let (a, b) = (&self.agents[x], &self.agents[y]);
                a.s.total_cmp(&b.s).then(a.id.cmp(&b.id))
            });
        }
        by_lane
    }
}
