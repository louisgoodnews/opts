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
            time: 0.0,
            clock_base: 0.0,
            signals: HashMap::new(),
            open_road_speed: crate::idm::OPEN_ROAD_SPEED,
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
                || nl.rules.density == Some(0.0)
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
            self.strategic_options(lane, class)
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
            let class = a.params.class;
            match self.pick_next(last, class, strategic) {
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
        if clash {
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
        if a.ignore_conflicts_until > self.time {
            return None;
        }
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
            if self.graph.lane(a.lane).is_none() {
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
        // A vehicle that waited too long at a zone goes anyway (cyclic waits, ghost vehicles).
        for (a, &b) in self.agents.iter_mut().zip(&blocked) {
            if b && a.speed < 0.2 {
                a.waiting_s += dt;
                if a.waiting_s > DEADLOCK_AFTER_S {
                    a.waiting_s = 0.0;
                    a.ignore_conflicts_until = self.time + DEADLOCK_IGNORE_S;
                }
            } else {
                a.waiting_s = 0.0;
            }
        }
        let ids: Vec<AgentId> = self.agents.iter().map(|a| a.id).collect();
        self.advance_lanes();
        // Axles after the lane transitions (agents removed at dead ends drop out of `ids`).
        let mut k = 0;
        for (id, t) in ids.into_iter().zip(travel) {
            if self.agents.get(k).is_some_and(|a| a.id == id) {
                self.update_axles(k, t);
                k += 1;
            }
        }
    }

    /// Moves agents that passed the end of their lane onto the next one.
    fn advance_lanes(&mut self) {
        let mut i = 0;
        while i < self.agents.len() {
            loop {
                let a = &self.agents[i];
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
        if blinker == Blinker::Off {
            if let Some(next) = a.next().and_then(|n| self.graph.lane(n)) {
                if lane.length() - a.s < BLINK_DISTANCE {
                    let turn = (next.geometry.end_heading() - lane.geometry.end_heading()
                        + std::f64::consts::PI)
                        .rem_euclid(std::f64::consts::TAU)
                        - std::f64::consts::PI;
                    blinker = if turn > BLINK_TURN {
                        Blinker::Right
                    } else if turn < -BLINK_TURN {
                        Blinker::Left
                    } else {
                        Blinker::Off
                    };
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
mod tests {
    use super::*;
    use crate::geometry::PathGeometry;
    use crate::graph::LaneRules;
    use crate::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
    use crate::neighbours::Side;
    use crate::signals::SignalProgram;

    fn straight(y: f64, len: f64) -> LaneSegment {
        LaneSegment::straight(DVec2::new(0.0, y), 0.0, len, LaneKind::Road)
    }

    fn world(segments: &[LaneSegment]) -> TrafficWorld {
        TrafficWorld::new(LaneGraph::build(segments, &ConnectParams::default()), 3)
    }

    #[test]
    fn an_agent_accelerates_and_crosses_into_the_next_lane() {
        let mut w = world(&[straight(0.0, 60.0), straight(60.0, 400.0)]);
        let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
        for _ in 0..200 {
            w.step(0.1);
        }
        let a = w.agent(id).unwrap();
        assert_eq!(a.lane, LaneId(1));
        assert!(a.speed > 10.0);
        let (p, h) = a.pose(w.graph()).unwrap();
        assert!(p.y > 60.0 && p.x.abs() < 1e-9 && h.abs() < 1e-9);
    }

    #[test]
    fn a_follower_keeps_its_distance_behind_a_stopped_leader() {
        let mut w = world(&[straight(0.0, 300.0)]);
        let lead = w
            .spawn(LaneId(0), 200.0, 0.0, IdmParams::default())
            .unwrap();
        let p = IdmParams {
            max_accel: 0.0,
            ..IdmParams::default()
        };
        // A leader that never moves: zero acceleration means it stays at speed 0.
        w.agents.iter_mut().find(|a| a.id == lead).unwrap().params = p;
        let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
        for _ in 0..1200 {
            w.step(0.1);
        }
        let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
        let gap = l.s - l.params.length - f.s;
        assert!(f.speed < 0.05, "follower still at {}", f.speed);
        assert!(gap > 0.5, "collision: gap {gap}");
        assert!(gap < 4.0, "stopped too early: gap {gap}");
    }

    #[test]
    fn agents_see_a_leader_across_the_lane_boundary() {
        let mut w = world(&[straight(0.0, 100.0), straight(100.0, 100.0)]);
        let lead = w
            .spawn(
                LaneId(1),
                10.0,
                0.0,
                IdmParams {
                    max_accel: 0.0,
                    ..IdmParams::default()
                },
            )
            .unwrap();
        let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
        for _ in 0..1200 {
            w.step(0.1);
        }
        let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
        assert_eq!(f.lane, LaneId(1));
        assert!(l.s - l.params.length - f.s > 0.5, "collision");
    }

    #[test]
    fn dead_ends_remove_the_agent_and_raise_an_event() {
        let mut w = world(&[straight(0.0, 30.0)]);
        let id = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
        for _ in 0..100 {
            w.step(0.1);
        }
        assert!(w.agent(id).is_none());
        assert_eq!(
            w.drain_events(),
            [TrafficEvent::ReachedDeadEnd {
                id,
                lane: LaneId(0)
            }]
        );
        assert!(w.drain_events().is_empty());
    }

    #[test]
    fn replacing_the_graph_keeps_agents_on_lanes_that_still_exist() {
        let mut a = straight(0.0, 100.0);
        a.source = 1;
        let mut b = straight(100.0, 100.0);
        b.source = 2;
        let mut w = world(&[a.clone(), b.clone()]);
        let on_a = w.spawn(LaneId(0), 50.0, 5.0, IdmParams::default()).unwrap();
        let on_b = w.spawn(LaneId(1), 50.0, 5.0, IdmParams::default()).unwrap();
        // Tile of lane `a` unloads; lane `b` stays but now has id 0.
        let removed = w.replace_graph(LaneGraph::build(&[b], &ConnectParams::default()));
        assert_eq!(removed, [on_a]);
        assert!(w.agent(on_a).is_none());
        assert_eq!(w.agent(on_b).unwrap().lane, LaneId(0));
        // Loading `a` again does not bring the agent back, and `on_b` keeps its lane by key.
        let removed = w.replace_graph(LaneGraph::build(
            &[a, straight(100.0, 100.0)],
            &ConnectParams::default(),
        ));
        assert!(removed.is_empty() || removed == [on_b]);
    }

    #[test]
    fn spawning_needs_room() {
        let mut w = world(&[straight(0.0, 100.0)]);
        w.spawn(LaneId(0), 20.0, 0.0, IdmParams::default()).unwrap();
        assert_eq!(
            w.spawn(LaneId(0), 22.0, 0.0, IdmParams::default()),
            Err(SpawnError::Occupied)
        );
        assert!(w.spawn(LaneId(0), 40.0, 0.0, IdmParams::default()).is_ok());
        assert_eq!(
            w.spawn(LaneId(9), 0.0, 0.0, IdmParams::default()),
            Err(SpawnError::NoSuchLane)
        );
    }

    #[test]
    fn commands_follow_the_acceleration_and_announce_turns() {
        // A right turn (quarter circle, radius 15) after 100 m.
        let turn = LaneSegment {
            geometry: PathGeometry {
                start: DVec2::new(0.0, 100.0),
                heading: 0.0,
                radius: 15.0,
                length: 15.0 * std::f64::consts::FRAC_PI_2,
            },
            kind: LaneKind::Road,
            width: 2.5,
            direction: Direction::Forward,
            speed_limit: None,
            rules: Default::default(),
            elevation: Default::default(),
            signal: None,
            junction: false,
            source: 0,
        };
        let mut w = world(&[straight(0.0, 100.0), turn]);
        let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
        w.step(0.1);
        let c = w.command(id).unwrap();
        assert!(c.throttle > 0.9 && c.brake == 0.0 && c.blinker == Blinker::Off);
        w.agents[0].s = 80.0;
        assert_eq!(w.command(id).unwrap().blinker, Blinker::Right);
    }

    /// Smallest distance between the bodies (front bumper to rear) of two agents over a run.
    fn min_body_distance(w: &mut TrafficWorld, steps: usize) -> f64 {
        let mut min = f64::INFINITY;
        for _ in 0..steps {
            w.step(0.1);
            let pts: Vec<Vec<DVec2>> = w
                .agents()
                .iter()
                .map(|a| {
                    let g = &w.graph().lane(a.lane).unwrap().geometry;
                    (0..=4)
                        .map(|k| g.point_at((a.s - a.params.length * f64::from(k) / 4.0).max(0.0)))
                        .collect()
                })
                .collect();
            for i in 0..pts.len() {
                for j in i + 1..pts.len() {
                    for p in &pts[i] {
                        for q in &pts[j] {
                            min = min.min(p.distance(*q));
                        }
                    }
                }
            }
        }
        min
    }

    #[test]
    fn crossing_flows_take_turns_without_touching() {
        use std::f64::consts::FRAC_PI_2;
        // North-bound and east-bound lanes cross at (0, 100); both vehicles arrive together.
        let north = straight(0.0, 200.0);
        let east =
            LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 200.0, LaneKind::Road);
        let mut w = world(&[north, east]);
        let a = w
            .spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
            .unwrap();
        let b = w
            .spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
            .unwrap();
        let min = min_body_distance(&mut w, 400);
        assert!(min > 1.4, "bodies came within {min} m");
        // Both got through (and left at the dead ends) without a deadlock.
        assert!(w.agent(a).is_none() && w.agent(b).is_none());
    }

    #[test]
    fn without_conflict_handling_the_same_crossing_would_collide() {
        use std::f64::consts::FRAC_PI_2;
        // Negative control for the test above: with conflicts ignored the bodies overlap.
        let north = straight(0.0, 200.0);
        let east =
            LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 200.0, LaneKind::Road);
        let mut w = world(&[north, east]);
        w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
            .unwrap();
        w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
            .unwrap();
        for a in &mut w.agents {
            a.ignore_conflicts_until = f64::INFINITY;
        }
        assert!(min_body_distance(&mut w, 400) < 1.0);
    }

    #[test]
    fn merging_lanes_take_turns() {
        let mut w = world(&[
            straight(0.0, 60.0),
            straight(60.0, 100.0),
            LaneSegment::straight(DVec2::new(-0.2, 10.0), 0.0, 50.2, LaneKind::Road),
        ]);
        w.spawn(LaneId(0), 30.0, 10.0, IdmParams::default())
            .unwrap();
        w.spawn(LaneId(2), 30.0, 10.0, IdmParams::default())
            .unwrap();
        let min = min_body_distance(&mut w, 400);
        assert!(min > 1.4, "bodies came within {min} m");
        assert!(w.agents().is_empty());
    }

    #[test]
    fn parallel_traffic_is_not_held_up() {
        let neighbour = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 400.0, LaneKind::Road);
        let mut w = world(&[straight(0.0, 400.0), neighbour]);
        let a = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
        w.spawn(LaneId(1), 0.0, 12.0, IdmParams::default()).unwrap();
        for _ in 0..100 {
            w.step(0.1);
        }
        assert!(w.agent(a).unwrap().speed > 10.0);
    }

    #[test]
    fn a_stuck_vehicle_goes_anyway_after_the_deadlock_timeout() {
        use std::f64::consts::FRAC_PI_2;
        // A stands in the middle of the crossing at speed 0 with no way to leave it (its desired
        // speed is 0), B approaches on the other lane: B waits, then goes after the timeout.
        let north = straight(0.0, 200.0);
        let east =
            LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 200.0, LaneKind::Road);
        let mut w = world(&[north, east]);
        let parked = IdmParams {
            desired_speed: 0.1,
            max_accel: 0.0,
            ..IdmParams::default()
        };
        w.spawn(LaneId(0), 101.0, 0.0, parked).unwrap();
        let b = w.spawn(LaneId(1), 40.0, 8.0, IdmParams::default()).unwrap();
        for _ in 0..600 {
            w.step(0.1);
        }
        assert!(
            w.agent(b).is_none_or(|a| a.s > 100.0),
            "B never passed the crossing"
        );
    }

    /// Two parallel north-bound lanes 3.5 m apart: lane 0 (left) and lane 1 (right, east).
    fn two_lanes() -> TrafficWorld {
        let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
        world(&[straight(0.0, 800.0), right])
    }

    fn slow() -> IdmParams {
        IdmParams {
            desired_speed: 3.0,
            ..IdmParams::default()
        }
    }

    #[test]
    fn a_car_behind_a_slow_one_changes_lanes_without_hard_braking() {
        let mut w = two_lanes();
        w.spawn(LaneId(0), 110.0, 3.0, slow()).unwrap();
        let fast = w
            .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
            .unwrap();
        let (mut worst, mut changed) = (0.0f64, false);
        for _ in 0..400 {
            w.step(0.1);
            worst = worst.min(w.agents().iter().map(|a| a.accel).fold(0.0, f64::min));
            changed |= w
                .drain_events()
                .iter()
                .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == fast));
        }
        assert!(changed, "the fast car never changed lanes");
        assert!(worst > -2.2, "someone braked at {worst} m/s²");
        // The slow car did not hold it up for the whole run.
        assert!(w.agent(fast).is_none_or(|a| a.speed > 10.0));
    }

    #[test]
    fn the_target_lane_has_priority_over_a_lane_change() {
        let mut w = two_lanes();
        // A slow leader makes the changer want to leave; a fast car closes in from behind in the
        // target lane, so it has to wait until that car has passed.
        w.spawn(LaneId(0), 124.0, 3.0, slow()).unwrap();
        let changer = w
            .spawn(LaneId(0), 100.0, 9.0, IdmParams::default())
            .unwrap();
        let fast = w
            .spawn(
                LaneId(1),
                55.0,
                20.0,
                IdmParams {
                    desired_speed: 20.0,
                    // 20 m/s needs a factor on the 50 km/h open-road limit (C3).
                    speed_factor: 1.5,
                    ..IdmParams::default()
                },
            )
            .unwrap();
        let mut worst_fast = 0.0f64;
        for _ in 0..250 {
            w.step(0.1);
            let (c, f) = (w.agent(changer), w.agent(fast));
            if let (Some(c), Some(f)) = (c, f) {
                worst_fast = worst_fast.min(f.accel);
                // While the fast car is still behind (or beside) the changer, no move into its lane.
                if f.s < c.s + 2.0 {
                    assert_eq!(
                        c.lane,
                        LaneId(0),
                        "cut in front of a fast car at t = {}",
                        w.time
                    );
                }
            }
        }
        assert!(
            worst_fast > -1.5,
            "the fast car braked at {worst_fast} m/s²"
        );
    }

    #[test]
    fn a_free_right_lane_is_taken_and_the_right_lane_is_kept() {
        let mut w = two_lanes();
        let left = w
            .spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
            .unwrap();
        let right = w
            .spawn(LaneId(1), 300.0, 12.0, IdmParams::default())
            .unwrap();
        let mut events = Vec::new();
        for _ in 0..300 {
            w.step(0.1);
            events.extend(w.drain_events());
        }
        assert!(events.contains(&TrafficEvent::LaneChanged {
            id: left,
            side: Side::Right
        }));
        assert!(!events
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == right)));
        assert_eq!(w.agent(left).unwrap().lane, LaneId(1));
    }

    #[test]
    fn a_lane_change_is_continuous_to_the_eye_and_glides_to_the_centre() {
        let mut w = two_lanes();
        w.spawn(LaneId(0), 110.0, 3.0, slow()).unwrap();
        let id = w
            .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
            .unwrap();
        let mut glided = false;
        for _ in 0..400 {
            let before = w.agent(id).and_then(|a| a.pose(w.graph()));
            w.step(0.1);
            let changed = w
                .drain_events()
                .iter()
                .any(|e| matches!(e, TrafficEvent::LaneChanged { id: who, .. } if *who == id));
            let Some(a) = w.agent(id).cloned() else { break };
            if changed {
                let (b, after) = (before.unwrap().0, a.pose(w.graph()).unwrap().0);
                // Travel plus a little: the car must not jump a lane width sideways.
                assert!(
                    b.distance(after) < 14.0 * 0.1 + 0.6,
                    "jumped {} m",
                    b.distance(after)
                );
                assert!(a.changing_lane() && a.lateral.abs() > 3.0);
                glided = true;
                for _ in 0..32 {
                    w.step(0.1);
                }
                let a = w.agent(id).unwrap();
                assert!(
                    !a.changing_lane() && a.lateral == 0.0,
                    "still offset by {}",
                    a.lateral
                );
                break;
            }
        }
        assert!(glided);
    }

    #[test]
    fn no_lane_change_is_started_towards_a_prohibited_overtaking_side() {
        // The right lane is the free one; the changer sits on it and the left lane forbids
        // overtaking, so it must not move left past the slow leader.
        let mut left = straight(0.0, 800.0);
        left.rules.no_overtaking = true;
        let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
        let mut w = world(&[left, right]);
        w.spawn(LaneId(1), 110.0, 3.0, slow()).unwrap();
        w.spawn(LaneId(1), 30.0, 14.0, IdmParams::default())
            .unwrap();
        for _ in 0..300 {
            w.step(0.1);
        }
        assert!(w.drain_events().iter().all(|e| !matches!(
            e,
            TrafficEvent::LaneChanged {
                side: Side::Left,
                ..
            }
        )));
    }

    /// A lane of 100 m followed by a lane governed by light `(1, 0)`.
    fn light_world(program: SignalProgram) -> TrafficWorld {
        let mut gated = straight(100.0, 200.0);
        gated.signal = Some((1, 0));
        let mut w = world(&[straight(0.0, 100.0), gated]);
        w.set_signals(HashMap::from([((1, 0), program)]));
        w.set_clock(0.0);
        w
    }

    #[test]
    fn vehicles_stop_at_a_red_light_and_go_on_green() {
        let mut w = light_world(SignalProgram::new(vec![(0, 20.0), (6, 100.0)], 50.0).unwrap());
        let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
        for _ in 0..150 {
            w.step(0.1);
        }
        let a = w.agent(id).unwrap();
        assert_eq!(a.lane, LaneId(0), "ran the red light");
        assert!(
            a.speed < 0.2 && a.s > 90.0 && a.s < 100.0,
            "stopped at s = {}",
            a.s
        );
        for _ in 0..100 {
            w.step(0.1);
        }
        let a = w.agent(id).unwrap();
        assert_eq!(a.lane, LaneId(1));
        assert!(a.speed > 5.0);
    }

    #[test]
    fn a_vehicle_too_close_to_stop_runs_the_yellow() {
        let mut w = light_world(SignalProgram::new(vec![(9, 10.0), (0, 30.0)], 0.0).unwrap());
        let late = w
            .spawn(LaneId(0), 90.0, 14.0, IdmParams::default())
            .unwrap();
        for _ in 0..30 {
            w.step(0.1);
        }
        assert_eq!(
            w.agent(late).unwrap().lane,
            LaneId(1),
            "stopped hard for a yellow"
        );
        assert!(w.agent(late).unwrap().speed > 10.0);
    }

    /// Two perpendicular roads crossing at (0, 100): lane 0 north-bound, lane 1 west-bound (it comes
    /// from the north-bound driver's right), both 200 m long.
    fn crossing(a: LaneRules, b: LaneRules) -> TrafficWorld {
        use std::f64::consts::FRAC_PI_2;
        let mut north = straight(0.0, 200.0);
        north.rules = a;
        let mut west = LaneSegment::straight(
            DVec2::new(100.0, 100.0),
            3.0 * FRAC_PI_2,
            200.0,
            LaneKind::Road,
        );
        west.rules = b;
        world(&[north, west])
    }

    /// Slowest speed each agent had while it was within 60 m of the crossing, plus the closest
    /// approach of the two bodies.
    fn run_crossing(w: &mut TrafficWorld, steps: usize) -> ([f64; 2], f64) {
        let mut slowest = [f64::INFINITY; 2];
        let mut closest = f64::INFINITY;
        for _ in 0..steps {
            w.step(0.1);
            let mut pos = Vec::new();
            for (k, a) in w.agents().iter().enumerate() {
                let near = a.s > 40.0 && a.s < 130.0;
                if near {
                    slowest[a.id.0 as usize - 1] = slowest[a.id.0 as usize - 1].min(a.speed);
                }
                let g = &w.graph().lane(a.lane).unwrap().geometry;
                pos.push((k, g.point_at(a.s)));
            }
            if pos.len() == 2 {
                closest = closest.min(pos[0].1.distance(pos[1].1));
            }
        }
        (slowest, closest)
    }

    #[test]
    fn right_before_left_the_vehicle_from_the_right_goes_first() {
        let mut w = crossing(LaneRules::default(), LaneRules::default());
        // Both reach the crossing together; the north-bound one has the other on its right.
        w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
            .unwrap();
        w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
            .unwrap();
        let (slowest, closest) = run_crossing(&mut w, 150);
        assert!(
            slowest[1] > 10.0,
            "the vehicle from the right slowed to {}",
            slowest[1]
        );
        assert!(
            slowest[0] < 6.0,
            "the vehicle from the left did not give way ({})",
            slowest[0]
        );
        assert!(closest > 1.4, "bodies came within {closest} m");
    }

    #[test]
    fn a_priority_road_goes_first_even_when_the_side_road_arrives_earlier() {
        let high = LaneRules {
            priority: Some(192),
            ..LaneRules::default()
        };
        let low = LaneRules {
            priority: Some(64),
            ..LaneRules::default()
        };
        // The west-bound vehicle is on the left-hand priority... flip: lane 0 is the priority road
        // although lane 1 comes from its right, and lane 1 is 10 m closer to the crossing.
        let mut w = crossing(high, low);
        w.spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
            .unwrap();
        w.spawn(LaneId(1), 60.0, 12.0, IdmParams::default())
            .unwrap();
        let (slowest, closest) = run_crossing(&mut w, 150);
        assert!(
            slowest[0] > 10.0,
            "the priority road was held up (slowest {})",
            slowest[0]
        );
        assert!(
            slowest[1] < 6.0,
            "the side road did not give way ({})",
            slowest[1]
        );
        assert!(closest > 1.4, "bodies came within {closest} m");
    }

    #[test]
    fn vehicles_without_right_of_way_do_not_wait_for_far_away_traffic() {
        let mut w = crossing(LaneRules::default(), LaneRules::default());
        // The vehicle from the right is 150 m away (12 s): the north-bound one just drives on.
        w.spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
            .unwrap();
        w.spawn(LaneId(1), 0.0, 12.0, IdmParams::default()).unwrap();
        let (slowest, _) = run_crossing(&mut w, 80);
        assert!(slowest[0] > 10.0, "slowed to {}", slowest[0]);
    }

    #[test]
    fn a_vehicle_waiting_at_red_does_not_hold_up_crossing_traffic() {
        use std::f64::consts::FRAC_PI_2;
        // East-bound approach lane, then the signalled lane across the north-bound road at x = 0.
        let approach =
            LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 97.0, LaneKind::Road);
        let mut gated =
            LaneSegment::straight(DVec2::new(-3.0, 100.0), FRAC_PI_2, 106.0, LaneKind::Road);
        gated.signal = Some((7, 0));
        let mut w = world(&[straight(0.0, 200.0), approach, gated]);
        w.set_signals(HashMap::from([(
            (7, 0),
            SignalProgram::new(vec![(0, 60.0), (6, 60.0)], 0.0).unwrap(),
        )]));
        w.set_clock(0.0);
        w.spawn(LaneId(1), 95.0, 0.0, IdmParams::default()).unwrap();
        let north = w
            .spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
            .unwrap();
        let mut slowest = f64::INFINITY;
        for _ in 0..80 {
            w.step(0.1);
            if let Some(a) = w.agent(north) {
                if a.s < 110.0 {
                    slowest = slowest.min(a.speed);
                }
            }
        }
        assert!(slowest > 9.0, "the crossing car slowed to {slowest} m/s");
    }

    /// A straight 100 m, a right-hand quarter circle of radius `r` and another straight.
    fn bend(r: f64) -> TrafficWorld {
        use std::f64::consts::FRAC_PI_2;
        let arc = LaneSegment {
            geometry: PathGeometry {
                start: DVec2::new(0.0, 100.0),
                heading: 0.0,
                radius: r,
                length: r * FRAC_PI_2,
            },
            ..straight(100.0, 0.0)
        };
        let after =
            LaneSegment::straight(DVec2::new(r, 100.0 + r), FRAC_PI_2, 200.0, LaneKind::Road);
        world(&[straight(0.0, 100.0), arc, after])
    }

    #[test]
    fn in_a_tight_bend_the_axles_track_the_lane_and_the_speed_suits_the_radius() {
        let r = 12.0;
        let mut w = bend(r);
        let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
        let centre = DVec2::new(r, 100.0);
        let (l, v_c) = (IdmParams::default().wheelbase, (2.5 * r).sqrt());
        let (mut braked_before, mut on_arc) = (false, 0);
        for _ in 0..300 {
            w.step(0.1);
            let Some(a) = w.agent(id) else { break };
            if a.lane == LaneId(0) && a.accel < -0.3 {
                braked_before = true;
            }
            if a.lane == LaneId(1) && a.s > a.params.front_overhang + l {
                on_arc += 1;
                assert!(a.speed <= v_c + 0.3, "{} m/s on the arc", a.speed);
                // Front axle on the lane; the rear axle off-tracks inwards by at most
                // R − √(R² − L²) (≈ 0.29 m) and never outwards.
                let front = a.front_axle.distance(centre);
                let rear = a.rear_axle.distance(centre);
                assert!(
                    (front - r).abs() < 0.1,
                    "front axle {front} from the centre"
                );
                assert!(
                    rear <= r + 0.1 && rear >= (r * r - l * l).sqrt() - 0.1,
                    "rear axle {rear} from the centre"
                );
                assert!(a.steer > 0.05, "steers right: {}", a.steer);
            }
        }
        assert!(braked_before, "braking starts before the arc");
        assert!(on_arc > 5);
    }

    #[test]
    fn a_kinked_lane_join_turns_the_body_smoothly() {
        // A 12° kink between two lanes (within the join tolerance): the body's heading must not
        // jump at the join.
        let kink = 12f64.to_radians();
        let next = LaneSegment::straight(DVec2::new(0.0, 100.0), kink, 100.0, LaneKind::Road);
        let mut w = world(&[straight(0.0, 100.0), next]);
        // 25 km/h, a junction speed: the body turns over about a wheelbase of travel.
        let slow = IdmParams {
            desired_speed: 7.0,
            ..IdmParams::default()
        };
        let id = w.spawn(LaneId(0), 70.0, 7.0, slow).unwrap();
        let mut h = w.agent(id).unwrap().heading();
        let mut worst = 0.0f64;
        for _ in 0..80 {
            w.step(0.1);
            let Some(a) = w.agent(id) else { break };
            worst = worst.max((a.heading() - h).abs());
            h = a.heading();
        }
        assert!(
            (h - kink).abs() < 0.01,
            "ends on the new lane's heading: {h}"
        );
        assert!(
            worst < 2f64.to_radians(),
            "heading jumped {}°",
            worst.to_degrees()
        );
    }

    #[test]
    fn a_vehicle_brakes_before_a_lower_limit_and_keeps_it() {
        let mut slow = straight(100.0, 200.0);
        slow.speed_limit = Some(30.0 / 3.6);
        let mut w = world(&[straight(0.0, 100.0), slow]);
        let id = w
            .spawn(LaneId(0), 50.0, 50.0 / 3.6, IdmParams::default())
            .unwrap();
        for _ in 0..200 {
            w.step(0.1);
            let a = w.agent(id).unwrap();
            if a.lane == LaneId(1) {
                assert!(a.speed <= 30.0 / 3.6 + 0.3, "{} km/h", a.speed * 3.6);
            }
        }
        assert_eq!(w.agent(id).unwrap().lane, LaneId(1));
    }

    #[test]
    fn the_speed_cap_is_not_carried_over_from_the_spawn_lane() {
        let mut slow = straight(0.0, 100.0);
        slow.speed_limit = Some(30.0 / 3.6);
        let mut fast = straight(100.0, 600.0);
        fast.speed_limit = Some(70.0 / 3.6);
        let mut w = world(&[slow, fast]);
        let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
        for _ in 0..400 {
            w.step(0.1);
        }
        let a = w.agent(id).unwrap();
        assert!(
            a.speed > 60.0 / 3.6,
            "{} km/h on a 70 km/h lane",
            a.speed * 3.6
        );
    }

    #[test]
    fn a_car_changes_into_a_trucks_lane() {
        // Changed reading (C3): `trucks` marks a lane trucks may use; cars use it too.
        let mut right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
        right.rules.trucks = true;
        let mut w = world(&[straight(0.0, 800.0), right]);
        let car = w
            .spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
            .unwrap();
        let mut events = Vec::new();
        for _ in 0..300 {
            w.step(0.1);
            events.extend(w.drain_events());
        }
        assert!(events.contains(&TrafficEvent::LaneChanged {
            id: car,
            side: Side::Right
        }));
        assert_eq!(w.agent(car).unwrap().lane, LaneId(1));
    }

    #[test]
    fn a_lane_change_is_possible_after_a_passed_crossing() {
        use std::f64::consts::FRAC_PI_2;
        // Both lanes are crossed at y = 50; the car is 40 m past it behind a slow one.
        let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
        let cross =
            LaneSegment::straight(DVec2::new(-50.0, 50.0), FRAC_PI_2, 100.0, LaneKind::Road);
        let mut w = world(&[straight(0.0, 800.0), right, cross]);
        w.spawn(LaneId(1), 170.0, 3.0, slow()).unwrap();
        let fast = w
            .spawn(LaneId(1), 90.0 + 4.4, 12.0, IdmParams::default())
            .unwrap();
        assert!(!w.graph().conflicts(LaneId(1)).is_empty());
        let mut changed = false;
        for _ in 0..150 {
            w.step(0.1);
            changed |= w
                .drain_events()
                .iter()
                .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == fast));
        }
        assert!(changed, "the passed crossing still blocked the change");
    }

    #[test]
    fn a_strategic_change_is_made_before_the_lane_ends() {
        use std::f64::consts::FRAC_PI_2;
        // The left lane ends (no successor); the right one continues into a right turn. The
        // route goes through the neighbour, so the car must move over in time.
        let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 200.0, LaneKind::Road);
        let turn = LaneSegment {
            geometry: PathGeometry {
                start: DVec2::new(3.5, 200.0),
                heading: 0.0,
                radius: 15.0,
                length: 15.0 * FRAC_PI_2,
            },
            ..straight(0.0, 0.0)
        };
        let exit = LaneSegment::straight(DVec2::new(18.5, 215.0), FRAC_PI_2, 300.0, LaneKind::Road);
        let mut w = world(&[straight(0.0, 200.0), right, turn, exit]);
        let id = w
            .spawn(LaneId(0), 60.0, 10.0, IdmParams::default())
            .unwrap();
        let s = w
            .agent(id)
            .unwrap()
            .strategic
            .expect("planned through the neighbour");
        assert_eq!((s.from, s.via, s.side), (LaneId(0), LaneId(1), Side::Right));
        let mut events = Vec::new();
        for _ in 0..300 {
            w.step(0.1);
            events.extend(w.drain_events());
        }
        assert!(events.contains(&TrafficEvent::LaneChanged {
            id,
            side: Side::Right
        }));
        assert!(!events
            .iter()
            .any(|e| matches!(e, TrafficEvent::ReachedDeadEnd { .. })));
        assert_eq!(w.agent(id).unwrap().lane, LaneId(3), "took the turn");
    }

    #[test]
    fn same_seed_same_run() {
        let run = || {
            let mut w = world(&[
                straight(0.0, 50.0),
                straight(50.0, 50.0),
                straight(50.0, 80.0),
            ]);
            let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
            for _ in 0..100 {
                w.step(0.1);
            }
            w.agent(id).map(|a| (a.lane, a.s.to_bits()))
        };
        assert_eq!(run(), run());
    }
}
