//! Which service a vehicle group provides, and the nearest responder for an incident.

use opts_traffic::{AgentId, LaneGraph, LaneId};

/// A service that roaming vehicles of the map provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    Police,
    Ambulance,
    Fire,
}

// TODO(owner): the group names of the service vehicles. `Emergency` (listed next to `Ambulance`
// in the map's AI lists) has no kind yet, so its vehicles roam by density but never respond.
const POLICE_GROUPS: &[&str] = &["Polizei", "Police"];
const AMBULANCE_GROUPS: &[&str] = &["RTW", "Ambulance"];
const FIRE_GROUPS: &[&str] = &["Feuerwehr"];

// TODO(owner): longest route (m) a responder drives to an incident.
pub const MAX_RESPONSE_ROUTE_M: f64 = 2000.0;

/// The service a group of this name provides (case-insensitive); `None` for any other group.
#[must_use]
pub fn service_kind(group: &str) -> Option<ServiceKind> {
    let named = |names: &[&str]| names.iter().any(|n| n.eq_ignore_ascii_case(group));
    if named(POLICE_GROUPS) {
        Some(ServiceKind::Police)
    } else if named(AMBULANCE_GROUPS) {
        Some(ServiceKind::Ambulance)
    } else if named(FIRE_GROUPS) {
        Some(ServiceKind::Fire)
    } else {
        None
    }
}

/// The lanes from `from` to `to` (both included) and their length (m), or `None` when there is no
/// route or it is longer than [`MAX_RESPONSE_ROUTE_M`].
#[must_use]
pub fn route_to(graph: &LaneGraph, from: LaneId, to: LaneId) -> Option<(Vec<LaneId>, f64)> {
    let route = graph.shortest_route(from, to)?;
    let len: f64 = route
        .iter()
        .filter_map(|&l| graph.lane(l))
        .map(|l| l.length())
        .sum();
    (len <= MAX_RESPONSE_ROUTE_M).then_some((route, len))
}

/// The candidate (agent, its lane) with the shortest route to `to`, and the lanes it drives after
/// its own lane.
#[must_use]
pub fn nearest_responder(
    graph: &LaneGraph,
    candidates: &[(AgentId, LaneId)],
    to: LaneId,
) -> Option<(AgentId, Vec<LaneId>)> {
    candidates
        .iter()
        .filter_map(|&(id, lane)| {
            let (route, len) = route_to(graph, lane, to)?;
            Some((len, id, route))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id, route)| (id, route.into_iter().skip(1).collect()))
}
