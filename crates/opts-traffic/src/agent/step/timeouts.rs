//! Timeouts of the step: strategic changes that wait too long, deadlock breaking and stuck vehicles.

use super::*;

impl TrafficWorld {
    pub(super) fn handle_strategic_timeouts(&mut self, dt: f64) {
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
    }

    pub(super) fn resolve_deadlocks(&mut self, blocked: &[bool], dt: f64) {
        let mut due: Vec<usize> = Vec::new();
        for (i, (a, &b)) in self.agents.iter_mut().zip(blocked).enumerate() {
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
                continue;
            }
            let a = &mut self.agents[i];
            a.waiting_s = 0.0;
            a.ignore_conflicts_until = self.time + DEADLOCK_IGNORE_S;
            released.push(i);
        }
    }

    pub(super) fn check_stuck_agents(
        &mut self,
        signal: &[Option<f64>],
        dt: f64,
    ) -> Option<(AgentId, LaneId)> {
        let limit = self.stuck_after_s;
        let mut stuck = None;
        for (a, g) in self.agents.iter_mut().zip(signal) {
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
        stuck
    }
}
