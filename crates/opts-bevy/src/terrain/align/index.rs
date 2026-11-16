//! The outer edges bucketed by position, for the nearest edge of a vertex.

use super::*;

/// Edges bucketed by position, each in every bucket its box expanded by the band touches.
pub(super) struct EdgeIndex {
    edges: Vec<(Vec3, Vec3)>,
    buckets: HashMap<(i64, i64), Vec<u32>>,
    size: f32,
}

impl EdgeIndex {
    pub(super) fn new(edges: Vec<(Vec3, Vec3)>, band: f32) -> Self {
        let size = band.max(0.5);
        let mut buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, (a, b)) in edges.iter().enumerate() {
            let lo = a.truncate().min(b.truncate()) - Vec2::splat(band);
            let hi = a.truncate().max(b.truncate()) + Vec2::splat(band);
            for y in (lo.y / size).floor() as i64..=(hi.y / size).floor() as i64 {
                for x in (lo.x / size).floor() as i64..=(hi.x / size).floor() as i64 {
                    buckets.entry((x, y)).or_default().push(i as u32);
                }
            }
        }
        Self {
            edges,
            buckets,
            size,
        }
    }

    /// Distance in the plane to the nearest edge and that edge's height there.
    pub(super) fn nearest(&self, p: Vec2) -> Option<(f32, f32)> {
        let cell = (
            (p.x / self.size).floor() as i64,
            (p.y / self.size).floor() as i64,
        );
        let mut best: Option<(f32, f32)> = None;
        for &i in self.buckets.get(&cell)? {
            let (a, b) = self.edges[i as usize];
            let (a2, b2) = (a.truncate(), b.truncate());
            let ab = b2 - a2;
            let len2 = ab.length_squared();
            let t = if len2 > 1e-10 {
                ((p - a2).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = p.distance(a2 + ab * t);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, a.z + (b.z - a.z) * t));
            }
        }
        best
    }
}
