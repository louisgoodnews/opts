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
        })
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

/// Centreline of an arc segment: `(position, heading_rad)` every [`SAMPLE_STEP_M`] metres.
///
/// Same arc as `opts_bevy::scenery::spline_pose` (heading clockwise from north; a positive
/// radius turns right); `height(s)` is the elevation above the start.
fn arc(
    start: Vec3,
    heading_deg: f32,
    radius: f32,
    length: f32,
    height: impl Fn(f32) -> f32,
) -> Vec<(Vec3, f32)> {
    let phi = heading_deg.to_radians();
    let steps = ((length / SAMPLE_STEP_M).ceil() as usize).clamp(1, 2000);
    (0..=steps)
        .map(|k| {
            let s = length * k as f32 / steps as f32;
            let (h, x, y) = if radius.abs() > 1e-3 && radius.abs() < 900.0 {
                let h = phi + s / radius;
                (
                    h,
                    start.x - radius * (h.cos() - phi.cos()),
                    start.y + radius * (h.sin() - phi.sin()),
                )
            } else {
                (phi, start.x + s * phi.sin(), start.y + s * phi.cos())
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
/// bike heuristic of [`SplinePath::kind`]); same geometry rules as [`lanes_from_spline`].
#[must_use]
pub fn lanes_of_kind(
    inst: &MapSplineInstance,
    profile: &SplineProfile,
    name: &str,
    shift: (f32, f32),
    kind: LaneKind,
) -> Vec<FootLane> {
    let length = inst.length_m.max(0.01);
    let explicit = inst
        .raw_params
        .get(2)
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|h| inst.has_height_profile && h.abs() > 1e-6);
    let height = |s: f32| match explicit {
        Some(h) => {
            let t = (s / length).clamp(0.0, 1.0);
            let (m0, m1) = (inst.gradient_start * length, inst.gradient_end * length);
            let (t2, t3) = (t * t, t * t * t);
            (t3 - 2.0 * t2 + t) * m0 + (-2.0 * t3 + 3.0 * t2) * h + (t3 - t2) * m1
        }
        None => {
            inst.gradient_start * s
                + (inst.gradient_end - inst.gradient_start) * s * s / (2.0 * length)
        }
    };
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
            let local = arc(
                Vec3::new(p.start.x, p.start.y, 0.0),
                p.heading_deg,
                p.radius_m,
                length,
                |s| s0 * s + (s1 - s0) * s * s / (2.0 * length),
            );
            let points = local
                .iter()
                .map(|(c, _)| to_world(Vec3::new(c.x, c.y, c.z + p.start.z)))
                .collect();
            FootLane::new(points, Travel::from_field(p.direction), p.width_m)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_core::formats::scenery::SplinePath;

    fn spline(heading: f32, radius: f32, length: f32) -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: 1,
            conn_id_1: 0,
            conn_id_2: 0,
            position: Vec3::new(10.0, 20.0, 5.0),
            heading_deg: heading,
            length_m: length,
            radius_m: radius,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: Vec::new(),
        }
    }

    fn profile(offset: f32) -> SplineProfile {
        SplineProfile {
            paths: vec![
                SplinePath {
                    path_type: 0,
                    offset_x: -2.5,
                    offset_z: 0.1,
                    width: 3.0,
                    direction: 0,
                    extra: None,
                },
                SplinePath {
                    path_type: 1,
                    offset_x: offset,
                    offset_z: 0.05,
                    width: 0.6,
                    direction: 2,
                    extra: None,
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn only_pedestrian_paths_become_lanes_right_of_travel() {
        // Heading north: right is east (+x).
        let lanes = lanes_from_spline(&spline(0.0, 0.0, 20.0), &profile(2.2), (0.0, 0.0));
        assert_eq!(lanes.len(), 1);
        let l = &lanes[0];
        assert_eq!(l.travel, Travel::Both);
        assert!((l.points[0] - Vec3::new(12.2, 20.0, 5.05)).length() < 1e-4);
        assert!((l.length() - 20.0).abs() < 1e-3);
        let (_, h) = l.at(5.0);
        assert!(h.abs() < 1e-4);
    }

    #[test]
    fn bike_lanes_of_a_composite_are_found_by_kind() {
        let mut p = profile(2.2);
        p.paths.push(SplinePath {
            path_type: 0,
            offset_x: 3.0,
            offset_z: 0.05,
            width: 0.1,
            direction: 2,
            extra: None,
        });
        let bikes = lanes_of_kind(
            &spline(0.0, 0.0, 20.0),
            &p,
            "x.sli",
            (0.0, 0.0),
            LaneKind::Bike,
        );
        assert_eq!(bikes.len(), 1);
        assert!((bikes[0].points[0].x - 13.0).abs() < 1e-4);
        // The 3 m road lane is a road, the footpath still a footpath.
        assert_eq!(
            lanes_of_kind(
                &spline(0.0, 0.0, 20.0),
                &p,
                "x.sli",
                (0.0, 0.0),
                LaneKind::Road
            )
            .len(),
            1
        );
        assert_eq!(
            lanes_from_spline(&spline(0.0, 0.0, 20.0), &p, (0.0, 0.0)).len(),
            1
        );
    }

    #[test]
    fn right_turn_lane_is_shorter_and_ends_where_the_arc_does() {
        // Radius +30 turns right (clockwise): a path 2.2 m to the right is on the inside.
        let inst = spline(0.0, 30.0, 30.0 * std::f32::consts::FRAC_PI_2);
        let lanes = lanes_from_spline(&inst, &profile(2.2), (0.0, 0.0));
        let l = &lanes[0];
        let expected = (30.0 - 2.2) * std::f32::consts::FRAC_PI_2;
        assert!(
            (l.length() - expected).abs() < 0.1,
            "{} vs {expected}",
            l.length()
        );
        let end = l.end_point(true);
        // Centreline end after the quarter turn: (40, 50) heading east; the lane is 2.2 m to its
        // right (south).
        assert!(
            (end - Vec3::new(40.0, 47.8, 5.05)).length() < 0.1,
            "{end:?}"
        );
        let (_, h) = l.at(l.length());
        assert!((h - std::f32::consts::FRAC_PI_2).abs() < 0.05);
    }

    #[test]
    fn object_paths_rotate_clockwise_with_the_object() {
        let path = ScoPath {
            start: Vec3::new(1.0, 0.0, 0.2),
            heading_deg: 0.0,
            radius_m: -999.99,
            length_m: 10.0,
            slope_start: 0.0,
            slope_end: 0.0,
            path_type: 1,
            width_m: 1.0,
            direction: 0,
            extra: None,
            traffic_light_index: None,
            approach_distance_m: None,
        };
        // Object turned 90° clockwise: local north becomes east, local east becomes south.
        let lanes = lanes_from_object(&[path], Vec3::new(100.0, 200.0, 3.0), 90.0);
        let l = &lanes[0];
        assert!((l.points[0] - Vec3::new(100.0, 199.0, 3.2)).length() < 1e-3);
        assert!((l.end_point(true) - Vec3::new(110.0, 199.0, 3.2)).length() < 1e-3);
    }

    #[test]
    fn position_at_follows_the_polyline() {
        let l = FootLane::new(
            vec![
                Vec3::ZERO,
                Vec3::new(0.0, 10.0, 0.0),
                Vec3::new(10.0, 10.0, 0.0),
            ],
            Travel::Forward,
            1.0,
        )
        .unwrap();
        let (p, h) = l.at(15.0);
        assert!((p - Vec3::new(5.0, 10.0, 0.0)).length() < 1e-4);
        assert!((h - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }
}
