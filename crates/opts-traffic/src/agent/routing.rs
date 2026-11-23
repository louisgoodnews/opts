//! Strategic routing: the lanes an agent may take through a change, and its plan.

use super::*;

impl TrafficWorld {
    /// The lanes a vehicle of `class` may continue into from `lane` through a lane change to a
    /// neighbour: successors of a neighbour that runs beside the end of `lane`, which are
    /// neither successors of `lane` nor parallel to one (so the change opens a new direction —
    /// a turn pocket — or avoids a dead end). Each with the neighbour and its side.
    fn strategic_options(
        &self,
        lane: LaneId,
        class: opts_car_following::VehicleClass,
        group: usize,
    ) -> Vec<(LaneId, LaneId, Side)> {
        let Some(own) = self.graph.lane(lane) else {
            return Vec::new();
        };
        let len = own.length();
        if len < STRATEGIC_MIN_LANE_M || own.junction {
            return Vec::new();
        }
        let succ = self.graph.successors(lane);
        let parallel: Vec<LaneId> = succ
            .iter()
            .flat_map(|&s| self.graph.neighbours(s).iter().map(|n| n.lane))
            .collect();
        let mut out = Vec::new();
        for n in self.graph.neighbours(lane) {
            let Some(nl) = self.graph.lane(n.lane) else {
                continue;
            };
            if nl.kind != own.kind
                || nl.rules.density.closed_for(group)
                || !nl.rules.allows(class)
                || n.at(len - STRATEGIC_LAST_M - 5.0).is_none()
            {
                continue;
            }
            for &t in self.graph.successors(n.lane) {
                let ok = !succ.contains(&t)
                    && !parallel.contains(&t)
                    && !out.iter().any(|(x, _, _)| *x == t)
                    && self.graph.lane(t).is_some_and(|l| {
                        l.rules.allows(class) && !l.rules.density.closed_for(group)
                    });
                if ok {
                    out.push((t, n.lane, n.side));
                }
            }
        }
        out
    }

    /// A successor of `lane` for a vehicle of `class`, chosen at random among the lanes it may
    /// use (all of them when none qualifies, so a vehicle is never stranded by a rule) and, when
    /// `strategic`, the lanes reachable through a change to a neighbour
    /// ([`Self::strategic_options`]; always one of them when `lane` is a dead end).
    fn pick_next(
        &mut self,
        lane: LaneId,
        class: opts_car_following::VehicleClass,
        group: usize,
        strategic: bool,
    ) -> Option<(LaneId, Option<Strategic>)> {
        let succ = self.graph.successors(lane);
        // The lanes the vehicle may use: open to its class and not closed to its density group;
        // failing that open to its class; failing that all (never stranded by a rule).
        let open = |s: &LaneId, closed: bool| {
            self.graph.lane(*s).is_none_or(|l| {
                l.rules.allows(class) && !(closed && l.rules.density.closed_for(group))
            })
        };
        let mut pool: Vec<LaneId> = succ.iter().copied().filter(|s| open(s, true)).collect();
        if pool.is_empty() {
            pool = succ.iter().copied().filter(|s| open(s, false)).collect();
        }
        if pool.is_empty() {
            pool = succ.to_vec();
        }
        let options = if strategic {
            self.strategic_options(lane, class, group)
        } else {
            Vec::new()
        };
        let total = pool.len() + options.len();
        if total == 0 {
            return None;
        }
        let k = self.rng.below(total);
        Some(match pool.get(k) {
            Some(&l) => (l, None),
            None => {
                let (t, via, side) = options[k - pool.len()];
                (
                    t,
                    Some(Strategic {
                        from: lane,
                        via,
                        side,
                    }),
                )
            }
        })
    }

    /// Extends agent `i`'s plan until it covers [`PLAN_HORIZON_M`] of road (or runs out).
    pub(crate) fn extend_plan(&mut self, i: usize) {
        if self.agents[i].foreign {
            return;
        }
        loop {
            let a = &self.agents[i];
            let Some(lane) = self.graph.lane(a.lane) else {
                return;
            };
            let mut covered = lane.length() - a.s;
            for p in &a.plan {
                covered += self.graph.lane(*p).map_or(0.0, |l| l.length());
            }
            if covered >= PLAN_HORIZON_M || a.plan.len() >= 16 {
                return;
            }
            let last = a.plan.back().copied().unwrap_or(a.lane);
            let strategic = a.strategic.is_none() && a.no_strategic != Some(last);
            let (class, group) = (a.params.class, usize::from(a.params.density_group));
            // A scheduled vehicle takes its fixed route while that follows the lanes.
            if let Some(n) = self.next_route_lane(i, last) {
                self.agents[i].plan.push_back(n);
                continue;
            }
            match self.pick_next(last, class, group, strategic) {
                Some((n, change)) => {
                    let a = &mut self.agents[i];
                    a.plan.push_back(n);
                    if change.is_some() {
                        a.strategic = change;
                        a.strategic_wait_s = 0.0;
                    }
                }
                None => return,
            }
        }
    }
}
