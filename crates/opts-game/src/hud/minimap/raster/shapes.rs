//! Filled triangles, rings and the vehicle arrow of the mini map.

use bevy::math::Vec2;

use super::canvas::Canvas;

/// Fills a triangle (anti-aliased by the signed distance to its edges).
pub(super) fn fill_triangle(canvas: &mut Canvas, p: [Vec2; 3], color: [u8; 4], grow: f32) {
    let size = canvas.size as f32;
    let min = p[0].min(p[1]).min(p[2]) - Vec2::splat(grow.abs() + 1.0);
    let max = p[0].max(p[1]).max(p[2]) + Vec2::splat(grow.abs() + 1.0);
    let area = (p[1] - p[0]).perp_dot(p[2] - p[0]);
    if area.abs() < 1e-3 {
        return;
    }
    let sign = area.signum();
    for y in min.y.floor().max(0.0) as i32..=max.y.ceil().min(size - 1.0) as i32 {
        for x in min.x.floor().max(0.0) as i32..=max.x.ceil().min(size - 1.0) as i32 {
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            // Signed distance to the nearest edge: positive inside.
            let mut inside = f32::MAX;
            for i in 0..3 {
                let (a, b) = (p[i], p[(i + 1) % 3]);
                let e = b - a;
                let d = sign * e.perp_dot(q - a) / e.length().max(1e-6);
                inside = inside.min(d);
            }
            canvas.blend(x, y, color, inside + grow + 0.5);
        }
    }
}

pub(super) fn draw_ring(canvas: &mut Canvas, c: Vec2, radius: f32, width: f32, color: [u8; 4]) {
    let r = radius + width / 2.0 + 1.0;
    let size = canvas.size as f32;
    for y in (c.y - r).floor().max(0.0) as i32..=(c.y + r).ceil().min(size - 1.0) as i32 {
        for x in (c.x - r).floor().max(0.0) as i32..=(c.x + r).ceil().min(size - 1.0) as i32 {
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let d = (q.distance(c) - radius).abs();
            canvas.blend(x, y, color, width / 2.0 + 0.5 - d);
        }
    }
}

/// The arrow of a vehicle: tip forward, two rear corners.
pub(super) fn arrow(centre: Vec2, screen_heading: f32, length: f32) -> [Vec2; 3] {
    let dir = Vec2::new(screen_heading.sin(), -screen_heading.cos());
    let side = Vec2::new(-dir.y, dir.x);
    [
        centre + dir * length,
        centre - dir * (length * 0.7) + side * (length * 0.75),
        centre - dir * (length * 0.7) - side * (length * 0.75),
    ]
}
