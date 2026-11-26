//! Path geometry in OMSI's conventions.
//!
//! Positions are `(x east, y north)` in metres; headings are **compass-style** (clockwise from
//! +Y, radians here; OMSI stores degrees). A positive radius turns clockwise (right). These are
//! the conventions of `[spline]` and `[path]` records (`docs/src/formats/map/tiles.md`,
//! `docs/src/formats/text/sco.md`); the same arc formula is used by the `crossing_audit` example
//! of `opts-core`.

use glam::DVec2;

/// `.sco` `[path]` records write `-999.99…` for "straight": radii of object paths at or beyond
/// this magnitude are read as straight (the importer `source` of `opts-traffic`). Splines have no
/// such marker: their mesh arcs for every radius, and so do their lanes.
pub const SCO_STRAIGHT_RADIUS: f64 = 999.0;

/// A straight line or circular arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathGeometry {
    /// Start point.
    pub start: DVec2,
    /// Heading at the start, radians, clockwise from +Y.
    pub heading: f64,
    /// Signed radius in metres (positive = clockwise); `0` means straight.
    pub radius: f64,
    /// Length along the path in metres.
    pub length: f64,
}

impl PathGeometry {
    /// A straight segment.
    #[must_use]
    pub fn straight(start: DVec2, heading: f64, length: f64) -> Self {
        Self {
            start,
            heading,
            radius: 0.0,
            length,
        }
    }

    /// Whether the segment is treated as a straight line.
    #[must_use]
    pub fn is_straight(&self) -> bool {
        self.radius.abs() < 1e-3 || !self.radius.is_finite()
    }

    /// Heading after `s` metres.
    #[must_use]
    pub fn heading_at(&self, s: f64) -> f64 {
        if self.is_straight() {
            self.heading
        } else {
            self.heading + s / self.radius
        }
    }

    /// Point after `s` metres (`s` is clamped to the path).
    #[must_use]
    pub fn point_at(&self, s: f64) -> DVec2 {
        // `max` then `min` (not `clamp`): degenerate lengths must not panic.
        let s = s.max(0.0).min(self.length.max(0.0));
        if self.is_straight() {
            self.start + DVec2::new(self.heading.sin(), self.heading.cos()) * s
        } else {
            // Chord form: exact for very large radii, where differences of cosines cancel.
            let turn = s / self.radius;
            let chord = 2.0 * self.radius * (turn / 2.0).sin();
            let mid = self.heading + turn / 2.0;
            self.start + DVec2::new(mid.sin(), mid.cos()) * chord
        }
    }

    /// End point.
    #[must_use]
    pub fn end(&self) -> DVec2 {
        self.point_at(self.length)
    }

    /// Heading at the end.
    #[must_use]
    pub fn end_heading(&self) -> f64 {
        self.heading_at(self.length)
    }

    /// The same path traversed backwards.
    #[must_use]
    pub fn reversed(&self) -> Self {
        Self {
            start: self.end(),
            heading: self.end_heading() + std::f64::consts::PI,
            radius: if self.is_straight() {
                self.radius
            } else {
                -self.radius
            },
            length: self.length,
        }
    }

    /// The path shifted sideways by `offset` metres (positive = to the right of the travel
    /// direction), as a `[path]` of a spline does with its `offsetX`. On an arc the radius
    /// shrinks (clockwise) or grows with the offset and the length scales with it.
    #[must_use]
    pub fn offset(&self, offset: f64) -> Self {
        let right = DVec2::new(self.heading.cos(), -self.heading.sin());
        let start = self.start + right * offset;
        if self.is_straight() {
            return Self { start, ..*self };
        }
        let radius = self.radius - offset;
        Self {
            start,
            heading: self.heading,
            radius,
            length: self.length * radius / self.radius,
        }
    }

    /// Places an object-local path in the world. The object sits at `origin` rotated clockwise by
    /// `object_heading` radians (the convention of `[object]` records, verified for crossings by
    /// the `crossing_audit` example).
    #[must_use]
    pub fn to_world(&self, origin: DVec2, object_heading: f64) -> Self {
        let (s, c) = object_heading.sin_cos();
        let rot = |p: DVec2| DVec2::new(p.x * c + p.y * s, -p.x * s + p.y * c);
        Self {
            start: origin + rot(self.start),
            heading: self.heading + object_heading,
            ..*self
        }
    }
}
