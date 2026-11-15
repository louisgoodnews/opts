//! Footpath and bike lanes of placed splines: the arc sampler and the spline ribbon lanes.

use glam::Vec3;
use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::{LaneKind, SplineProfile};
use super::model::{FootLane, Travel};
use super::{PATH_TYPE_PEDESTRIAN, SAMPLE_STEP_M};

/// Centreline of an arc segment: `(position, heading_rad)` every [`SAMPLE_STEP_M`] metres.
///
/// Same arc as `opts_bevy::scenery::spline_pose` (heading clockwise from north; a positive
/// radius turns right; `0` is straight); `height(s)` is the elevation above the start. The chord
/// form keeps very large radii exact in `f32`.
pub(super) fn arc(
    start: Vec3,
    heading_deg: f32,
    radius: f32,
    length: f32,
    height: impl Fn(f32) -> f32,
) -> Vec<(Vec3, f32)> {
    let phi = f64::from(heading_deg.to_radians());
    let steps = ((length / SAMPLE_STEP_M).ceil() as usize).clamp(1, 2000);
    (0..=steps)
        .map(|k| {
            let s = length * k as f32 / steps as f32;
            let (h, x, y) = if radius.abs() > 1e-3 {
                let (r, sd) = (f64::from(radius), f64::from(s));
                let turn = sd / r;
                let chord = 2.0 * r * (turn / 2.0).sin();
                let mid = phi + turn / 2.0;
                (
                    (phi + turn) as f32,
                    start.x + (chord * mid.sin()) as f32,
                    start.y + (chord * mid.cos()) as f32,
                )
            } else {
                (
                    phi as f32,
                    start.x + s * (phi.sin() as f32),
                    start.y + s * (phi.cos() as f32),
                )
            };
            (Vec3::new(x, y, start.z + height(s)), h)
        })
        .collect()
}

/// Footpath lanes of a placed spline: one per type-1 `[path]` of its profile.
///
/// See [`lanes_of_kind`] for other lane kinds (bike lanes).
///
/// `shift` moves the tile-local coordinates into the world frame. `[spline_h]` records with an
/// explicit height change follow the cubic profile of the ribbon.
#[must_use]
pub fn lanes_from_spline(
    inst: &MapSplineInstance,
    profile: &SplineProfile,
    shift: (f32, f32),
) -> Vec<FootLane> {
    lanes_of_kind(inst, profile, "", shift, LaneKind::Pedestrian)
}

/// Lanes of one [`LaneKind`] of a placed spline (`name` is the `.sli` file name, which feeds the
/// bike heuristic of [`opts_core::formats::scenery::SplinePath::kind`]); same geometry rules as [`lanes_from_spline`].
#[must_use]
pub fn lanes_of_kind(
    inst: &MapSplineInstance,
    profile: &SplineProfile,
    name: &str,
    shift: (f32, f32),
    kind: LaneKind,
) -> Vec<FootLane> {
    let length = inst.length_m.max(0.01);
    let height = |s: f32| inst.height_offset(s);
    let centre = arc(
        inst.position,
        inst.heading_deg,
        inst.radius_m,
        length,
        height,
    );
    profile
        .paths
        .iter()
        .filter(|p| match kind {
            LaneKind::Pedestrian => p.path_type == PATH_TYPE_PEDESTRIAN,
            k => p.kind(name) == k,
        })
        .filter_map(|p| {
            let offset = if inst.mirrored {
                -p.offset_x
            } else {
                p.offset_x
            };
            let points = centre
                .iter()
                .map(|(c, h)| {
                    // Right of the direction of travel.
                    Vec3::new(
                        c.x + h.cos() * offset + shift.0,
                        c.y - h.sin() * offset + shift.1,
                        c.z + p.offset_z,
                    )
                })
                .collect();
            FootLane::new(points, Travel::from_field(p.direction), p.width)
        })
        .collect()
}
