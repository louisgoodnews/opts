//! Extrusion of `.sli` graphical profiles along a placed spline into Bevy meshes.

use super::{MapSplineInstance, SplineProfile};
use bevy::math::Vec3;
use bevy::mesh::Mesh;

use super::pose::spline_stations;
use crate::mesh::smooth_normal_mesh;

/// Extrudes a 2D spline profile along a straight segment of length `length_m`
/// producing Bevy [`Mesh`] primitives partitioned by texture slot.
///
/// Convenience wrapper over [`extrude_spline_segment`] for a segment starting at the origin and
/// heading north (Bevy `-Z`). Returns `(texture_index, Mesh)` per graphical profile ribbon.
#[must_use]
pub fn extrude_spline_profile(profile: &SplineProfile, length_m: f32) -> Vec<(usize, Mesh)> {
    let segment = MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 0,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::ZERO,
        heading_deg: 0.0,
        length_m,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    };
    extrude_spline_segment(profile, &segment, (0.0, 0.0))
}

/// Extrudes a `.sli` graphical profile along a placed spline segment.
///
/// The segment starts at `inst.position` (tile-local), heads along `inst.heading_deg`
/// (0° = north, **clockwise** positive like a compass), follows the arc given by
/// `inst.radius_m` (positive radius = heading increases = turns right) and blends the
/// longitudinal slope from `gradient_start` to `gradient_end`. Both sign conventions were
/// derived from the corpus: the end of a placed segment coincides with the start of the
/// segment it connects to only under these conventions.
///
/// Returns `(texture_index, mesh)` per graphical profile, in absolute Bevy world coordinates.
#[must_use]
pub fn extrude_spline_segment(
    profile: &SplineProfile,
    inst: &MapSplineInstance,
    tile_origin: (f32, f32),
) -> Vec<(usize, Mesh)> {
    let samples = spline_stations(inst, tile_origin);
    let mut out = Vec::new();
    for gp in &profile.graphical_profiles {
        if gp.points.len() < 2 {
            continue;
        }
        let n = gp.points.len();
        let mut positions = Vec::with_capacity(n * samples.len());
        let mut uvs = Vec::with_capacity(n * samples.len());
        // A mirrored spline lays its profile out with x negated.
        let side = if inst.mirrored { -1.0 } else { 1.0 };
        for &(ex, ny, z, h, s) in &samples {
            let (rx, ry) = (h.cos(), -h.sin());
            for p in &gp.points {
                let x = p.x * side;
                positions.push([ex + rx * x, z + p.z, -(ny + ry * x)]);
                uvs.push([p.u, s * p.v_slope]);
            }
        }
        let mut indices = Vec::new();
        for k in 0..samples.len() - 1 {
            for i in 0..n - 1 {
                let a = (k * n + i) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                if inst.mirrored {
                    indices.extend_from_slice(&[a, c, b, b, c, d]);
                } else {
                    indices.extend_from_slice(&[a, b, c, b, d, c]);
                }
            }
        }
        out.push((
            gp.texture_index,
            smooth_normal_mesh(positions, uvs, indices),
        ));
    }
    out
}
