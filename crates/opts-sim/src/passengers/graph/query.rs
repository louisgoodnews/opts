//! Queries on the cabin graph: the nearest pathpoints, the units, reachability and links.

use super::CabinGraph;
use glam::Vec3;

impl CabinGraph {
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
}
