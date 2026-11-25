//! Access to the world's graph, agents and events, and the removal of agents.

use super::*;

impl TrafficWorld {
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

    /// Sets (or with `None` clears) the acceleration of agent `id` for the next steps, in place
    /// of the car-following law (M-0103: a near vehicle driven by its own script).
    pub fn set_external_accel(&mut self, id: AgentId, acc: Option<f64>) -> bool {
        match self.agents.iter_mut().find(|a| a.id == id) {
            Some(a) => {
                a.external_accel = acc;
                true
            }
            None => false,
        }
    }

    /// Removes an agent (despawn outside the player's range, say).
    pub fn despawn(&mut self, id: AgentId) -> bool {
        let before = self.agents.len();
        self.agents.retain(|a| a.id != id);
        self.agents.len() != before
    }

    /// Takes the events raised since the last call.
    pub fn drain_events(&mut self) -> Vec<TrafficEvent> {
        std::mem::take(&mut self.events)
    }

    pub(crate) fn push_event(&mut self, e: TrafficEvent) {
        self.events.push(e);
    }
}
