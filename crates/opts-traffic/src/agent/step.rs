//! The simulation step, lane changes and driver commands.

use super::*;

impl TrafficWorld {
    /// Advances every agent by `dt` seconds (use steps of at most 0.1 s).
    pub fn step(&mut self, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        for i in 0..self.agents.len() {
            self.extend_plan(i);
        }
        self.add_phantoms();
        let mut by_lane = self.agents_by_lane();
        // One lane change per step at most; the lists are rebuilt when it happened.
        if self.lane_change_pass(&by_lane) {
            for i in 0..self.agents.len() {
                self.extend_plan(i);
            }
            by_lane = self.agents_by_lane();
        }
        let looks: Vec<Vec<(LaneId, f64)>> =
            (0..self.agents.len()).map(|i| self.lookahead(i)).collect();
        // Lights first: vehicles standing at a red light are not "arriving" for conflict zones.
        let signal: Vec<Option<f64>> = (0..self.agents.len())
            .map(|i| self.signal_gap(i, &looks[i]))
            .collect();
        let held: Vec<bool> = self
            .agents
            .iter()
            .zip(&signal)
            .map(|(a, g)| g.is_some_and(|g| g < 12.0) && a.speed < 3.0)
            .collect();
        // Decide accelerations from the state at the start of the step.
        let mut accels = Vec::with_capacity(self.agents.len());
        let mut blocked = vec![false; self.agents.len()];
        for (i, a) in self.agents.iter().enumerate() {
            if a.foreign || self.graph.lane(a.lane).is_none() {
                accels.push(0.0);
                continue;
            }
            let leader = self.leader(i, &looks[i], &by_lane);
            let wait = self.conflict_gap(i, &looks[i], &by_lane, &held);
            blocked[i] = wait.is_some();
            // The end of the lane a strategic change has to happen on is a stop line until the
            // change is made.
            let stuck = a.strategic.and_then(|c| {
                let (_, off) = looks[i].iter().find(|(l, _)| *l == c.from)?;
                let len = self.graph.lane(c.from)?.length();
                Some((off + len - STRATEGIC_STOP_M).max(0.0))
            });
            // The nearest of leader, light, conflict zone and such a stop limits the vehicle.
            let mut ahead = leader;
            let stop = self.stop_gap(i, &looks[i]);
            for gap in [signal[i], wait, stuck, stop].into_iter().flatten() {
                if ahead.is_none_or(|(g, _)| gap < g) {
                    ahead = Some((gap, 0.0));
                }
            }
            let (cap, brake) = self.speed_cap(i);
            let acc = a.params.acceleration(a.speed, cap, ahead).min(brake);
            accels.push(acc.max(-a.params.max_decel));
        }
        // Integrate (ballistic; a vehicle that would reverse stops within the step).
        let travel: Vec<f64> = self
            .agents
            .iter()
            .zip(&accels)
            .map(|(a, &acc)| (a.speed.max(a.speed + acc * dt) + MAX_GLIDE_SPEED) * dt)
            .collect();
        let mut hard = 0;
        for (a, &acc) in self.agents.iter_mut().zip(&accels) {
            if a.foreign {
                continue;
            }
            let v1 = a.speed + acc * dt;
            if v1 < 0.0 {
                a.s += if acc < 0.0 {
                    -a.speed * a.speed / (2.0 * acc)
                } else {
                    0.0
                };
                a.speed = 0.0;
            } else {
                a.s += a.speed * dt + 0.5 * acc * dt * dt;
                a.speed = v1;
            }
            if acc < -super::HARD_BRAKE && a.accel >= -super::HARD_BRAKE {
                hard += 1;
            }
            a.accel = acc;
            a.glide(dt);
        }
        self.stats.hard_brakes += hard;
        self.time += dt;
        self.update_stops();
        // A vehicle that cannot make its strategic change re-plans from its lane after a while.
        for i in 0..self.agents.len() {
            let a = &mut self.agents[i];
            let Some(c) = a.strategic.filter(|c| c.from == a.lane) else {
                continue;
            };
            let near_end = self
                .graph
                .lane(a.lane)
                .is_some_and(|l| l.length() - a.s < STRATEGIC_STOP_M + 5.0);
            if near_end && a.speed < 0.2 {
                a.strategic_wait_s += dt;
            }
            if a.strategic_wait_s > STRATEGIC_TIMEOUT_S {
                a.strategic = None;
                a.strategic_wait_s = 0.0;
                a.no_strategic = Some(c.from);
                a.plan.clear();
                if a.lc_side == Some(c.side) && !a.changing_lane() {
                    a.lc_due = None;
                    a.lc_side = None;
                }
                self.extend_plan(i);
            }
        }
        // A vehicle that waited too long at a zone goes anyway (cyclic waits, ghost vehicles), but
        // **one at a time** per junction: in a cycle (right before left at a four-way) every
        // vehicle times out together, and releasing them all sends them through the middle at
        // once. The one that waited longest (ties: the lower id) is released; the others that
        // share a conflict with it keep waiting, see it inside the zone and yield, and get their
        // turn when its zone is clear.
        let mut due: Vec<usize> = Vec::new();
        for (i, (a, &b)) in self.agents.iter_mut().zip(&blocked).enumerate() {
            if b && a.speed < 0.2 {
                a.waiting_s += dt;
                if a.waiting_s > DEADLOCK_AFTER_S {
                    due.push(i);
                }
            } else {
                a.waiting_s = 0.0;
            }
        }
        due.sort_by(|&x, &y| {
            let (a, b) = (&self.agents[x], &self.agents[y]);
            b.waiting_s.total_cmp(&a.waiting_s).then(a.id.cmp(&b.id))
        });
        let mut released: Vec<usize> = (0..self.agents.len())
            .filter(|&k| {
                self.agents[k].ignore_conflicts_until > self.time && !self.agents[k].foreign
            })
            .collect();
        for i in due {
            if released
                .iter()
                .any(|&j| j != i && self.shares_conflict(i, j))
            {
                continue; // its turn comes when the one released ahead of it has gone through
            }
            let a = &mut self.agents[i];
            a.waiting_s = 0.0;
            a.ignore_conflicts_until = self.time + DEADLOCK_IGNORE_S;
            released.push(i);
        }
        // The "vehicle stuck" flag: standing with no light to wait for. The first agent to reach
        // the limit is removed (one per step), which usually frees the others.
        let limit = self.stuck_after_s;
        let mut stuck = None;
        for (a, g) in self.agents.iter_mut().zip(&signal) {
            // A scheduled vehicle waiting at its stop is not stuck.
            let dwelling = a.stops.front().is_some_and(|s| s.arrived_at.is_some());
            if !a.foreign && a.speed < 0.2 && g.is_none() && !dwelling {
                a.stuck_s += dt;
                if stuck.is_none() && a.stuck_s > limit {
                    stuck = Some((a.id, a.lane));
                }
            } else {
                a.stuck_s = 0.0;
            }
        }
        if let Some((id, lane)) = stuck {
            self.despawn(id);
            self.events.push(TrafficEvent::Stuck { id, lane });
            self.remove_phantoms();
            return;
        }
        let ids: Vec<AgentId> = self.agents.iter().map(|a| a.id).collect();
        self.advance_lanes();
        // Axles after the lane transitions (agents removed at dead ends drop out of `ids`).
        let mut k = 0;
        for (id, t) in ids.into_iter().zip(travel) {
            if self.agents.get(k).is_some_and(|a| a.id == id) {
                if !self.agents[k].foreign {
                    self.update_axles(k, t);
                }
                k += 1;
            }
        }
        self.remove_phantoms();
    }

    /// Moves agents that passed the end of their lane onto the next one.
    fn advance_lanes(&mut self) {
        let mut i = 0;
        while i < self.agents.len() {
            loop {
                let a = &self.agents[i];
                if a.foreign {
                    break;
                }
                let Some(len) = self.graph.lane(a.lane).map(|l| l.length()) else {
                    break;
                };
                if a.s <= len {
                    break;
                }
                // It may not leave a lane it still has to change away from.
                if a.strategic.is_some_and(|c| c.from == a.lane) {
                    let a = &mut self.agents[i];
                    a.s = len;
                    a.speed = 0.0;
                    break;
                }
                let (over, id, lane) = (a.s - len, a.id, a.lane);
                match self.agents[i].plan.pop_front() {
                    Some(n) => {
                        let a = &mut self.agents[i];
                        a.prev_lane = Some(a.lane);
                        a.lane = n;
                        a.s = over;
                        self.extend_plan(i);
                    }
                    None => {
                        self.events.push(TrafficEvent::ReachedDeadEnd { id, lane });
                        self.agents.remove(i);
                        i = i.wrapping_sub(1);
                        break;
                    }
                }
            }
            i = i.wrapping_add(1);
        }
    }

    /// Pedal and indicator values for `id` derived from its last acceleration: positive
    /// accelerations map to `throttle = a / A`, braking to `brake = −a / max_decel`. The
    /// indicator shows an announced or running lane change, else an upcoming turn.
    #[must_use]
    pub fn command(&self, id: AgentId) -> Option<DriverCommand> {
        let a = self.agent(id)?;
        let (throttle, brake) = if a.accel >= 0.0 {
            ((a.accel / a.params.max_accel).clamp(0.0, 1.0), 0.0)
        } else {
            (0.0, (-a.accel / a.params.max_decel).clamp(0.0, 1.0))
        };
        let lane = self.graph.lane(a.lane)?;
        let mut blinker = match a.lc_side {
            Some(Side::Left) => Blinker::Left,
            Some(Side::Right) => Blinker::Right,
            None => Blinker::Off,
        };
        // A crossing path that states its indicator (`[path]` parameter 12) keeps it on while
        // it is driven and announces it over the last metres before it; paths without one fall
        // back to the turn of the next lane.
        if blinker == Blinker::Off {
            if let Some(b) = lane.indicator.filter(|b| *b != Blinker::Off) {
                blinker = b;
            } else if let Some(next) = a.next().and_then(|n| self.graph.lane(n)) {
                if lane.length() - a.s < BLINK_DISTANCE {
                    blinker = next.indicator.unwrap_or_else(|| {
                        let turn = (next.geometry.end_heading() - lane.geometry.end_heading()
                            + std::f64::consts::PI)
                            .rem_euclid(std::f64::consts::TAU)
                            - std::f64::consts::PI;
                        if turn > BLINK_TURN {
                            Blinker::Right
                        } else if turn < -BLINK_TURN {
                            Blinker::Left
                        } else {
                            Blinker::Off
                        }
                    });
                }
            }
        }
        Some(DriverCommand {
            throttle: throttle as f32,
            brake: brake as f32,
            blinker,
        })
    }
}
