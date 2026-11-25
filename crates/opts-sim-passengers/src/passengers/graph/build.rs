//! Building the cabin graph from `paths.cfg`, and joining the gaps between its parts.

use super::super::spec::Paths;
use super::{CabinGraph, GAP_BRIDGE_M};
use glam::Vec3;

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
        let mut graph = Self {
            points,
            links,
            bridges: Vec::new(),
            unit_of,
            adjacent,
        };
        graph.bridge_gaps();
        graph
    }

    /// Joins the parts of the paths that no link connects where two of their points in the same
    /// unit lie at most `GAP_BRIDGE_M` apart, the nearest pair first, until none is left.
    fn bridge_gaps(&mut self) {
        loop {
            let part = self.parts();
            let mut best: Option<(usize, usize, f32)> = None;
            for a in 0..self.points.len() {
                for b in a + 1..self.points.len() {
                    if part[a] == part[b] || self.unit(a) != self.unit(b) {
                        continue;
                    }
                    let d = self.points[a].distance(self.points[b]);
                    if d <= GAP_BRIDGE_M && best.is_none_or(|(_, _, e)| d < e) {
                        best = Some((a, b, d));
                    }
                }
            }
            let Some((a, b, d)) = best else {
                return;
            };
            self.adjacent[a].push((b, d));
            self.adjacent[b].push((a, d));
            self.bridges.push((a, b));
        }
    }

    /// The part (connected component, links taken both ways) of every pathpoint, named by its
    /// lowest point.
    fn parts(&self) -> Vec<usize> {
        let n = self.points.len();
        let mut part = vec![usize::MAX; n];
        let mut undirected = vec![Vec::new(); n];
        for (u, edges) in self.adjacent.iter().enumerate() {
            for &(v, _) in edges {
                undirected[u].push(v);
                undirected[v].push(u);
            }
        }
        for s in 0..n {
            if part[s] != usize::MAX {
                continue;
            }
            let mut todo = vec![s];
            while let Some(u) = todo.pop() {
                if part[u] != usize::MAX {
                    continue;
                }
                part[u] = s;
                todo.extend(undirected[u].iter().copied());
            }
        }
        part
    }
}
