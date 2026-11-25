//! The responders' events at incidents: arrival, departure and a missed stop. They are logged and
//! the vehicle is freed when it leaves or misses the stop.

use opts_traffic::TrafficEvent;

use super::{IncidentQueue, INCIDENT_STATION};

impl IncidentQueue {
    /// Logs the responders' arrivals and departures at incidents, and frees them afterwards.
    pub fn on_event(&mut self, e: &TrafficEvent) {
        match *e {
            TrafficEvent::ArrivedAtStop {
                id,
                station: INCIDENT_STATION,
                clock,
            } => {
                if let Some(q) = self.responding.get(&id) {
                    tracing::debug!(
                        vehicle = id.0,
                        incident = q.id,
                        lane = q.lane.0,
                        clock,
                        "responder arrived"
                    );
                }
            }
            TrafficEvent::DepartedStop {
                id,
                station: INCIDENT_STATION,
                clock,
            } => {
                if let Some(q) = self.responding.remove(&id) {
                    tracing::debug!(vehicle = id.0, incident = q.id, clock, "responder left");
                }
            }
            TrafficEvent::StopMissed {
                id,
                station: INCIDENT_STATION,
            } => {
                if let Some(q) = self.responding.remove(&id) {
                    tracing::debug!(vehicle = id.0, incident = q.id, "responder missed the stop");
                }
            }
            _ => {}
        }
    }
}
