//! Walking along the paths: the route, the steps along it and the arrival at a goal.

use super::super::*;

impl PassengerSim {
    pub(super) fn walk(&mut self, i: usize, to: usize, dt: f32) {
        if self.passengers[i].route.is_empty() {
            let (at, unit, position) = {
                let p = &self.passengers[i];
                (p.at, p.unit, p.position)
            };
            let way = if at == to {
                Some(Vec::new())
            } else {
                self.graph.route(at, to).map(|(r, _)| r)
            };
            match way {
                Some(mut r) => {
                    // From a place (or a walk cut short) back onto the paths first: straight to
                    // the next point of the way would cut through the seats.
                    let off_path = self.graph.unit(at) == unit
                        && self
                            .graph
                            .points
                            .get(at)
                            .is_some_and(|q| q.distance(position) > 0.01);
                    if off_path {
                        r.insert(0, at);
                    }
                    self.passengers[i].route = r;
                }
                None => {
                    // No way: the passenger arrives where they are.
                    self.passengers[i].at = to;
                }
            }
        }
        let mut budget = WALK_SPEED_M_S * dt;
        while budget > 0.0 {
            let p = &mut self.passengers[i];
            let Some(&next) = p.route.first() else {
                break;
            };
            let target = self.graph.points[next];
            // The joint between two units is a hop: the two points are in different spaces and in
            // the same place of the vehicle.
            if self.graph.unit(next) != p.unit {
                p.position = target;
                p.unit = self.graph.unit(next);
                p.at = next;
                p.route.remove(0);
                continue;
            }
            let d = p.position.distance(target);
            if d <= budget {
                budget -= d;
                p.position = target;
                p.at = next;
                p.route.remove(0);
            } else {
                p.position += (target - p.position) / d * budget;
                budget = 0.0;
            }
        }
        if self.passengers[i].route.is_empty() && self.passengers[i].at == to {
            self.arrived(i);
        }
    }
}
