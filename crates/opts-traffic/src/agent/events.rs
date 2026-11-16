//! The events a traffic world reports to its host after a step.

use super::AgentId;
use crate::graph::LaneId;
use crate::neighbours::Side;

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
    /// [`crate::agent::LANE_CHANGE_DURATION_S`] seconds).
    LaneChanged {
        /// The agent.
        id: AgentId,
        /// Side it moved to.
        side: Side,
    },
    /// A scheduled agent stopped at its stop point (see [`crate::agent::StopPoint`]).
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
    /// The agent stood still for [`crate::agent::STUCK_AFTER_S`] seconds without a red light or a lane change
    /// to wait for, and was removed (forced despawn) — the first agent to get stuck goes.
    Stuck {
        /// The removed agent.
        id: AgentId,
        /// The lane it stood on.
        lane: LaneId,
    },
}
