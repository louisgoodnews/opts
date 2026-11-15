//! Shortest walks on the cabin graph: Dijkstra from a pathpoint and the route to another.

use super::CabinGraph;

impl CabinGraph {
    /// Dijkstra from `from`, stopping once `to` is settled: distances and predecessors.
    pub(super) fn search(&self, from: usize, to: Option<usize>) -> (Vec<f32>, Vec<usize>) {
        let n = self.points.len();
        let mut dist = vec![f32::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        if from >= n {
            return (dist, prev);
        }
        dist[from] = 0.0;
        for _ in 0..n {
            let Some(u) = (0..n)
                .filter(|&i| !done[i] && dist[i].is_finite())
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))
            else {
                break;
            };
            if Some(u) == to {
                break;
            }
            done[u] = true;
            for &(v, w) in &self.adjacent[u] {
                if dist[u] + w < dist[v] {
                    dist[v] = dist[u] + w;
                    prev[v] = u;
                }
            }
        }
        (dist, prev)
    }

    /// Shortest way from `from` to `to` as the pathpoints after `from` (empty when there is none or
    /// when they are the same point), with its length.
    #[must_use]
    pub fn route(&self, from: usize, to: usize) -> Option<(Vec<usize>, f32)> {
        let n = self.points.len();
        if from >= n || to >= n {
            return None;
        }
        if from == to {
            return Some((Vec::new(), 0.0));
        }
        let (dist, prev) = self.search(from, Some(to));
        if !dist[to].is_finite() {
            return None;
        }
        let mut path = vec![to];
        let mut at = to;
        while prev[at] != usize::MAX && prev[at] != from {
            at = prev[at];
            path.push(at);
        }
        path.reverse();
        Some((path, dist[to]))
    }
}
