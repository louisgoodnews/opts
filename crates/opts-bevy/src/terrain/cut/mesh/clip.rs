//! Polygon clipping of the terrain's triangles against the strips: the split by a line and the
//! area of a polygon.

use super::*;

/// The part of convex `poly` on the right of the directed line `p → q` (outside a
/// counter-clockwise triangle edge), and the part on its left.
pub(super) fn split(poly: &[Vec2], p: Vec2, q: Vec2) -> (Vec<Vec2>, Vec<Vec2>) {
    let (mut right, mut left) = (Vec::new(), Vec::new());
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        let (sa, sb) = (cross2(p, q, a), cross2(p, q, b));
        if sa >= 0.0 {
            left.push(a);
        } else {
            right.push(a);
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            let x = a + (b - a) * (sa / (sa - sb));
            left.push(x);
            right.push(x);
        }
    }
    (right, left)
}

/// Area of a polygon (counter-clockwise positive).
pub(super) fn area(poly: &[Vec2]) -> f32 {
    (1..poly.len().saturating_sub(1))
        .map(|k| cross2(poly[0], poly[k], poly[k + 1]))
        .sum::<f32>()
        * 0.5
}
