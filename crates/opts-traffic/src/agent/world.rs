//! Traffic world container, speed limits, graph replacement, and agent spawning.

use super::*;

mod access;
mod clock;
mod replace;
mod spawn;
mod speed;

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
}
