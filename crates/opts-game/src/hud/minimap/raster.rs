//! The pure drawing of the mini map: roads and markers into an RGBA8 buffer.
//!
//! No Bevy, no world: the inputs are a [`MapView`], road polylines and markers in metres relative
//! to the centre (east, north), and the colours. That makes the drawing deterministic, testable
//! against pixels, and cheap: coverage-based anti-aliased lines, culled to the square, with arcs
//! cut into chords of at least a few pixels. The budget is 1 ms for a frame (checked by a test
//! and reported live by `hud.state`).

use bevy::math::Vec2;

/// What the map shows of the world: visible radius and orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapView {
    /// Metres from the centre to the edge of the square.
    pub radius_m: f32,
    /// Compass heading (radians, clockwise from north) that points up on the screen; `0` is
    /// north-up.
    pub up_heading: f32,
}

/// One drawable piece of a road: a straight chord, relative to the centre of the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapLine {
    /// Start (east, north) in metres.
    pub a: Vec2,
    /// End (east, north) in metres.
    pub b: Vec2,
    /// Width in metres.
    pub width_m: f32,
}

/// A vehicle (or the free camera) on the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapMarker {
    /// Position (east, north) in metres relative to the centre.
    pub pos: Vec2,
    /// Compass heading in radians.
    pub heading: f32,
    /// The vehicle the user drives (bigger, filled, with a ring).
    pub active: bool,
}

/// Colours as RGBA8, from the instrument tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapColors {
    /// Opaque ground.
    pub ground: [u8; 4],
    /// Roads.
    pub road: [u8; 4],
    /// The active vehicle.
    pub active: [u8; 4],
    /// The other session vehicles.
    pub other: [u8; 4],
    /// North mark and frame.
    pub north: [u8; 4],
}

/// The buffer: `size * size` RGBA8 pixels, row 0 on top.
#[derive(Debug, Clone)]
pub struct Canvas {
    /// Side in pixels.
    pub size: u32,
    /// The pixels.
    pub data: Vec<u8>,
}

impl Canvas {
    /// A canvas filled with `ground`.
    #[must_use]
    pub fn new(size: u32, ground: [u8; 4]) -> Self {
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..size * size {
            data.extend_from_slice(&ground);
        }
        Self { size, data }
    }

    /// Resets to `ground` (keeps the allocation).
    pub fn clear(&mut self, ground: [u8; 4]) {
        for px in self.data.chunks_exact_mut(4) {
            px.copy_from_slice(&ground);
        }
    }

    /// The pixel at (`x`, `y`).
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.size + x) * 4) as usize;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    fn blend(&mut self, x: i32, y: i32, color: [u8; 4], coverage: f32) {
        if x < 0 || y < 0 || x >= self.size as i32 || y >= self.size as i32 || coverage <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.size + x as u32) * 4) as usize;
        let c = coverage.min(1.0);
        for k in 0..3 {
            let old = f32::from(self.data[i + k]);
            let new = f32::from(color[k]);
            self.data[i + k] = (old + (new - old) * c).round() as u8;
        }
        self.data[i + 3] = 255;
    }
}

/// Maps metres relative to the centre to pixels.
#[derive(Debug, Clone, Copy)]
struct Projection {
    centre: f32,
    px_per_m: f32,
    cos: f32,
    sin: f32,
}

impl Projection {
    fn new(size: u32, view: MapView) -> Self {
        let centre = size as f32 / 2.0;
        Self {
            centre,
            px_per_m: centre / view.radius_m.max(1.0),
            cos: view.up_heading.cos(),
            sin: view.up_heading.sin(),
        }
    }

    /// Screen position with y down.
    fn point(self, p: Vec2) -> Vec2 {
        let x = p.x * self.cos - p.y * self.sin;
        let up = p.x * self.sin + p.y * self.cos;
        Vec2::new(
            self.centre + x * self.px_per_m,
            self.centre - up * self.px_per_m,
        )
    }
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 <= f32::EPSILON {
        0.0
    } else {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    p.distance(a + ab * t)
}

fn draw_line(canvas: &mut Canvas, a: Vec2, b: Vec2, width_px: f32, color: [u8; 4]) {
    let half = width_px / 2.0 + 0.5;
    let size = canvas.size as f32;
    let (min, max) = (a.min(b) - Vec2::splat(half), a.max(b) + Vec2::splat(half));
    if max.x < 0.0 || max.y < 0.0 || min.x > size || min.y > size {
        return;
    }
    let x0 = min.x.floor().max(0.0) as i32;
    let x1 = max.x.ceil().min(size - 1.0) as i32;
    let y0 = min.y.floor().max(0.0) as i32;
    let y1 = max.y.ceil().min(size - 1.0) as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let d = segment_distance(p, a, b);
            canvas.blend(x, y, color, width_px / 2.0 + 0.5 - d);
        }
    }
}

/// Fills a triangle (anti-aliased by the signed distance to its edges).
fn fill_triangle(canvas: &mut Canvas, p: [Vec2; 3], color: [u8; 4], grow: f32) {
    let size = canvas.size as f32;
    let min = p[0].min(p[1]).min(p[2]) - Vec2::splat(grow + 1.0);
    let max = p[0].max(p[1]).max(p[2]) + Vec2::splat(grow + 1.0);
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

fn draw_ring(canvas: &mut Canvas, c: Vec2, radius: f32, width: f32, color: [u8; 4]) {
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
fn arrow(centre: Vec2, screen_heading: f32, length: f32) -> [Vec2; 3] {
    let dir = Vec2::new(screen_heading.sin(), -screen_heading.cos());
    let side = Vec2::new(-dir.y, dir.x);
    [
        centre + dir * length,
        centre - dir * (length * 0.7) + side * (length * 0.75),
        centre - dir * (length * 0.7) - side * (length * 0.75),
    ]
}

/// Draws the whole map into `canvas` (which is cleared first).
pub fn render(
    canvas: &mut Canvas,
    view: MapView,
    lines: &[MapLine],
    markers: &[MapMarker],
    colors: &MapColors,
) {
    canvas.clear(colors.ground);
    let proj = Projection::new(canvas.size, view);
    let unit = canvas.size as f32 / 176.0;
    for l in lines {
        let width = (l.width_m * proj.px_per_m).max(1.6 * unit.max(0.75));
        draw_line(canvas, proj.point(l.a), proj.point(l.b), width, colors.road);
    }
    // Others first, the active vehicle on top. A ground-coloured halo separates a marker from the
    // roads under it.
    let mut order: Vec<&MapMarker> = markers.iter().collect();
    order.sort_by_key(|m| m.active);
    for m in order {
        let c = proj.point(m.pos);
        let heading = m.heading - view.up_heading;
        if m.active {
            let t = arrow(c, heading, 9.0 * unit);
            fill_triangle(canvas, t, colors.ground, 2.0 * unit);
            draw_ring(canvas, c, 12.5 * unit, 2.0 * unit, colors.ground);
            draw_ring(canvas, c, 12.5 * unit, 1.2 * unit, colors.active);
            fill_triangle(canvas, t, colors.active, 0.0);
        } else {
            // Hollow arrow: the outline is the colour, the inside is the ground.
            let t = arrow(c, heading, 7.0 * unit);
            fill_triangle(canvas, t, colors.ground, 2.2 * unit);
            fill_triangle(canvas, t, colors.other, 1.2 * unit);
            let inner = arrow(c, heading, 7.0 * unit);
            fill_triangle(canvas, inner, colors.ground, -1.4 * unit);
        }
    }
    // The north mark: a small triangle on the edge at the direction of north.
    let north = -view.up_heading;
    let dir = Vec2::new(north.sin(), -north.cos());
    let at = Vec2::splat(proj.centre) + dir * (proj.centre - 9.0 * unit);
    let t = arrow(at, north, 6.0 * unit);
    fill_triangle(canvas, t, colors.ground, 1.5 * unit);
    fill_triangle(canvas, t, colors.north, 0.0);
}
