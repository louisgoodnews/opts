//! Traffic agents and the world that steps them.
//!
//! The first fidelity level is **kinematic**: [`TrafficWorld::step`] integrates every agent along
//! its lane with the IDM and the host places a vehicle model at [`Agent::pose`]. The second level,
//! driving a real vehicle simulation through its scripts, uses [`TrafficWorld::command`] — the
//! pedal and indicator values OMSI's `{frame}` blocks of AI vehicles read (`Throttle`, `Brake`,
//! `AI_Blinker_L/R`; `docs/src/architecture/ai-traffic.md`, "The vehicle contract").
//!
//! What limits a vehicle each step, nearest first: the leader (same lane or ahead along the
//! agent's planned lanes), a traffic light it must stop for ([`crate::signals`]), a conflict zone
//! it must give way at ([`crate::conflict`]). Lane changes ([`crate::lanechange`]) are decided
//! before the accelerations are computed.

use std::collections::{HashMap, VecDeque};

use glam::DVec2;

use crate::graph::{LaneGraph, LaneId, SpawnError};
use crate::idm::IdmParams;
use crate::neighbours::Side;
use crate::rightofway::{precedence, Precedence};
use crate::rng::Rng;
use crate::signals::{Aspect, SignalKey, SignalProgram};

mod stops;
pub use stops::{StopPoint, ARRIVED_SPEED, ARRIVE_WITHIN_M};

/// Identifies an agent for its lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(pub u64);

/// Indicator request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blinker {
    /// Off.
    #[default]
    Off,
    /// Left indicator (`AI_Blinker_L`).
    Left,
    /// Right indicator (`AI_Blinker_R`).
    Right,
}

/// What the AI "driver" asks of a vehicle this step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DriverCommand {
    /// Accelerator `0..=1` (`Throttle`).
    pub throttle: f32,
    /// Service brake `0..=1` (`Brake`).
    pub brake: f32,
    /// Indicator.
    pub blinker: Blinker,
}

/// Something the host may want to react to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrafficEvent {
    /// The agent drove off the end of a lane that has no successor and was removed.
    ReachedDeadEnd {
        /// The removed agent.
        id: AgentId,
        /// The lane it left.
        lane: LaneId,
    },
    /// The agent moved over to a neighbouring lane (its sideways motion takes
    /// [`LANE_CHANGE_DURATION_S`] seconds).
    LaneChanged {
        /// The agent.
        id: AgentId,
        /// Side it moved to.
        side: Side,
    },
    /// A scheduled agent stopped at its stop point (see [`StopPoint`]).
    ArrivedAtStop {
        /// The agent.
        id: AgentId,
        /// The station.
        station: u32,
        /// Clock time (seconds of the day).
        clock: f64,
    },
    /// A scheduled agent left its stop point.
    DepartedStop {
        /// The agent.
        id: AgentId,
        /// The station.
        station: u32,
        /// Clock time (seconds of the day).
        clock: f64,
    },
    /// A scheduled agent passed the lane of a stop without standing there; the stop is dropped.
    StopMissed {
        /// The agent.
        id: AgentId,
        /// The station.
        station: u32,
    },
    /// The agent stood still for [`STUCK_AFTER_S`] seconds without a red light or a lane change
    /// to wait for, and was removed (forced despawn) — the first agent to get stuck goes.
    Stuck {
        /// The removed agent.
        id: AgentId,
        /// The lane it stood on.
        lane: LaneId,
    },
}

/// Seconds a lane change takes sideways (smooth start and end).
pub const LANE_CHANGE_DURATION_S: f64 = 3.0;

/// A planned (mandatory) lane change: the plan continues from a successor of the neighbour
/// lane `via`, so the vehicle must move over to it while on lane `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strategic {
    /// The lane on which the change has to happen.
    pub from: LaneId,
    /// The neighbour lane to change to.
    pub via: LaneId,
    /// Its side.
    pub side: Side,
}

/// A vehicle on a lane.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// Identity.
    pub id: AgentId,
    /// Current lane.
    pub lane: LaneId,
    /// Position of the front bumper along the lane, metres.
    pub s: f64,
    /// Speed in m/s.
    pub speed: f64,
    /// Acceleration of the last step in m/s².
    pub accel: f64,
    /// Driver and vehicle parameters.
    pub params: IdmParams,
    /// The lanes that follow the current one (chosen ahead of time, at least ~150 m).
    pub plan: VecDeque<LaneId>,
    /// A fixed route of a scheduled vehicle: the lanes the plan takes next, before any random
    /// choice ([`TrafficWorld::set_route`]).
    pub route: VecDeque<LaneId>,
    /// The stops of a scheduled vehicle in running order ([`TrafficWorld::push_stop`]).
    pub stops: VecDeque<StopPoint>,
    /// Seconds spent standing still because of a conflict zone.
    pub waiting_s: f64,
    /// Seconds standing still without a red light ahead (the "vehicle stuck" timer).
    pub stuck_s: f64,
    /// World time until which conflicts are ignored (breaks deadlocks).
    pub ignore_conflicts_until: f64,
    /// Sideways offset from the lane's centre line in metres (positive = right); non-zero while a
    /// lane change is still being driven.
    pub lateral: f64,
    /// Offset at the start of the running lane change and seconds since then.
    pub(crate) lc_from: f64,
    pub(crate) lc_t: f64,
    /// Front axle in the world: on the lane (plus the lane-change offset).
    pub front_axle: DVec2,
    /// Rear axle in the world: dragged behind the front axle at the wheelbase (a tractrix).
    pub rear_axle: DVec2,
    /// Steering angle of the front wheels (rad, positive = right): the lane's heading at the
    /// front axle minus the body's heading.
    pub steer: f64,
    /// The path point the front axle pursued at the last update.
    pub(crate) pursuit: DVec2,
    /// The lane before the current one (the front axle may still be on it).
    pub(crate) prev_lane: Option<LaneId>,
    /// The axles were placed (else the next update places them afresh).
    pub(crate) axles_ready: bool,
    /// A lane change the route needs (see [`Strategic`]).
    pub strategic: Option<Strategic>,
    /// Seconds spent waiting at the end of a lane for a strategic change.
    pub(crate) strategic_wait_s: f64,
    /// No strategic route choice from this lane (a change there timed out).
    pub(crate) no_strategic: Option<LaneId>,
    /// Side of the lane change being driven or announced.
    pub(crate) lc_side: Option<Side>,
    /// Announced lane change: the indicator is on until this world time, then it is executed if
    /// still safe.
    pub(crate) lc_due: Option<f64>,
    /// No new lane change before this world time; next evaluation time.
    pub(crate) lc_cooldown_until: f64,
    pub(crate) lc_next_check: f64,
    /// A phantom standing for a vehicle outside the traffic ([`crate::obstacles`]): only exists
    /// during a step.
    pub(crate) foreign: bool,
}

impl Agent {
    /// The lane taken after this one, if any.
    #[must_use]
    pub fn next(&self) -> Option<LaneId> {
        self.plan.front().copied()
    }

    /// Whether the sideways motion of a lane change is still going on.
    #[must_use]
    pub fn changing_lane(&self) -> bool {
        self.lc_t < LANE_CHANGE_DURATION_S && self.lc_from != 0.0
    }

    /// Compass heading (radians) of the body: from the rear axle to the front axle.
    #[must_use]
    pub fn heading(&self) -> f64 {
        let d = self.front_axle - self.rear_axle;
        d.x.atan2(d.y)
    }

    /// World position and compass heading (radians) of the front bumper of the body (ahead of
    /// the front axle along the body's heading). `None` when the agent's lane is gone.
    #[must_use]
    pub fn pose(&self, graph: &LaneGraph) -> Option<(DVec2, f64)> {
        graph.lane(self.lane)?;
        let h = self.heading();
        let dir = DVec2::new(h.sin(), h.cos());
        Some((self.front_axle + dir * self.params.front_overhang, h))
    }

    /// Height of the front bumper in metres (from the lane's elevation profile).
    #[must_use]
    pub fn height(&self, graph: &LaneGraph) -> Option<f64> {
        Some(graph.lane(self.lane)?.z_at(self.s))
    }

    /// Height of the rear axle (m) and the pitch of the body (rad, nose up positive) from the
    /// lane's elevation profile under the two axles: on a slope the body lies along it instead of
    /// hanging at the front axle's height. Across a lane's start the profile is clamped, which
    /// gives a flat stretch at most one wheelbase long.
    #[must_use]
    pub fn ground_pose(&self, graph: &LaneGraph) -> Option<(f64, f64)> {
        let lane = graph.lane(self.lane)?;
        // `s` is the front bumper; the axles lie behind it.
        let front_s = (self.s - self.params.front_overhang).max(0.0);
        let wheelbase = self.params.wheelbase.max(1.0);
        let rear_s = (front_s - wheelbase).max(0.0);
        let front = lane.z_at(front_s);
        let span = front_s - rear_s;
        if span < 0.5 {
            return Some((front, 0.0));
        }
        let rear = lane.z_at(rear_s);
        let pitch = (front - rear).atan2(span);
        // The axles are one wheelbase apart: extrapolate the rear height when the lane starts
        // closer than that.
        Some((front - pitch.tan() * wheelbase, pitch))
    }

    /// Starts a lane change: the car is now on the new lane, `offset` metres beside its centre.
    pub(crate) fn begin_lane_change(&mut self, offset: f64, side: Side) {
        self.lc_from = offset;
        self.lateral = offset;
        self.lc_t = 0.0;
        self.lc_side = Some(side);
    }

    /// Advances the sideways motion of a lane change by `dt`.
    pub(super) fn glide(&mut self, dt: f64) {
        if self.lc_from == 0.0 || self.lc_t >= LANE_CHANGE_DURATION_S {
            self.lateral = 0.0;
            if self.lc_due.is_none() {
                self.lc_side = None;
            }
            return;
        }
        self.lc_t = (self.lc_t + dt).min(LANE_CHANGE_DURATION_S);
        let u = self.lc_t / LANE_CHANGE_DURATION_S;
        // Smoothstep: zero lateral speed at both ends. The body yaws by itself: the rear axle
        // follows the front axle's sideways motion.
        self.lateral = self.lc_from * (1.0 - u * u * (3.0 - 2.0 * u));
        if self.lc_t >= LANE_CHANGE_DURATION_S {
            self.lateral = 0.0;
            self.lc_from = 0.0;
            if self.lc_due.is_none() {
                self.lc_side = None;
            }
        }
    }
}

/// Conflict zones further ahead than this (m) are not considered.
const CONFLICT_LOOKAHEAD_M: f64 = 60.0;
/// Vehicles stop this far (m) before a conflict zone they have to wait for.
const CONFLICT_STOP_MARGIN_M: f64 = 1.5;
/// Speed (m/s) assumed for arrival-time estimates of slow or standing vehicles.
const MIN_ARRIVAL_SPEED: f64 = 2.0;
/// A vehicle without right of way waits for others that reach the zone within this time (s).
const GAP_ACCEPTANCE_S: f64 = 6.0;
/// A vehicle that has stood this long (s) at a conflict zone goes anyway …
const DEADLOCK_AFTER_S: f64 = 12.0;
/// A vehicle standing this long (s) with no red light ahead is stuck and is despawned.
pub const STUCK_AFTER_S: f64 = 60.0;
/// A released vehicle still gives way to one inside the zone that moves faster than this (m/s).
const RELEASE_RESPECT_SPEED: f64 = 0.5;
/// … and ignores conflicts for this long (s).
const DEADLOCK_IGNORE_S: f64 = 5.0;
/// How far along its planned lanes an agent looks for leaders, signals and zones (m).
pub(crate) const LOOKAHEAD_M: f64 = 100.0;
/// The plan covers at least this much road ahead (m).
const PLAN_HORIZON_M: f64 = 150.0;
/// Vehicles stop this far (m) before the start of a lane whose light is red.
const SIGNAL_STOP_MARGIN_M: f64 = 1.0;
/// Deceleration (m/s²) a driver accepts to stop for a yellow light.
const YELLOW_COMFORT_DECEL: f64 = 3.0;
/// Light-checking distance (m) when the program does not name one (`[approachdist]` `0`).
const DEFAULT_APPROACH_M: f64 = 60.0;

/// Slack (m) on the front axle's motion before the rear axle is placed afresh.
const AXLE_RESET_M: f64 = 0.3;
/// Distance (m) of the point on the path the front axle pursues.
const FRONT_PURSUIT_M: f64 = 1.0;
/// Largest travel (m) of one sub-step of the axle update.
const AXLE_SUBSTEP_M: f64 = 0.2;
/// Largest sideways speed (m/s) of a lane change's glide, for the axle jump check.
const MAX_GLIDE_SPEED: f64 = 3.0;

/// Distance (m) beyond the comfortable braking distance in which lower lane speeds are seen.
const SPEED_LOOK_EXTRA_M: f64 = 15.0;
/// A strategic change must be made this far (m) before the end of its lane …
pub(crate) const STRATEGIC_LAST_M: f64 = 15.0;
/// … where the vehicle otherwise stops this far (m) before the end and waits …
const STRATEGIC_STOP_M: f64 = 1.0;
/// … for at most this long (s) before it re-plans from its own lane.
const STRATEGIC_TIMEOUT_S: f64 = 10.0;
/// Lanes shorter than this (m) offer no strategic changes.
const STRATEGIC_MIN_LANE_M: f64 = 30.0;

/// Vehicles reach a lower lane speed this far (m) before the lane's start.
const SPEED_MARGIN_M: f64 = 2.0;
/// Time constant (s) of shedding excess speed on the current lane.
const SPEED_TAU_S: f64 = 1.0;

/// Indicators are requested when the next lane turns by more than this (radians) …
const BLINK_TURN: f64 = 20.0 * std::f64::consts::PI / 180.0;
/// … and the lane end is closer than this (metres).
const BLINK_DISTANCE: f64 = 30.0;

mod gaps;
mod routing;
mod step;
mod world;
pub use world::{TrafficStats, TrafficWorld, HARD_BRAKE};

#[cfg(test)]
mod tests;
