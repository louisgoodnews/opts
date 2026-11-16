//! Object-level ordering inside a tile (M-0114): when the player is slow, the objects nearest to
//! the player are placed first, so a stationary player sees a complete near field first.

/// Whether the object regime orders the objects of a tile: below half the blend.
#[must_use]
pub fn near_first_for(blend: f32) -> bool {
    blend < 0.5
}

/// The order in which the objects at `points` are placed: nearest to `focus` first (stable on
/// ties); the identity when there is no focus.
#[must_use]
pub fn build_order(points: &[(f32, f32)], focus: Option<(f32, f32)>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..points.len()).collect();
    if let Some(f) = focus {
        let d2 = |i: usize| {
            let (dx, dy) = (points[i].0 - f.0, points[i].1 - f.1);
            dx * dx + dy * dy
        };
        order.sort_by(|&a, &b| d2(a).total_cmp(&d2(b)));
    }
    order
}
