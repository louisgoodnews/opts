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
pub mod density;
pub mod geometry;
pub mod graph;
pub mod idm;
pub mod rng;
pub mod source;

pub use agent::{Agent, AgentId, Blinker, DriverCommand, TrafficEvent, TrafficWorld};
pub use density::{weighted_pick, DensityCurve};
pub use geometry::PathGeometry;
pub use graph::{
    ConnectParams, Direction, Elevation, Lane, LaneGraph, LaneId, LaneKind, LaneRules, LaneSegment,
    SpawnError,
};
pub use idm::IdmParams;
pub use rng::Rng;
pub use source::{segments_from_tile, tile_origin, PathSource, TILE_SIZE_M};
