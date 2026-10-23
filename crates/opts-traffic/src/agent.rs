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

    /// Starts a lane change: the car is now on the new lane, `offset` metres beside its centre.
    pub(crate) fn begin_lane_change(&mut self, offset: f64, side: Side) {
        self.lc_from = offset;
        self.lateral = offset;
        self.lc_t = 0.0;
        self.lc_side = Some(side);
    }

    /// Advances the sideways motion of a lane change by `dt`.
    fn glide(&mut self, dt: f64) {
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

/// Lane graph plus the agents driving on it.
#[derive(Debug, Clone)]
pub struct TrafficWorld {
    pub(crate) graph: LaneGraph,
    pub(crate) agents: Vec<Agent>,
    next_id: u64,
    pub(crate) rng: Rng,
    events: Vec<TrafficEvent>,
    /// Simulation time in seconds since the world was created.
    pub(crate) time: f64,
    /// Clock time (seconds of the day) at simulation time zero.
    clock_base: f64,
    signals: HashMap<SignalKey, SignalProgram>,
    /// Speed limit (m/s) assumed on lanes without one.
    open_road_speed: f64,
    /// Vehicles outside the traffic ([`crate::obstacles`]).
    pub(crate) obstacles: Vec<crate::obstacles::Obstacle>,
    /// Phantom agents of the last step (diagnostics).
    pub(crate) last_phantoms: usize,
    /// Seconds standing without a light ahead after which an agent counts as stuck.
    stuck_after_s: f64,
    /// Counters for the behaviour audit ([`TrafficStats`]).
    pub(crate) stats: TrafficStats,
}

/// Behaviour counters since the world was created (M-0111 audit).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrafficStats {
    /// Times an agent's deceleration first exceeded [`HARD_BRAKE`] (m/s²).
    pub hard_brakes: u64,
    /// Lane changes that were due but found no gap and were dropped.
    pub lane_change_aborts: u64,
}

/// Deceleration (m/s², magnitude) above which a braking counts as hard.
pub const HARD_BRAKE: f64 = 3.0;

mod routing;

mod gaps;

mod step;

impl TrafficWorld {
    /// Creates an empty world; `seed` fixes every random choice (route branches, lane changes).
    #[must_use]
    pub fn new(graph: LaneGraph, seed: u64) -> Self {
        Self {
            graph,
            agents: Vec::new(),
            next_id: 1,
            rng: Rng::new(seed),
            events: Vec::new(),
            stuck_after_s: STUCK_AFTER_S,
            time: 0.0,
            clock_base: 0.0,
            signals: HashMap::new(),
            open_road_speed: crate::idm::OPEN_ROAD_SPEED,
            obstacles: Vec::new(),
            last_phantoms: 0,
            stats: TrafficStats::default(),
        }
    }

    /// Behaviour counters since the world was created.
    #[must_use]
    pub fn stats(&self) -> TrafficStats {
        self.stats
    }

    /// Sets the limit (m/s) assumed on lanes without a `speedlimit` rule (default 50 km/h).
    pub fn set_open_road_speed(&mut self, speed: f64) {
        self.open_road_speed = speed.max(0.1);
    }

    /// The speed (m/s) a vehicle with `params` wants on `lane` by itself: the lane speed of the
    /// driver ([`IdmParams::lane_speed`]) and the curve limit of the lane's arc.
    #[must_use]
    pub fn lane_cap(&self, params: &IdmParams, lane: LaneId) -> f64 {
        self.graph.lane(lane).map_or(f64::INFINITY, |l| {
            params
                .lane_speed(l.speed_limit, self.open_road_speed)
                .min(params.curve_speed(l.geometry.radius))
        })
    }

    /// The free-road speed cap of agent `i` (the IDM's desired speed) and the braking it needs
    /// for lower lane speeds ahead (m/s², `≤ 0`, or `+∞` when none).
    ///
    /// The cap is that of its own lane, and of every planned lane within its comfortable braking
    /// distance plus [`SPEED_LOOK_EXTRA_M`], reduced by what it can shed on the way
    /// (`√(v_lim² + 2·b·d)`). Because the IDM's free term lags behind a falling cap, a vehicle
    /// faster than a lane's speed brakes explicitly once it needs at least half its comfortable
    /// deceleration to reach it at the lane's start: it slows before a bend or a lower limit
    /// instead of in it.
    pub(crate) fn speed_cap(&self, i: usize) -> (f64, f64) {
        let a = &self.agents[i];
        let b = a.params.comfort_decel.max(0.1);
        let v = a.speed;
        let reach = v * v / (2.0 * b) + SPEED_LOOK_EXTRA_M;
        let own = self.lane_cap(&a.params, a.lane);
        let mut cap = own;
        let mut brake = if v > own {
            -(v - own) / SPEED_TAU_S
        } else {
            f64::INFINITY
        };
        let Some(lane) = self.graph.lane(a.lane) else {
            return (cap, brake);
        };
        let mut d = lane.length() - a.s;
        for &p in &a.plan {
            if d > reach {
                break;
            }
            let v_lim = self.lane_cap(&a.params, p);
            cap = cap.min((v_lim * v_lim + 2.0 * b * d).sqrt());
            if v > v_lim {
                let need = (v * v - v_lim * v_lim) / (2.0 * (d - SPEED_MARGIN_M).max(0.5));
                if need > 0.5 * b {
                    brake = brake.min(-need);
                }
            }
            d += self.graph.lane(p).map_or(0.0, |l| l.length());
        }
        (cap, brake)
    }

    /// The lane graph.
    #[must_use]
    pub fn graph(&self) -> &LaneGraph {
        &self.graph
    }

    /// All agents in spawn order.
    #[must_use]
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// The agent `id`.
    #[must_use]
    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Sets how long (s) an agent may stand without a light ahead before it is despawned as
    /// stuck (default [`STUCK_AFTER_S`]; `f64::INFINITY` switches the check off).
    pub fn set_stuck_after(&mut self, seconds: f64) {
        self.stuck_after_s = seconds;
    }

    /// Removes an agent (despawn outside the player's range, say).
    pub fn despawn(&mut self, id: AgentId) -> bool {
        let before = self.agents.len();
        self.agents.retain(|a| a.id != id);
        self.agents.len() != before
    }

    /// Sets the time of day (seconds since midnight) of the current simulation time; it keeps
    /// running with [`Self::step`]. Light programs follow this clock.
    pub fn set_clock(&mut self, seconds_of_day: f64) {
        self.clock_base = seconds_of_day - self.time;
    }

    /// The current time of day in seconds (not wrapped).
    #[must_use]
    pub fn clock(&self) -> f64 {
        self.clock_base + self.time
    }

    /// Replaces the light programs (the resident tiles changed).
    pub fn set_signals(&mut self, signals: HashMap<SignalKey, SignalProgram>) {
        self.signals = signals;
    }

    /// The aspect of the light `key` now, if the program is known.
    #[must_use]
    pub fn aspect(&self, key: SignalKey) -> Option<Aspect> {
        self.signals.get(&key).map(|p| p.aspect_at(self.clock()))
    }

    /// Swaps in a rebuilt graph (the resident tiles changed). Agents keep their lane when a lane
    /// with the same [`crate::graph::Lane::key`] exists and lose it otherwise; the latter are
    /// removed. Planned lanes that vanished cut the plan short. Returns the ids of the removed
    /// agents.
    pub fn replace_graph(&mut self, graph: LaneGraph) -> Vec<AgentId> {
        let old = std::mem::replace(&mut self.graph, graph);
        let mut removed = Vec::new();
        let graph = &self.graph;
        self.agents.retain_mut(|a| {
            let remap = |id: LaneId| old.lane(id).and_then(|l| graph.lane_by_key(l.key));
            match remap(a.lane) {
                Some(lane) if a.s <= graph.lane(lane).map_or(0.0, |l| l.length()) + 1e-6 => {
                    a.lane = lane;
                    let mut kept = VecDeque::new();
                    for p in &a.plan {
                        match remap(*p) {
                            Some(n) => kept.push_back(n),
                            None => break,
                        }
                    }
                    a.plan = kept;
                    if a.strategic.take().is_some() {
                        // The plan beyond the change point may no longer fit the new graph.
                        a.plan.clear();
                    }
                    a.prev_lane = a.prev_lane.and_then(remap);
                    a.lc_due = None;
                    // Lanes may have moved: the axles are placed afresh.
                    a.axles_ready = false;
                    true
                }
                _ => {
                    removed.push(a.id);
                    false
                }
            }
        });
        for i in 0..self.agents.len() {
            self.update_axles(i, 0.0);
        }
        removed
    }

    /// Takes the events raised since the last call.
    pub fn drain_events(&mut self) -> Vec<TrafficEvent> {
        std::mem::take(&mut self.events)
    }

    pub(crate) fn push_event(&mut self, e: TrafficEvent) {
        self.events.push(e);
    }

    /// Places an agent at `s` metres on `lane` with `speed`. Fails when the spot is not free
    /// (own length plus minimum gap against every agent on the lane).
    pub fn spawn(
        &mut self,
        lane: LaneId,
        s: f64,
        speed: f64,
        params: IdmParams,
    ) -> Result<AgentId, SpawnError> {
        let length = self
            .graph
            .lane(lane)
            .ok_or(SpawnError::NoSuchLane)?
            .length();
        let s = s.clamp(0.0, length);
        let clash = self.agents.iter().any(|o| {
            o.lane == lane
                && o.s - o.params.length - params.min_gap < s
                && s - params.length - params.min_gap < o.s
        });
        if clash || self.blocked_by_obstacle(lane, s, params.length + params.min_gap) {
            return Err(SpawnError::Occupied);
        }
        let id = AgentId(self.next_id);
        self.next_id += 1;
        self.agents.push(Agent {
            id,
            lane,
            s,
            speed: speed.max(0.0),
            accel: 0.0,
            params,
            plan: VecDeque::new(),
            route: VecDeque::new(),
            stops: VecDeque::new(),
            waiting_s: 0.0,
            stuck_s: 0.0,
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
            lc_cooldown_until: 0.0,
            lc_next_check: self.time + (id.0 % 10) as f64 * 0.1,
            foreign: false,
        });
        let last = self.agents.len() - 1;
        self.extend_plan(last);
        self.update_axles(last, 0.0);
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
