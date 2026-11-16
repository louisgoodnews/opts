//! One terrain triangle cut by the strips: the parts inside a strip follow it, the rest keep the
//! ground's plane.

use super::clip::{area, split};
use super::*;

/// One terrain triangle that strips cross: the parts outside the strip triangles keep the ground's
/// plane, the parts inside follow the strips. A triangle no strip really touches is kept whole.
pub(super) fn cut_triangle(
    b: &mut Builder,
    cut: &TerrainCut,
    (tri, ground): ([usize; 3], [Vec3; 3]),
    candidates: &[u32],
) {
    let mut outside = vec![ground.map(Vec3::truncate).to_vec()];
    let mut inside: Vec<(Vec<Vec2>, u32)> = Vec::new();
    for &c in candidates {
        let t = cut.strips.tris[c as usize];
        let clip = t.map(Vec3::truncate);
        let mut rest = Vec::new();
        for poly in outside {
            let within = clip_to_triangle(&poly, clip);
            if area(&within) < 1e-5 {
                rest.push(poly);
                continue;
            }
            inside.push((within, c));
            // The parts outside the strip triangle, edge by edge.
            let mut remain = poly;
            for e in 0..3 {
                let (r, l) = split(&remain, clip[e], clip[(e + 1) % 3]);
                if area(&r) >= 1e-5 {
                    rest.push(r);
                }
                remain = l;
            }
        }
        outside = rest;
    }
    if inside.is_empty() {
        b.indices
            .extend(tri.iter().map(|&k| u32::try_from(k).unwrap_or(0)));
        return;
    }
    for poly in &outside {
        // A sliver left along the seam of two strip triangles lies inside them.
        let centre = poly.iter().copied().sum::<Vec2>() / poly.len() as f32;
        if cut.height(centre.x, centre.y).is_some() {
            continue;
        }
        b.polygon(poly, |q| plane_height(&ground, q));
    }
    for (poly, c) in &inside {
        let t = cut.strips.tris[*c as usize];
        b.polygon(poly, |q| cut.capped(plane_height(&t, q), q.x, q.y));
    }
}
