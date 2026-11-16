//! spots where walkers appear, and the xorshift random step.

use super::*;

impl PedestrianNet {
    /// A random point on a lane at a horizontal distance of `min_m..max_m` from `centre`.
    pub fn random_spot(
        &self,
        centre: (f32, f32),
        min_m: f32,
        max_m: f32,
        rng: &mut u32,
    ) -> Option<(LaneId, f32)> {
        let near: Vec<(LaneId, f32)> = self
            .lanes
            .iter()
            .filter(|(_, l)| {
                let (p, _) = l.at(l.length() * 0.5);
                (p.x - centre.0).hypot(p.y - centre.1) <= max_m + l.length() * 0.5
            })
            .map(|(id, l)| (*id, l.length()))
            .collect();
        let total: f32 = near.iter().map(|(_, len)| *len).sum();
        if total <= 0.0 {
            return None;
        }
        let mut near = near;
        near.sort_by_key(|(id, _)| *id); // HashMap order must not leak into the result
        for _ in 0..24 {
            let mut pick = next_f32(rng) * total;
            for (id, len) in &near {
                if pick <= *len {
                    let s = (pick / len.max(1e-3)).clamp(0.0, 1.0) * len;
                    let (p, _) = self.lanes[id].at(s);
                    let d = (p.x - centre.0).hypot(p.y - centre.1);
                    if (min_m..=max_m).contains(&d) {
                        return Some((*id, s));
                    }
                    break;
                }
                pick -= len;
            }
        }
        None
    }
}

/// xorshift32 step returning a value in `[0, 1)`.
pub fn next_f32(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1u32 << 24) as f32
}
