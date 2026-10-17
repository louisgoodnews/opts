//! Path geometry in OMSI's conventions.
//!
//! Positions are `(x east, y north)` in metres; headings are **compass-style** (clockwise from
//! +Y, radians here; OMSI stores degrees). A positive radius turns clockwise (right). These are
//! the conventions of `[spline]` and `[path]` records (`docs/src/formats/map/tiles.md`,
//! `docs/src/formats/text/sco.md`); the same arc formula is used by the `crossing_audit` example
//! of `opts-core`.

use glam::DVec2;

/// `.sco` `[path]` records write `-999.99…` for "straight": radii of object paths at or beyond
/// this magnitude are read as straight ([`crate::source`]). Splines have no such marker: their
/// mesh arcs for every radius, and so do their lanes.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn heading_zero_goes_north_and_ninety_east() {
        let n = PathGeometry::straight(DVec2::ZERO, 0.0, 10.0);
        assert!(n.end().abs_diff_eq(DVec2::new(0.0, 10.0), 1e-9));
        let e = PathGeometry::straight(DVec2::ZERO, FRAC_PI_2, 10.0);
        assert!(e.end().abs_diff_eq(DVec2::new(10.0, 0.0), 1e-9));
    }

    #[test]
    fn positive_radius_turns_right() {
        // Quarter circle of radius 10 starting north: ends 10 east and 10 north, heading east.
        let arc = PathGeometry {
            start: DVec2::ZERO,
            heading: 0.0,
            radius: 10.0,
            length: 10.0 * FRAC_PI_2,
        };
        assert!(
            arc.end().abs_diff_eq(DVec2::new(10.0, 10.0), 1e-9),
            "{:?}",
            arc.end()
        );
        assert!((arc.end_heading() - FRAC_PI_2).abs() < 1e-9);
        let left = PathGeometry {
            radius: -10.0,
            ..arc
        };
        assert!(left.end().abs_diff_eq(DVec2::new(-10.0, 10.0), 1e-9));
    }

    #[test]
    fn large_radii_arc_like_the_spline_mesh() {
        // A spline of radius 1100 m and 183 m length (Spandau has such): the end lies
        // L²/2R ≈ 15.2 m beside the straight line, exactly where the mesh formula puts it.
        let g = PathGeometry {
            start: DVec2::new(5.0, 7.0),
            heading: 0.3,
            radius: 1100.0,
            length: 183.0,
        };
        assert!(!g.is_straight());
        let h = g.end_heading();
        let mesh = g.start
            + DVec2::new(
                -1100.0 * (h.cos() - 0.3f64.cos()),
                1100.0 * (h.sin() - 0.3f64.sin()),
            );
        assert!(g.end().distance(mesh) < 0.01, "{:?} vs {mesh:?}", g.end());
        let straight = PathGeometry::straight(g.start, 0.3, 183.0).end();
        assert!((g.end().distance(straight) - 183.0 * 183.0 / 2200.0).abs() < 0.1);
        // Even a radius of 10⁷ m stays exact (no cancellation).
        let huge = PathGeometry {
            radius: 1.0e7,
            ..PathGeometry::straight(DVec2::ZERO, 0.0, 100.0)
        };
        assert!(huge.end().abs_diff_eq(DVec2::new(0.0005, 100.0), 1e-6));
    }

    #[test]
    fn reversed_path_retraces_the_original() {
        let arc = PathGeometry {
            start: DVec2::new(3.0, 4.0),
            heading: 0.7,
            radius: -25.0,
            length: 18.0,
        };
        let back = arc.reversed();
        assert!(back.start.abs_diff_eq(arc.end(), 1e-9));
        assert!(back.end().abs_diff_eq(arc.start, 1e-9));
        let mid = arc.point_at(7.0);
        assert!(back.point_at(18.0 - 7.0).abs_diff_eq(mid, 1e-9));
    }

    #[test]
    fn right_offset_on_a_right_curve_is_the_inner_lane() {
        let arc = PathGeometry {
            start: DVec2::ZERO,
            heading: 0.0,
            radius: 20.0,
            length: 10.0,
        };
        let inner = arc.offset(2.5);
        assert!((inner.radius - 17.5).abs() < 1e-12);
        // Same turning angle, shorter arc, start 2.5 m to the east.
        assert!(inner.start.abs_diff_eq(DVec2::new(2.5, 0.0), 1e-12));
        assert!((inner.length / inner.radius - arc.length / arc.radius).abs() < 1e-12);
    }

    #[test]
    fn object_local_paths_rotate_clockwise() {
        // A path starting 5 m north of the object, object turned 90° clockwise: it starts 5 m east.
        let local = PathGeometry::straight(DVec2::new(0.0, 5.0), 0.0, 10.0);
        let world = local.to_world(DVec2::new(100.0, 100.0), FRAC_PI_2);
        assert!(
            world.start.abs_diff_eq(DVec2::new(105.0, 100.0), 1e-9),
            "{:?}",
            world.start
        );
        assert!((world.heading - FRAC_PI_2).abs() < 1e-12);
    }
}

#[cfg(test)]
mod degenerate_tests {
    use super::*;

    #[test]
    fn negative_or_zero_lengths_do_not_panic() {
        let g = PathGeometry::straight(DVec2::new(1.0, 2.0), 0.3, -9.11);
        assert_eq!(g.point_at(5.0), g.start);
        assert_eq!(
            PathGeometry::straight(DVec2::ZERO, 0.0, 0.0).end(),
            DVec2::ZERO
        );
    }
}
