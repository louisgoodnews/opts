//! The path graph of a cabin: shortest ways between pathpoints.

use super::spec::Paths;
use glam::Vec3;

/// Walkable graph of one vehicle unit (links are walkable both ways).
#[derive(Debug, Clone, Default)]
pub struct CabinGraph {
    /// Position of every pathpoint (segment space: x right, y forward, z up).
    pub points: Vec<Vec3>,
    /// `(from, to)` of every link in file order (the index of `GetHumanCountOnPathLink`).
    pub links: Vec<(usize, usize)>,
    adjacent: Vec<Vec<(usize, f32)>>,
}

impl CabinGraph {
    /// Builds the graph of a `paths.cfg`; links to pathpoints that do not exist are dropped.
    #[must_use]
    pub fn new(paths: &Paths) -> Self {
        let points: Vec<Vec3> = paths.points.clone();
        let mut adjacent = vec![Vec::new(); points.len()];
        let mut links = Vec::new();
        for &(from, to) in &paths.links {
            if from < points.len() && to < points.len() && from != to {
                let len = points[from].distance(points[to]);
                adjacent[from].push((to, len));
                adjacent[to].push((from, len));
                links.push((from, to));
            }
        }
        Self {
            points,
            links,
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

    /// The index of the link between two neighbours.
    #[must_use]
    pub fn link_between(&self, a: usize, b: usize) -> Option<usize> {
        self.links
            .iter()
            .position(|&(f, t)| (f == a && t == b) || (f == b && t == a))
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
        let mut dist = vec![f32::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        dist[from] = 0.0;
        for _ in 0..n {
            let Some(u) = (0..n)
                .filter(|&i| !done[i] && dist[i].is_finite())
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))
            else {
                break;
            };
            if u == to {
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
