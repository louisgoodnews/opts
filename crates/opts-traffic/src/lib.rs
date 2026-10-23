//! # `opts-traffic` — AI road traffic for OPTS
//!
//! Pure Rust and deterministic, like `opts-sim`: no Bevy, no I/O. The crate turns the `[path]`
//! lanes of OMSI splines and junction objects into a directed [`LaneGraph`], moves
//! [`Agent`]s along it with the Intelligent Driver Model ([`idm`]) and reports the pedal and
//! indicator commands a vehicle script expects ([`DriverCommand`]).
//!
//! Design, data flow and open questions: `docs/src/architecture/ai-traffic.md`.
//!
//! ```
//! use opts_traffic::{ConnectParams, LaneGraph, LaneKind, LaneSegment, TrafficWorld};
//! use glam::DVec2;
//!
//! // Two 100 m straight segments heading north, end to end.
//! let seg = |y: f64| LaneSegment::straight(DVec2::new(0.0, y), 0.0, 100.0, LaneKind::Road);
//! let graph = LaneGraph::build(&[seg(0.0), seg(100.0)], &ConnectParams::default());
//! let mut world = TrafficWorld::new(graph, 1);
//! let first = world.graph().lane_ids().next().unwrap();
//! let id = world.spawn(first, 0.0, 0.0, Default::default()).unwrap();
//! for _ in 0..100 {
//!     world.step(0.1);
//! }
//! assert!(world.agent(id).is_some_and(|a| a.speed > 5.0));
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod agent;
pub mod conflict;
pub mod density;
pub mod geometry;
pub mod graph;
pub mod idm;
pub mod lanechange;
pub mod neighbours;
pub mod obstacles;
pub mod rightofway;
pub mod rng;
pub mod signals;
pub mod source;
pub mod traits;

pub use agent::{
    Agent, AgentId, Blinker, DriverCommand, StopPoint, TrafficEvent, TrafficStats, TrafficWorld,
    HARD_BRAKE, LANE_CHANGE_DURATION_S,
};
pub use conflict::{Conflict, ConflictKind};
pub use density::{
    group_targets, weighted_pick, DensityCurve, LaneDensity, MAX_DENSITY_GROUPS, MEDIUM_DENSITY,
};
pub use geometry::PathGeometry;
pub use graph::{
    ConnectParams, Direction, Elevation, Lane, LaneGraph, LaneId, LaneKind, LaneRules, LaneSegment,
    SpawnError,
};
pub use idm::{IdmParams, VehicleClass};
pub use neighbours::{Neighbour, Side};
pub use obstacles::Obstacle;
pub use rightofway::{precedence, Precedence};
pub use rng::Rng;
pub use signals::{Aspect, SignalKey, SignalProgram};
pub use source::{segments_from_tile, signals_from_tile, tile_origin, PathSource};
pub use traits::{Calibration, VehicleKind, VehicleTraits};
