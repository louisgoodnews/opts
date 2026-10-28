//! Footpath lanes sampled from spline profiles and scenery-object paths.

use glam::Vec3;
use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::sco::ScoPath;
use opts_core::formats::scenery::{LaneKind, SplineProfile};

/// Path type of pedestrians in `[path]` records.
pub const PATH_TYPE_PEDESTRIAN: u32 = 1;
/// Spacing of the lane's polyline vertices (m).
const SAMPLE_STEP_M: f32 = 1.0;

/// Directions a lane may be walked in, relative to its own direction of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Travel {
    /// Only from the start to the end (`direction` 0).
    Forward,
    /// Only from the end to the start (`direction` 1).
    Reverse,
    /// Both ways (`direction` 2).
    Both,
}

impl Travel {
    /// Reads the `direction` field of a `[path]`.
    #[must_use]
    pub fn from_field(direction: u32) -> Self {
        match direction {
            0 => Self::Forward,
            1 => Self::Reverse,
            _ => Self::Both,
        }
    }
}

/// A walkable polyline in the world frame (x east, y north, z up, metres).
#[derive(Debug, Clone, PartialEq)]
pub struct FootLane {
    /// Polyline vertices.
    pub points: Vec<Vec3>,
    /// Cumulative length at each vertex (`cum[0] = 0`).
    pub cum: Vec<f32>,
    /// Allowed directions.
    pub travel: Travel,
    /// Lane width (m).
    pub width: f32,
    /// The rail points position this lane belongs to, if any.
    pub gate: Option<Gate>,
}

/// A lane that is passable only while rail points are in one position (`[switchdir]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    /// The points, one key per placed object.
    pub switch: u64,
    /// The position of the points (`[switchdir]`) in which the lane is passable.
    pub position: u8,
}

impl FootLane {
    /// Builds a lane; `None` when it has fewer than two distinct points.
    #[must_use]
    pub fn new(points: Vec<Vec3>, travel: Travel, width: f32) -> Option<Self> {
        let mut cum = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                total += (*p - points[i - 1]).length();
            }
            cum.push(total);
        }
        (points.len() >= 2 && total > 0.05).then_some(Self {
            points,
            cum,
            travel,
            width,
            gate: None,
        })
    }

    /// The same lane, passable only in one position of rail points.
    #[must_use]
    pub fn with_gate(mut self, gate: Gate) -> Self {
        self.gate = Some(gate);
        self
    }

    /// Total length (m).
    #[must_use]
    pub fn length(&self) -> f32 {
        self.cum.last().copied().unwrap_or(0.0)
    }

    /// Position and heading (radians, clockwise from north) at arc length `s` (clamped).
    #[must_use]
    pub fn at(&self, s: f32) -> (Vec3, f32) {
        let s = s.clamp(0.0, self.length());
        let i = self
            .cum
            .partition_point(|c| *c <= s)
            .clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[i - 1], self.points[i]);
        let seg = (self.cum[i] - self.cum[i - 1]).max(1e-6);
        let t = (s - self.cum[i - 1]) / seg;
        let d = b - a;
        (a + d * t, d.x.atan2(d.y))
    }

    /// Start (`false`) or end (`true`) point.
    #[must_use]
    pub fn end_point(&self, end: bool) -> Vec3 {
        if end {
            self.points[self.points.len() - 1]
        } else {
            self.points[0]
        }
    }
}

/// Radii of `.sco` `[path]` records at or beyond this magnitude are the straight marker
/// (`-999.99`); splines arc for every radius, like their mesh.
pub const SCO_STRAIGHT_RADIUS: f32 = 999.0;

/// Centreline of an arc segment: `(position, heading_rad)` every [`SAMPLE_STEP_M`] metres.
///
/// Same arc as `opts_bevy::scenery::spline_pose` (heading clockwise from north; a positive
/// radius turns right; `0` is straight); `height(s)` is the elevation above the start. The chord
/// form keeps very large radii exact in `f32`.
fn arc(
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
/// is passable only in that position of the points `switch` (see [`super::switches`]).
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

#[cfg(test)]
mod tests;
