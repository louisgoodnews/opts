//! Projection of the map onto the buffer, and the anti-aliased line drawing.

use bevy::math::Vec2;

use super::canvas::Canvas;
use super::MapView;

/// Maps metres relative to the centre to pixels.
#[derive(Debug, Clone, Copy)]
pub(super) struct Projection {
    pub(super) centre: f32,
    pub(super) px_per_m: f32,
    cos: f32,
    sin: f32,
}

impl Projection {
    pub(super) fn new(size: u32, view: MapView) -> Self {
        let centre = size as f32 / 2.0;
        Self {
            centre,
            px_per_m: centre / view.radius_m.max(1.0),
            cos: view.up_heading.cos(),
            sin: view.up_heading.sin(),
        }
    }

    /// Screen position with y down.
    pub(super) fn point(self, p: Vec2) -> Vec2 {
        let x = p.x * self.cos - p.y * self.sin;
        let up = p.x * self.sin + p.y * self.cos;
        Vec2::new(
            self.centre + x * self.px_per_m,
            self.centre - up * self.px_per_m,
        )
    }
}

/// An anti-aliased line of `width_px`: only the pixels near the line are visited (a few per step
/// along the major axis), with no square roots, so a redraw stays far below a millisecond.
pub(super) fn draw_line(canvas: &mut Canvas, a: Vec2, b: Vec2, width_px: f32, color: [u8; 4]) {
    let d = b - a;
    let len = d.length();
    let half = width_px / 2.0 + 0.5;
    let size = canvas.size as f32;
    let (min, max) = (a.min(b) - Vec2::splat(half), a.max(b) + Vec2::splat(half));
    if max.x < 0.0 || max.y < 0.0 || min.x > size || min.y > size {
        return;
    }
    let (u, n) = if len > 1e-4 {
        let u = d / len;
        (u, Vec2::new(-u.y, u.x))
    } else {
        (Vec2::X, Vec2::Y)
    };
    // Extend the ends a little so chords of one arc join without notches.
    let cap = (width_px / 2.0).min(2.0);
    // Plain scalars and `inline(always)`: this is the hot loop (about 100 pixels per chord), and
    // the dev profile (opt-level 1 for this crate) does not inline `Vec2` operator calls here.
    let (ax, ay, ux, uy, nx, ny) = (a.x, a.y, u.x, u.y, n.x, n.y);
    let hw = width_px / 2.0 + 0.5;
    #[inline(always)]
    fn cover(
        (px, py): (f32, f32),
        (ax, ay, ux, uy, nx, ny): (f32, f32, f32, f32, f32, f32),
        (len, cap, hw): (f32, f32, f32),
    ) -> f32 {
        let (qx, qy) = (px - ax, py - ay);
        let t = qx * ux + qy * uy;
        if t < -cap || t > len + cap {
            return 0.0;
        }
        hw - (qx * nx + qy * ny).abs()
    }
    let frame = (ax, ay, ux, uy, nx, ny);
    let ends = (len, cap, hw);
    if d.x.abs() >= d.y.abs() {
        let x0 = min.x.floor().max(0.0) as i32;
        let x1 = max.x.ceil().min(size - 1.0) as i32;
        // Vertical reach of the line at a column: half / cos(angle).
        let reach = half / u.x.abs().max(0.2);
        for x in x0..=x1 {
            let cx = x as f32 + 0.5;
            let yc = if d.x.abs() > 1e-4 {
                a.y + (cx - a.x) * d.y / d.x
            } else {
                a.y
            };
            let yc = yc.clamp(min.y, max.y);
            let y0 = ((yc - reach).floor() as i32).max(0);
            let y1 = ((yc + reach).ceil() as i32).min(size as i32 - 1);
            for y in y0..=y1 {
                let c = cover((cx, y as f32 + 0.5), frame, ends);
                if c > 0.0 {
                    canvas.blend(x, y, color, c);
                }
            }
        }
    } else {
        let y0 = min.y.floor().max(0.0) as i32;
        let y1 = max.y.ceil().min(size - 1.0) as i32;
        let reach = half / u.y.abs().max(0.2);
        for y in y0..=y1 {
            let cy = y as f32 + 0.5;
            let xc = a.x + (cy - a.y) * d.x / d.y;
            let xc = xc.clamp(min.x, max.x);
            let x0 = ((xc - reach).floor() as i32).max(0);
            let x1 = ((xc + reach).ceil() as i32).min(size as i32 - 1);
            for x in x0..=x1 {
                let c = cover((x as f32 + 0.5, cy), frame, ends);
                if c > 0.0 {
                    canvas.blend(x, y, color, c);
                }
            }
        }
    }
}
