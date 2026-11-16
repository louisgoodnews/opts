//! Accelerations of all agents for one step and their integration into positions and speeds.

use super::*;

impl TrafficWorld {
    pub(super) fn compute_accelerations(
        &self,
        looks: &[Vec<(LaneId, f64)>],
        signal: &[Option<f64>],
        held: &[bool],
        by_lane: &HashMap<LaneId, Vec<usize>>,
    ) -> (Vec<f64>, Vec<bool>) {
        let mut accels = Vec::with_capacity(self.agents.len());
        let mut blocked = vec![false; self.agents.len()];
        for (i, a) in self.agents.iter().enumerate() {
            if a.foreign || self.graph.lane(a.lane).is_none() {
                accels.push(0.0);
                continue;
            }
            let leader = self.leader(i, &looks[i], by_lane);
            let wait = self.conflict_gap(i, &looks[i], by_lane, held);
            blocked[i] = wait.is_some();
            let stuck = a.strategic.and_then(|c| {
                let (_, off) = looks[i].iter().find(|(l, _)| *l == c.from)?;
                let len = self.graph.lane(c.from)?.length();
                Some((off + len - STRATEGIC_STOP_M).max(0.0))
            });
            let mut ahead = leader;
            let stop = self.stop_gap(i, &looks[i]);
            for gap in [signal[i], wait, stuck, stop].into_iter().flatten() {
                if ahead.is_none_or(|(g, _)| gap < g) {
                    ahead = Some((gap, 0.0));
                }
            }
            let (cap, brake) = self.speed_cap(i);
            let p = self.driver(&a.params);
            let acc = p.acceleration(a.speed, cap, ahead).min(brake);
            accels.push(acc.max(-p.max_decel));
        }
        (accels, blocked)
    }

    pub(super) fn integrate_agents(&mut self, accels: &[f64], dt: f64) -> Vec<f64> {
        let travel: Vec<f64> = self
            .agents
            .iter()
            .zip(accels)
            .map(|(a, &acc)| (a.speed.max(a.speed + acc * dt) + MAX_GLIDE_SPEED) * dt)
            .collect();
        let mut hard = 0;
        for (a, &acc) in self.agents.iter_mut().zip(accels) {
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
            if acc < -HARD_BRAKE && a.accel >= -super::HARD_BRAKE {
                hard += 1;
            }
            a.accel = acc;
            a.glide(dt);
        }
        self.stats.hard_brakes += hard;
        self.time += dt;
        self.update_stops();
        travel
    }
}
