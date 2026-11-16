//! Footpath lanes of placed scenery objects (crossings, stops, junction plates) and of rail points.

use super::model::{FootLane, Gate, Travel};
use super::spline::arc;
use super::PATH_TYPE_PEDESTRIAN;
use glam::Vec3;
use opts_core::formats::scenery::sco::ScoPath;

/// Radii of `.sco` `[path]` records at or beyond this magnitude are the straight marker
/// (`-999.99`); splines arc for every radius, like their mesh.
pub const SCO_STRAIGHT_RADIUS: f32 = 999.0;

/// Footpath lanes of a placed scenery object (crossings, stops, junction plates).
///
/// `origin` is the object's world position (with its final elevation) and `heading_deg` its
/// clockwise heading; the path coordinates are object-local.
#[must_use]
pub fn lanes_from_object(paths: &[ScoPath], origin: Vec3, heading_deg: f32) -> Vec<FootLane> {
    lanes_from_object_of_type(paths, origin, heading_deg, PATH_TYPE_PEDESTRIAN)
}

/// Lanes of one `[path]` type of a placed scenery object (`0` road, `1` footpath, `2` rail);
/// see [`lanes_from_object`].
#[must_use]
pub fn lanes_from_object_of_type(
    paths: &[ScoPath],
    origin: Vec3,
    heading_deg: f32,
    path_type: u32,
) -> Vec<FootLane> {
    object_lanes(paths, origin, heading_deg, path_type, None)
}

/// Like [`lanes_from_object_of_type`] for rail points (`[switch]`): a path with a `[switchdir]`
/// is passable only in that position of the points `switch` (see [`crate::pedestrian::switches`]).
#[must_use]
pub fn lanes_from_points_object(
    paths: &[ScoPath],
    origin: Vec3,
    heading_deg: f32,
    path_type: u32,
    switch: u64,
) -> Vec<FootLane> {
    object_lanes(paths, origin, heading_deg, path_type, Some(switch))
}

fn object_lanes(
    paths: &[ScoPath],
    origin: Vec3,
    heading_deg: f32,
    path_type: u32,
    switch: Option<u64>,
) -> Vec<FootLane> {
    let (sin, cos) = heading_deg.to_radians().sin_cos();
    // Clockwise rotation of an object-local (x, y) vector.
    let to_world = |p: Vec3| {
        Vec3::new(
            origin.x + p.x * cos + p.y * sin,
            origin.y - p.x * sin + p.y * cos,
            origin.z + p.z,
        )
    };
    paths
        .iter()
        .filter(|p| p.path_type == path_type && p.length_m > 0.05)
        .filter_map(|p| {
            let length = p.length_m;
            let (s0, s1) = (p.slope_start, p.slope_end);
            let radius = if p.radius_m.abs() >= SCO_STRAIGHT_RADIUS {
                0.0
            } else {
                p.radius_m
            };
            let local = arc(
                Vec3::new(p.start.x, p.start.y, 0.0),
                p.heading_deg,
                radius,
                length,
                |s| s0 * s + (s1 - s0) * s * s / (2.0 * length),
            );
            let points = local
                .iter()
                .map(|(c, _)| to_world(Vec3::new(c.x, c.y, c.z + p.start.z)))
                .collect();
            let lane = FootLane::new(points, Travel::from_field(p.direction), p.width_m)?;
            Some(match (switch, p.switch_position) {
                (Some(switch), Some(position)) => lane.with_gate(Gate {
                    switch,
                    position: position.min(u32::from(u8::MAX)) as u8,
                }),
                _ => lane,
            })
        })
        .collect()
}
