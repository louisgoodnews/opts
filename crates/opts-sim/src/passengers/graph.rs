//! The path graph of a cabin: shortest ways between pathpoints.

use super::spec::Paths;
use glam::Vec3;

/// Walkable graph of a vehicle (links are walkable both ways unless they are one-way).
#[derive(Debug, Clone, Default)]
pub struct CabinGraph {
    /// Position of every pathpoint (segment space: x right, y forward, z up).
    pub points: Vec<Vec3>,
    /// `(from, to)` of every link in file order (the index of `GetHumanCountOnPathLink`).
    pub links: Vec<(usize, usize)>,
    unit_of: Vec<usize>,
    adjacent: Vec<Vec<(usize, f32)>>,
}

impl CabinGraph {
    /// Builds the graph of a `paths.cfg`; links to pathpoints that do not exist are dropped.
    #[must_use]
    pub fn new(paths: &Paths) -> Self {
        let points: Vec<Vec3> = paths.points.clone();
        let mut adjacent = vec![Vec::new(); points.len()];
        let mut links = Vec::new();
        for (i, &(from, to)) in paths.links.iter().enumerate() {
            if from < points.len() && to < points.len() && from != to {
                // Joined units are in different spaces: the joint costs a short step.
                let len = if paths.unit_of.get(from) == paths.unit_of.get(to) {
                    points[from].distance(points[to])
                } else {
                    0.6
                };
                adjacent[from].push((to, len));
                if !paths.one_way.get(i).copied().unwrap_or(false) {
                    adjacent[to].push((from, len));
                }
                links.push((from, to));
            }
        }
        let mut unit_of = paths.unit_of.clone();
        unit_of.resize(points.len(), 0);
        Self {
            points,
            links,
            unit_of,
            adjacent,
        }
    }

    /// The pathpoint nearest to `p`.
    #[must_use]
    pub fn nearest(&self, p: Vec3) -> Option<usize> {
        self.points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.distance_squared(p).total_cmp(&b.distance_squared(p)))
            .map(|(i, _)| i)
    }

    /// The unit a pathpoint is in.
    #[must_use]
    pub fn unit(&self, node: usize) -> usize {
        self.unit_of.get(node).copied().unwrap_or(0)
    }

    /// The pathpoint of `unit` nearest to `p` (in that unit's space).
    #[must_use]
    pub fn nearest_in_unit(&self, p: Vec3, unit: usize) -> Option<usize> {
        (0..self.points.len())
            .filter(|&i| self.unit(i) == unit)
            .min_by(|&a, &b| {
                self.points[a]
                    .distance_squared(p)
                    .total_cmp(&self.points[b].distance_squared(p))
            })
    }

    /// Which pathpoints can be walked to from any of `starts`.
    #[must_use]
    pub fn reachable(&self, starts: &[usize]) -> Vec<bool> {
        let mut seen = vec![false; self.points.len()];
        let mut todo: Vec<usize> = starts.iter().copied().filter(|&s| s < seen.len()).collect();
        while let Some(u) = todo.pop() {
            if std::mem::replace(&mut seen[u], true) {
                continue;
            }
            todo.extend(self.adjacent[u].iter().map(|e| e.0).filter(|&v| !seen[v]));
        }
        seen
    }

    /// The pathpoint of `unit` nearest to `p` among those `reach` marks, else among all of the
    /// unit's: a cabin whose paths are cut somewhere (the Valiant 7700A's rear unit has no link
    /// between its points 9 and 10) still has its places reached from the nearest point that
    /// can be walked to.
    #[must_use]
    pub fn nearest_reachable(&self, p: Vec3, unit: usize, reach: &[bool]) -> Option<usize> {
        (0..self.points.len())
            .filter(|&i| self.unit(i) == unit && reach.get(i).copied().unwrap_or(false))
            .min_by(|&a, &b| {
                self.points[a]
                    .distance_squared(p)
                    .total_cmp(&self.points[b].distance_squared(p))
            })
            .or_else(|| self.nearest_in_unit(p, unit))
    }

    /// The index of the link between two neighbours.
    #[must_use]
    pub fn link_between(&self, a: usize, b: usize) -> Option<usize> {
        self.links
            .iter()
            .position(|&(f, t)| (f == a && t == b) || (f == b && t == a))
    }

    /// Walking distance from `from` to every pathpoint (`INFINITY` where there is no way).
    #[must_use]
    pub fn distances(&self, from: usize) -> Vec<f32> {
        self.search(from, None).0
    }

    /// Dijkstra from `from`, stopping once `to` is settled: distances and predecessors.
    fn search(&self, from: usize, to: Option<usize>) -> (Vec<f32>, Vec<usize>) {
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
