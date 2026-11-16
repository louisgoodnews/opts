//! Shortest-path routing on the lane graph: Dijkstra over lane lengths.

use super::*;

impl LaneGraph {
    /// Shortest route (by length) from `from` to `to`, both included.
    #[must_use]
    pub fn shortest_route(&self, from: LaneId, to: LaneId) -> Option<Vec<LaneId>> {
        #[derive(PartialEq)]
        struct Item(f64, u32);
        impl Eq for Item {}
        impl Ord for Item {
            fn cmp(&self, o: &Self) -> Ordering {
                o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
            }
        }
        impl PartialOrd for Item {
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        self.lane(from)?;
        self.lane(to)?;
        let mut dist = vec![f64::INFINITY; self.lanes.len()];
        let mut prev = vec![u32::MAX; self.lanes.len()];
        let mut heap = BinaryHeap::new();
        dist[from.0 as usize] = self.lanes[from.0 as usize].length();
        heap.push(Item(dist[from.0 as usize], from.0));
        while let Some(Item(d, u)) = heap.pop() {
            if u == to.0 {
                let mut route = vec![to];
                let mut cur = to.0;
                while prev[cur as usize] != u32::MAX {
                    cur = prev[cur as usize];
                    route.push(LaneId(cur));
                }
                route.reverse();
                return Some(route);
            }
            if d > dist[u as usize] {
                continue;
            }
            for &LaneId(v) in &self.successors[u as usize] {
                let nd = d + self.lanes[v as usize].length();
                if nd < dist[v as usize] {
                    dist[v as usize] = nd;
                    prev[v as usize] = u;
                    heap.push(Item(nd, v));
                }
            }
        }
        None
    }
}
