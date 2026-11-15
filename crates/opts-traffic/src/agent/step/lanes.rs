//! Moving agents over the end of their lane onto the next one, or off the network at a dead end.

use super::*;

impl TrafficWorld {
    /// Moves agents that passed the end of their lane onto the next one.
    pub(super) fn advance_lanes(&mut self) {
        let mut i = 0;
        while i < self.agents.len() {
            loop {
                let a = &self.agents[i];
                if a.foreign {
                    break;
                }
                let Some(len) = self.graph.lane(a.lane).map(|l| l.length()) else {
                    break;
                };
                if a.s <= len {
                    break;
                }
                // It may not leave a lane it still has to change away from.
                if a.strategic.is_some_and(|c| c.from == a.lane) {
                    let a = &mut self.agents[i];
                    a.s = len;
                    a.speed = 0.0;
                    break;
                }
                let (over, id, lane) = (a.s - len, a.id, a.lane);
                match self.agents[i].plan.pop_front() {
                    Some(n) => {
                        let a = &mut self.agents[i];
                        a.prev_lane = Some(a.lane);
                        a.lane = n;
                        a.s = over;
                        self.extend_plan(i);
                    }
                    None => {
                        self.events.push(TrafficEvent::ReachedDeadEnd { id, lane });
                        self.agents.remove(i);
                        i = i.wrapping_sub(1);
                        break;
                    }
                }
            }
            i = i.wrapping_add(1);
        }
    }
}
