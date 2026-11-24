//! Placed splines of a tile: the spline record, its height profile and its end pose.

use super::*;

/// Placed spline instance on a tile (`[spline]`, `[spline_h]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapSplineInstance {
    /// Spline placement flag.
    pub flag: i32,
    /// Relative path to `.sli` profile file.
    pub sli_file: String,
    /// Unique spline identifier code.
    pub id_code: u64,
    /// Connected element identifier 1 (or 0 if unconnected).
    pub conn_id_1: u64,
    /// Connected element identifier 2 (or 0 if unconnected).
    pub conn_id_2: u64,
    /// 3D position in standard engine coordinates (X = easting, Y = northing, Z = elevation).
    ///
    /// In OMSI's raw `tile_*.map` format, Line 5 is X, Line 6 is Z (elevation), and Line 7 is Y.
    /// This coordinate ordering was experimentally verified via in-engine Mutation Kit 2.
    pub position: Vec3,
    /// Heading (yaw) in degrees around vertical axis.
    pub heading_deg: f32,
    /// Segment arc length in meters.
    pub length_m: f32,
    /// Curvature radius in meters (0 for straight segment).
    pub radius_m: f32,
    /// Longitudinal gradient at start as a slope ratio (`[spline]` parameter 11, percent / 100).
    pub gradient_start: f32,
    /// Longitudinal gradient at end as a slope ratio (parameter 12, percent / 100).
    pub gradient_end: f32,
    /// Whether this spline includes an explicit height profile (`[spline_h]`).
    pub has_height_profile: bool,
    /// Whether the profile is mirrored across the spline axis (trailing `mirror` token).
    pub mirrored: bool,
    /// Additional raw parameters preserved from the block.
    pub raw_params: Vec<String>,
}

impl opts_types::BorderSpline for MapSplineInstance {
    fn id_code(&self) -> u64 {
        self.id_code
    }
    fn next_id_code(&self) -> u64 {
        self.conn_id_2
    }
    fn start(&self) -> (f64, f64) {
        (f64::from(self.position.x), f64::from(self.position.y))
    }
    fn heading_deg(&self) -> f64 {
        f64::from(self.heading_deg)
    }
    fn length_m(&self) -> f64 {
        f64::from(self.length_m)
    }
    fn radius_m(&self) -> f64 {
        f64::from(self.radius_m)
    }
}

/// Height above the start of a spline's height profile after `s` of `length` metres.
///
/// With an explicit total height change `rise` (`[spline_h]` parameter 13) the profile is a cubic
/// Hermite ramp between the end gradients; otherwise the gradient changes linearly from start to
/// end (`rise = length · (g0 + g1) / 2`). The road mesh, footpaths and AI lanes share this rule
/// (`docs/src/formats/map/tiles.md`).
#[must_use]
pub fn spline_height_offset(
    gradient_start: f64,
    gradient_end: f64,
    rise: Option<f64>,
    s: f64,
    length: f64,
) -> f64 {
    let length = length.max(1e-6);
    let s = s.max(0.0).min(length);
    match rise {
        Some(h) => {
            let t = s / length;
            let (m0, m1) = (gradient_start * length, gradient_end * length);
            let (t2, t3) = (t * t, t * t * t);
            (t3 - 2.0 * t2 + t) * m0 + (-2.0 * t3 + 3.0 * t2) * h + (t3 - t2) * m1
        }
        None => gradient_start * s + (gradient_end - gradient_start) * s * s / (2.0 * length),
    }
}

impl MapSplineInstance {
    /// The explicit total height change of a `[spline_h]` record (parameter 13), when it has one
    /// that is not zero.
    #[must_use]
    pub fn explicit_rise(&self) -> Option<f32> {
        self.raw_params
            .get(2)
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|h| self.has_height_profile && h.abs() > 1e-6)
    }

    /// Height of the centre line above the spline's start after `s` metres (see
    /// [`spline_height_offset`]).
    #[must_use]
    pub fn height_offset(&self, s: f32) -> f32 {
        spline_height_offset(
            f64::from(self.gradient_start),
            f64::from(self.gradient_end),
            self.explicit_rise().map(f64::from),
            f64::from(s),
            f64::from(self.length_m.max(0.01)),
        ) as f32
    }
}

/// End pose `(x, y, z, heading_deg)` of a spline: straight or circular arc (`radius_m != 0`),
/// the end height rises by `length * (g_start + g_end) / 2`.
pub(super) fn spline_end_pose(s: &MapSplineInstance) -> (f32, f32, f32, f32) {
    let phi = f64::from(s.heading_deg).to_radians();
    let len = f64::from(s.length_m);
    let r = f64::from(s.radius_m);
    let (x0, y0) = (f64::from(s.position.x), f64::from(s.position.y));
    let (h1, x1, y1) = if r.abs() > 1e-3 {
        let h = phi + len / r;
        (
            h,
            x0 - r * (h.cos() - phi.cos()),
            y0 + r * (h.sin() - phi.sin()),
        )
    } else {
        (phi, x0 + len * phi.sin(), y0 + len * phi.cos())
    };
    let dz = len * f64::from(s.gradient_start + s.gradient_end) / 2.0;
    (
        x1 as f32,
        y1 as f32,
        (f64::from(s.position.z) + dz) as f32,
        h1.to_degrees().rem_euclid(360.0) as f32,
    )
}
