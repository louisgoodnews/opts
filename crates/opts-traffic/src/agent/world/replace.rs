//! Replacing the lane graph while agents drive on it: their lanes, plans and axles are remapped.

use super::*;

impl TrafficWorld {
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
}
