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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

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
        }
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
                    && self.graph.lane(t).is_some_and(|l| l.rules.allows(class));
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
        let allowed: Vec<LaneId> = succ
            .iter()
            .copied()
            .filter(|s| self.graph.lane(*s).is_none_or(|l| l.rules.allows(class)))
            .collect();
        let pool: Vec<LaneId> = if allowed.is_empty() {
            succ.to_vec()
        } else {
            allowed
        };
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
    fn signal_gap(&self, i: usize, look: &[(LaneId, f64)]) -> Option<f64> {
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
    fn conflict_gap(
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
    fn shares_conflict(&self, i: usize, j: usize) -> bool {
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
            for gap in [signal[i], wait, stuck].into_iter().flatten() {
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
            a.accel = acc;
            a.glide(dt);
        }
        self.time += dt;
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
            if !a.foreign && a.speed < 0.2 && g.is_none() {
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

#[cfg(test)]
mod tests;
