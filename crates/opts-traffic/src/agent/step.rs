//! The simulation step, lane changes and driver commands, split by concern.

use super::*;

mod accel;
mod lanes;
mod timeouts;

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
        if self.lane_change_pass(&by_lane) {
            for i in 0..self.agents.len() {
                self.extend_plan(i);
            }
            by_lane = self.agents_by_lane();
        }
        let looks: Vec<Vec<(LaneId, f64)>> =
            (0..self.agents.len()).map(|i| self.lookahead(i)).collect();
        let signal: Vec<Option<f64>> = (0..self.agents.len())
            .map(|i| self.signal_gap(i, &looks[i]))
            .collect();
        let held: Vec<bool> = self
            .agents
            .iter()
            .zip(&signal)
            .map(|(a, g)| g.is_some_and(|g| g < 12.0) && a.speed < 3.0)
            .collect();

        let (accels, blocked) = self.compute_accelerations(&looks, &signal, &held, &by_lane);
        let travel = self.integrate_agents(&accels, dt);
        self.handle_strategic_timeouts(dt);
        self.resolve_deadlocks(&blocked, dt);

        if let Some((id, lane)) = self.check_stuck_agents(&signal, dt) {
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
