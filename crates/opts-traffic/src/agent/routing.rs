//! Strategic routing and the path of an agent.

use super::*;

impl TrafficWorld {
    /// The lanes a vehicle of `class` may continue into from `lane` through a lane change to a
    /// neighbour: successors of a neighbour that runs beside the end of `lane`, which are
    /// neither successors of `lane` nor parallel to one (so the change opens a new direction —
    /// a turn pocket — or avoids a dead end). Each with the neighbour and its side.
    fn strategic_options(
        &self,
        lane: LaneId,
        class: crate::idm::VehicleClass,
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
        class: crate::idm::VehicleClass,
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

    /// The point `back` metres behind agent `i`'s front bumper along its path, with the path's
    /// heading there and the sideways offset of a lane change: on the current lane, else on the
    /// previous one, else on the extension of the lane's start tangent.
    fn path_point(&self, i: usize, back: f64) -> Option<(DVec2, f64)> {
        let a = &self.agents[i];
        let g = &self.graph.lane(a.lane)?.geometry;
        let s = a.s - back;
        let (p, h) = if s >= 0.0 {
            (g.point_at(s), g.heading_at(s))
        } else if let Some(prev) = a
            .prev_lane
            .and_then(|l| self.graph.lane(l))
            .filter(|l| l.length() + s >= 0.0)
        {
            let t = prev.length() + s;
            (prev.geometry.point_at(t), prev.geometry.heading_at(t))
        } else {
            let h = g.heading;
            (g.start + DVec2::new(h.sin(), h.cos()) * s, h)
        };
        Some((p + DVec2::new(h.cos(), -h.sin()) * a.lateral, h))
    }

    /// Moves agent `i`'s axles: the front axle onto its path, the rear axle after it at the
    /// wheelbase. `travel` (m) is how far the front axle may have moved since the last update;
    /// a larger jump (a rebuilt graph, a teleport) places the rear axle afresh on the path's
    /// tangent, as on spawn.
    pub(crate) fn update_axles(&mut self, i: usize, travel: f64) {
        let p = self.agents[i].params;
        let Some((on_path, h)) = self.path_point(i, p.front_overhang) else {
            return;
        };
        let Some((target, _)) = self.path_point(i, p.front_overhang - FRONT_PURSUIT_M) else {
            return;
        };
        let a = &mut self.agents[i];
        let l = p.wheelbase.max(0.1);
        let jumped = !a.axles_ready
            || on_path.distance(a.front_axle) > travel + AXLE_RESET_M
            || (a.front_axle.distance(a.rear_axle) - l).abs() > AXLE_RESET_M;
        if jumped {
            a.front_axle = on_path;
            a.rear_axle = on_path - DVec2::new(h.sin(), h.cos()) * l;
        } else {
            // The front axle pursues a point just ahead on the path (on an arc it stays within
            // d²/2R of the lane, a kink at a lane join is rounded off), the rear axle the front
            // one; both in sub-steps of at most `AXLE_SUBSTEP_M` of travel.
            let n = (a.pursuit.distance(target) / AXLE_SUBSTEP_M)
                .ceil()
                .clamp(1.0, 64.0);
            let from = a.pursuit;
            for k in 1..=n as usize {
                let t = from.lerp(target, k as f64 / n);
                let d = t - a.front_axle;
                if d.length_squared() > 1e-12 {
                    a.front_axle = t - d.normalize() * FRONT_PURSUIT_M;
                }
                let d = a.front_axle - a.rear_axle;
                if d.length_squared() > 1e-12 {
                    a.rear_axle = a.front_axle - d.normalize() * l;
                }
            }
        }
        a.pursuit = target;
        a.axles_ready = true;
        let steer = h - a.heading();
        a.steer =
            (steer + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
    }
}
