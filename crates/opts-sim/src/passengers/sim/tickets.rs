//! ticket draws and the choice of a place for a passenger.

use super::*;

impl PassengerSim {
    pub(super) fn nearest_exit(&self, from: usize) -> usize {
        (0..self.exit_nodes.len())
            .filter_map(|e| {
                self.graph
                    .route(from, self.exit_nodes[e])
                    .map(|(_, len)| (e, len))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |(e, _)| e)
    }

    /// The tickets a passenger of `age` may buy for `stops` stops, one drawn by weight.
    pub(super) fn draw_ticket(&mut self, age: u32, stops: u32) -> Option<usize> {
        let eligible: Vec<(usize, f32)> = self
            .tickets
            .eligible(age, stops)
            .map(|(i, t)| (i, t.probability.max(0.0)))
            .collect();
        let total: f32 = eligible.iter().map(|e| e.1).sum();
        if eligible.is_empty() || total <= 0.0 {
            return None;
        }
        let mut x = self.rng.next_f32() * total;
        for (i, w) in &eligible {
            if x < *w {
                return Some(*i);
            }
            x -= w;
        }
        eligible.last().map(|e| e.0)
    }

    /// A free place for a passenger at `from`: a seat with the given chance and when there is
    /// one, else a standing place; drawn among the free ones that can be walked to, a place
    /// [`PLACE_SPREAD_M`] farther away half as likely, so that people fill the whole vehicle (the
    /// unit behind the joint too) and not only the places next to the door.
    pub(super) fn pick_place(&mut self, from: usize) -> Option<usize> {
        let taken: Vec<usize> = self.passengers.iter().filter_map(|p| p.place).collect();
        let dist = self.graph.distances(from);
        let want_seat = self.rng.next_f32() < SIT_CHANCE;
        let roll = self.rng.next_f32();
        [want_seat, !want_seat].into_iter().find_map(|seats| {
            let free: Vec<(usize, f32)> = (0..self.cabin.places.len())
                .filter(|i| !taken.contains(i) && self.cabin.places[*i].is_seat() == seats)
                .filter_map(|i| {
                    let d = dist.get(self.place_node[i]).copied()?;
                    d.is_finite().then(|| (i, 1.0 / (1.0 + d / PLACE_SPREAD_M)))
                })
                .collect();
            let mut x = roll * free.iter().map(|f| f.1).sum::<f32>();
            free.iter()
                .find(|(_, w)| {
                    x -= w;
                    x < 0.0
                })
                .or(free.last())
                .map(|f| f.0)
        })
    }
}
