//! Ground triangles of the splines and the carving statistics.

use super::*;

/// A triangle of a drawn ground surface (a spline's surface ribbon, a crossing's ground face)
/// in a tile's frame: `(east, north, height)` metres.
pub type GroundTriangle = [Vec3; 3];

/// The surface ribbon of a placed spline: its profile's ground surface
/// ([`SplineProfile::ground_surface`]) extruded over the same stations as the drawn spline
/// ([`spline_stations`], `[spline_h]` ramps included). `offset` is the corner of the spline's tile
/// in the frame the triangles are wanted in (zero for the spline's own tile). Mirrored splines
/// negate the profile's `x`; profiles without a surface give nothing.
#[must_use]
pub fn spline_ground_triangles(
    profile: &SplineProfile,
    inst: &MapSplineInstance,
    offset: (f32, f32),
) -> Vec<GroundTriangle> {
    let surface = profile.ground_surface();
    if surface.is_empty() {
        return Vec::new();
    }
    let side = if inst.mirrored { -1.0 } else { 1.0 };
    let stations = spline_stations(inst, offset);
    let at = |k: usize, x: f32, z: f32| {
        let (ex, ny, sz, h, _) = stations[k];
        let x = x * side;
        Vec3::new(ex + h.cos() * x, ny - h.sin() * x, sz + z)
    };
    let mut out = Vec::with_capacity(surface.len() * stations.len() * 2);
    for k in 0..stations.len().saturating_sub(1) {
        for seg in &surface {
            let (a, b) = (at(k, seg.x0, seg.z0), at(k, seg.x1, seg.z1));
            let (c, d) = (at(k + 1, seg.x0, seg.z0), at(k + 1, seg.x1, seg.z1));
            out.push([a, b, c]);
            out.push([b, d, c]);
        }
    }
    out
}

/// What [`carve_terrain`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CarveStats {
    /// Vertices lowered.
    pub lowered: usize,
    /// Vertices a surface would have lowered by more than [`CARVE_COVER_M`] (left as authored).
    pub guarded: usize,
    /// Vertices under a surface raised up to just below it (the terrain lay lower than the
    /// surface by at most [`ALIGN_MAX_M`]): the terrain beside the surface edge then starts at
    /// the edge's height instead of a pavement's height lower.
    pub raised: usize,
    /// Vertices changed by the fit of the terrain just outside the outer edges to their height.
    pub fitted: usize,
    /// Vertices outside the surfaces pulled towards the height of the nearest outer edge
    /// ([`align_band`], [`ALIGN_MAX_M`]).
    pub aligned: usize,
}
