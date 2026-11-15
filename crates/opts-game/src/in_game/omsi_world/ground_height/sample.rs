//! One road sample of a spline's centre line: its height, width and the profile's surface around it.

use super::*;

#[derive(Clone)]
pub(super) struct RoadSample {
    pub(super) x: f32,
    pub(super) y: f32,
    /// Height of the spline's datum at this sample.
    pub(super) z: f32,
    pub(super) half_width: f32,
    /// Heading of the spline here (rad, clockwise from north).
    pub(super) heading: f32,
    /// The profile's ground surface (heights relative to the datum), shared by the samples of
    /// one spline: the drawn road surface is the datum plus this, not the datum itself.
    pub(super) surface: std::sync::Arc<Vec<opts_core::formats::scenery::SurfaceSegment>>,
    /// The spline is mirrored (its profile's `x` is negated).
    pub(super) mirrored: bool,
    /// Distance of the sample along its spline (m).
    pub(super) along_m: f32,
    /// The profile's strips with a height map (M-0146), shared by the samples of one spline.
    pub(super) strips: std::sync::Arc<Vec<super::super::road_surf::TexStrip>>,
}

impl RoadSample {
    /// Height of the drawn road surface at `(x, y)`: the datum plus the profile's ground
    /// surface at the point's lateral offset; the datum where the profile lays none there.
    pub(super) fn surface_z(&self, x: f32, y: f32) -> f32 {
        let px = self.profile_x(x, y);
        self.surface
            .iter()
            .find(|seg| seg.x0 - 0.05 <= px && px <= seg.x1 + 0.05)
            .map_or(self.z, |seg| self.z + seg.z_at(px))
    }

    /// Lateral offset of `(x, y)` in the profile's frame (m, right of the direction of travel,
    /// as the ribbons are extruded: `(cos h, -sin h)`; negated for a mirrored spline).
    pub(super) fn profile_x(&self, x: f32, y: f32) -> f32 {
        let (sin, cos) = self.heading.sin_cos();
        let lateral = (x - self.x) * cos - (y - self.y) * sin;
        if self.mirrored {
            -lateral
        } else {
            lateral
        }
    }

    /// The fine unevenness of the road texture at `(x, y)` (m, M-0146).
    pub(super) fn unevenness(&self, x: f32, y: f32) -> f32 {
        if self.strips.is_empty() {
            return 0.0;
        }
        let (sin, cos) = self.heading.sin_cos();
        let along = self.along_m + (x - self.x) * sin + (y - self.y) * cos;
        let surface = self.surface_z(x, y) - self.z;
        super::super::road_surf::unevenness_at(&self.strips, self.profile_x(x, y), along, surface)
    }
}
