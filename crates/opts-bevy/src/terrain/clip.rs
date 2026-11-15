//! Clipping a triangle to a triangle in the plane (the 2D helpers of the carving).

use super::*;

/// Twice the signed area of the 2D triangle `a b c`.
pub(super) fn cross2(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b - a).perp_dot(c - a)
}

/// The part of convex polygon `subject` inside the counter-clockwise triangle `clip`
/// (Sutherland-Hodgman).
pub(super) fn clip_to_triangle(subject: &[Vec2], clip: [Vec2; 3]) -> Vec<Vec2> {
    let mut poly = subject.to_vec();
    for e in 0..3 {
        if poly.is_empty() {
            break;
        }
        let (p, q) = (clip[e], clip[(e + 1) % 3]);
        let side = |x: Vec2| cross2(p, q, x);
        let mut next = Vec::with_capacity(poly.len() + 2);
        for k in 0..poly.len() {
            let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
            let (sa, sb) = (side(a), side(b));
            if sa >= 0.0 {
                next.push(a);
            }
            if (sa >= 0.0) != (sb >= 0.0) {
                next.push(a + (b - a) * (sa / (sa - sb)));
            }
        }
        poly = next;
    }
    poly
}
