//! Traffic world container, speed limits, graph replacement, and agent spawning.

use super::*;

/// Lane graph plus the agents driving on it.
#[derive(Debug, Clone)]
pub struct TrafficWorld {
    pub(crate) graph: LaneGraph,
    pub(crate) agents: Vec<Agent>,
    next_id: u64,
    pub(crate) rng: Rng,
    pub(crate) events: Vec<TrafficEvent>,
    /// Simulation time in seconds since the world was created.
    pub(crate) time: f64,
    /// Clock time (seconds of the day) at simulation time zero.
    clock_base: f64,
    pub(super) signals: HashMap<SignalKey, SignalProgram>,
    /// Speed limit (m/s) assumed on lanes without one.
    open_road_speed: f64,
    /// Vehicles outside the traffic ([`crate::obstacles`]).
    pub(crate) obstacles: Vec<crate::obstacles::Obstacle>,
    /// Phantom agents of the last step (diagnostics).
    pub(crate) last_phantoms: usize,
    /// Seconds standing without a light ahead after which an agent counts as stuck.
    pub(super) stuck_after_s: f64,
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
