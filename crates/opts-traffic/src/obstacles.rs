//! Vehicles the traffic did not create: the player's bus, a parked session vehicle. The agents
//! have to stop behind them, and change lanes round them, as they do for each other.
//!
//! Each step the host hands over the footprints ([`TrafficWorld::set_obstacles`]); the world turns
//! every footprint into *phantom agents*, one per lane the footprint blocks, placed where the
//! lane's centre line runs through the footprint, standing still or moving with the obstacle.
//! The phantoms take part in the leader search, the lane-change gap checks and the conflict
//! zones like any agent and are removed again at the end of the step, so [`TrafficWorld::agents`]
//! never shows them.

use glam::DVec2;

use crate::agent::{Agent, AgentId, TrafficWorld, LANE_CHANGE_DURATION_S};
use crate::graph::LaneId;
use crate::idm::IdmParams;

/// Half the width of a typical AI vehicle (m): a lane is blocked when its centre line comes closer
/// to the footprint's axis than half the obstacle's width plus this.
pub const AI_HALF_WIDTH_M: f64 = 0.95;
/// Sampling step along a lane (m).
const STEP_M: f64 = 1.0;

/// A vehicle outside the traffic simulation, as a rectangle on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    /// Centre of the footprint, metres (x east, y north), in the frame of the lane graph.
    pub centre: DVec2,
    /// Heading of the long axis, radians, clockwise from north.
    pub heading: f64,
    /// Length of the footprint (m).
    pub length: f64,
    /// Width of the footprint (m).
    pub width: f64,
    /// Velocity (m/s, x east, y north).
    pub velocity: DVec2,
}

impl TrafficWorld {
    /// Sets the obstacles of the next steps (replacing the previous ones; an empty list clears
    /// them). Call it once per frame with the current footprints.
    pub fn set_obstacles(&mut self, obstacles: Vec<Obstacle>) {
        self.obstacles = obstacles;
    }

    /// The obstacles last set.
    #[must_use]
    pub fn obstacles(&self) -> &[Obstacle] {
        &self.obstacles
    }

    /// Whether a vehicle whose front bumper is at `s` on `lane` and that is `span` metres long
    /// (with its gap) would stand in an obstacle's footprint (or its margin of one car width).
    pub(crate) fn blocked_by_obstacle(&self, lane: LaneId, s: f64, span: f64) -> bool {
        let Some(l) = self.graph.lane(lane) else {
            return false;
        };
        self.obstacles.iter().any(|o| {
            let axis = DVec2::new(o.heading.sin(), o.heading.cos());
            let right = DVec2::new(o.heading.cos(), -o.heading.sin());
            let mut x = s;
            while x >= s - span - 1e-9 {
                let r = l.geometry.point_at(x.max(0.0)) - o.centre;
                if r.dot(axis).abs() <= o.length * 0.5 + 2.0
                    && r.dot(right).abs() <= o.width * 0.5 + AI_HALF_WIDTH_M + 0.3
                {
                    return true;
                }
                x -= STEP_M;
            }
            false
        })
    }

    /// Number of lanes the obstacles blocked in the last step (diagnostics).
    #[must_use]
    pub fn phantom_count(&self) -> usize {
        self.last_phantoms
    }

    /// Appends one phantom agent per lane an obstacle blocks (called at the start of a step).
    pub(crate) fn add_phantoms(&mut self) {
        self.last_phantoms = 0;
        if self.obstacles.is_empty() {
            return;
        }
        let mut next = u64::MAX;
        let mut phantoms = Vec::new();
        for o in &self.obstacles {
            let axis = DVec2::new(o.heading.sin(), o.heading.cos());
            let right = DVec2::new(o.heading.cos(), -o.heading.sin());
            let reach = o.length * 0.5 + o.width + 4.0;
            for id in self.graph.lane_ids().collect::<Vec<LaneId>>() {
                let Some(lane) = self.graph.lane(id) else {
                    continue;
                };
                let g = &lane.geometry;
                // A lane whose middle is further away than half its length plus the reach cannot
                // touch the footprint.
                if g.point_at(g.length * 0.5).distance(o.centre) > g.length * 0.5 + reach {
                    continue;
                }
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                let mut s = 0.0;
                while s <= g.length + 1e-9 {
                    let r = g.point_at(s) - o.centre;
                    if r.dot(axis).abs() <= o.length * 0.5
                        && r.dot(right).abs() <= o.width * 0.5 + AI_HALF_WIDTH_M
                    {
                        lo = lo.min(s);
                        hi = hi.max(s);
                    }
                    s += STEP_M;
                }
                if lo > hi {
                    continue;
                }
                let dir = DVec2::new(g.heading_at(hi).sin(), g.heading_at(hi).cos());
                let speed = o.velocity.dot(dir).max(0.0);
                let params = IdmParams {
                    length: (hi - lo + STEP_M).max(1.0),
                    ..IdmParams::default()
                };
                phantoms.push(Agent::phantom(
                    AgentId(next),
                    id,
                    hi + STEP_M * 0.5,
                    speed,
                    params,
                    self.time,
                ));
                next -= 1;
            }
        }
        self.last_phantoms = phantoms.len();
        self.agents.extend(phantoms);
    }

    /// Removes the phantom agents (at the end of a step).
    pub(crate) fn remove_phantoms(&mut self) {
        self.agents.retain(|a| !a.foreign);
    }
}

impl Agent {
    /// A phantom: a standing or moving block on a lane that never plans, changes lanes or leaves.
    pub(crate) fn phantom(
        id: AgentId,
        lane: LaneId,
        s: f64,
        speed: f64,
        params: IdmParams,
        time: f64,
    ) -> Self {
        Self {
            id,
            lane,
            s,
            speed,
            accel: 0.0,
            params,
            plan: std::collections::VecDeque::new(),
            waiting_s: 0.0,
            ignore_conflicts_until: 0.0,
            lateral: 0.0,
            lc_from: 0.0,
            lc_t: LANE_CHANGE_DURATION_S,
            front_axle: DVec2::ZERO,
            rear_axle: DVec2::ZERO,
            steer: 0.0,
            pursuit: DVec2::ZERO,
            prev_lane: None,
            axles_ready: false,
            strategic: None,
            strategic_wait_s: 0.0,
            no_strategic: None,
            lc_side: None,
            lc_due: None,
            lc_cooldown_until: f64::INFINITY,
            lc_next_check: time + 1.0e9,
            foreign: true,
        }
    }
}
