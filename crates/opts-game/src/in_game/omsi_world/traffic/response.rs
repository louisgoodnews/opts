//! Roaming emergency services: an incident that needs police, an ambulance or the fire brigade
//! sends the nearest free service vehicle on the road to it.
//!
//! The responders are ordinary AI vehicles of the map's service groups (`Polizei`, `RTW`,
//! `Feuerwehr`); the density rules spawn them like any other group. An incident is queued with
//! [`IncidentQueue::report`] (the collision response of ISS-044 reports them).
//! [`dispatch_responders`] gives the nearest free vehicle of the right service a route to the
//! incident and a stop there,
//! which the engine drives as it drives a scheduled bus to a stop. The stop has the station
//! [`INCIDENT_STATION`]; [`IncidentQueue::on_event`] logs the arrival and frees the vehicle.

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use opts_traffic::{AgentId, LaneId, StopPoint};

use super::{ScheduledTraffic, TrafficPool, TrafficState};

mod events;
mod kind;
pub use kind::{service_kind, ServiceKind, MAX_RESPONSE_ROUTE_M};
use kind::nearest_responder;

/// The station of the stop at an incident; no timetable station has this id.
pub const INCIDENT_STATION: u32 = u32::MAX;

// TODO(owner): seconds a responder stays at the incident before it drives on.
pub const DEFAULT_ON_SCENE_S: f64 = 120.0;

// TODO(owner): seconds an incident waits for a free responder before it is dropped.
pub const MAX_WAIT_S: f32 = 60.0;

/// An incident that needs a service.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Incident {
    /// The reporter's id of the incident.
    pub id: u64,
    /// The lane the incident is on.
    pub lane: LaneId,
    /// Position on the lane (m).
    pub s: f64,
    /// The service it needs.
    pub kind: ServiceKind,
    /// Seconds the responder stays at the incident.
    pub on_scene_s: f64,
}

impl Incident {
    /// An incident with the default time on scene ([`DEFAULT_ON_SCENE_S`]).
    #[must_use]
    pub fn new(id: u64, lane: LaneId, s: f64, kind: ServiceKind) -> Self {
        Self {
            id,
            lane,
            s,
            kind,
            on_scene_s: DEFAULT_ON_SCENE_S,
        }
    }
}

/// An incident waiting for a responder, with the seconds it has waited.
struct Queued {
    incident: Incident,
    waited_s: f32,
}

/// The incidents waiting for a responder and the responders on their way.
#[derive(Resource, Default)]
pub struct IncidentQueue {
    pending: VecDeque<Queued>,
    /// The incident each dispatched vehicle drives to.
    responding: HashMap<AgentId, Incident>,
}

impl IncidentQueue {
    /// Queues an incident; the next [`dispatch_responders`] pass sends a vehicle to it.
    pub fn report(&mut self, incident: Incident) {
        self.pending.push_back(Queued {
            incident,
            waited_s: 0.0,
        });
    }
}

/// Sends the nearest free vehicle of the right service to each queued incident. Incidents without
/// a responder wait; after [`MAX_WAIT_S`] they are dropped.
pub fn dispatch_responders(
    time: Res<Time>,
    mut queue: ResMut<IncidentQueue>,
    mut state: ResMut<TrafficState>,
    pool: Res<TrafficPool>,
    sched: Res<ScheduledTraffic>,
) {
    let IncidentQueue { pending, responding } = &mut *queue;
    responding.retain(|id, _| state.world.agent(*id).is_some());
    let dt = time.delta_secs();
    let mut waiting = VecDeque::new();
    while let Some(mut q) = pending.pop_front() {
        q.waited_s += dt;
        if send_responder(&mut state, &pool, &sched, responding, q.incident) {
            continue;
        }
        if q.waited_s > MAX_WAIT_S {
            tracing::info!(
                incident = q.incident.id,
                kind = ?q.incident.kind,
                "incident dropped: no responder"
            );
        } else {
            waiting.push_back(q);
        }
    }
    *pending = waiting;
}

/// Sends the nearest free vehicle of the incident's service to it; `false` when none can go.
fn send_responder(
    state: &mut TrafficState,
    pool: &TrafficPool,
    sched: &ScheduledTraffic,
    responding: &mut HashMap<AgentId, Incident>,
    incident: Incident,
) -> bool {
    let candidates: Vec<(AgentId, LaneId)> = pool
        .agent_groups
        .iter()
        .filter(|&(id, g)| {
            !responding.contains_key(id)
                && sched.run_of(*id).is_none()
                && pool.groups.get(*g).and_then(|p| service_kind(&p.name)) == Some(incident.kind)
        })
        .filter_map(|(&id, _)| state.world.agent(id).map(|a| (id, a.lane)))
        .collect();
    let Some((id, route)) = nearest_responder(state.world.graph(), &candidates, incident.lane)
    else {
        return false;
    };
    let lanes = route.len();
    let stop = StopPoint::new(
        INCIDENT_STATION,
        incident.lane,
        incident.s,
        incident.on_scene_s,
        0.0,
    );
    if !(state.world.set_route(id, route) && state.world.push_stop(id, stop)) {
        return false;
    }
    tracing::info!(incident = incident.id, kind = ?incident.kind, "incident responded to");
    tracing::debug!(
        vehicle = id.0,
        incident = incident.id,
        route_lanes = lanes,
        lane = incident.lane.0,
        s = incident.s,
        "responder dispatched"
    );
    responding.insert(id, incident);
    true
}
