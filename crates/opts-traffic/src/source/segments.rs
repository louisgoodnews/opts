//! Lane segments of a tile's splines and objects: geometry, kind, rules and elevation per path.

use glam::DVec2;

use super::{rules_for, MapTile, PathSource, RuleOwner, ScoPath};
use crate::agent::Blinker;
use crate::geometry::{PathGeometry, SCO_STRAIGHT_RADIUS};
use crate::graph::{Direction, Elevation, LaneKind, LaneSegment};

fn spline_segments(
    tile: &MapTile,
    origin: DVec2,
    source: &mut impl PathSource,
) -> Vec<LaneSegment> {
    let mut out = Vec::new();
    for (si, s) in tile.splines.iter().enumerate() {
        if !(s.length_m.is_finite() && s.length_m > 0.0) {
            continue;
        }
        let Some(paths) = source.sli_paths(&s.sli_file) else {
            continue;
        };
        let centre = PathGeometry {
            start: origin + DVec2::new(f64::from(s.position.x), f64::from(s.position.y)),
            heading: f64::from(s.heading_deg).to_radians(),
            radius: f64::from(s.radius_m),
            length: f64::from(s.length_m),
        };
        let owner = RuleOwner::Spline {
            index: si,
            id_code: s.id_code,
        };
        let mirror = if s.mirrored { -1.0 } else { 1.0 };
        for (pi, p) in paths.iter().enumerate() {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            let (speed_limit, rules) = rules_for(&tile.rules, owner, pi);
            let geometry = centre.offset(mirror * f64::from(p.offset_x));
            let scale = centre.length / geometry.length.max(1e-6);
            out.push(LaneSegment {
                geometry,
                kind,
                width: f64::from(p.width),
                direction: if s.mirrored {
                    Direction::from_number(p.direction).reversed()
                } else {
                    Direction::from_number(p.direction)
                },
                speed_limit,
                rules,
                elevation: Elevation {
                    z0: f64::from(s.position.z) + f64::from(p.offset_z),
                    grad_start: f64::from(s.gradient_start) * scale,
                    grad_end: f64::from(s.gradient_end) * scale,
                    rise: s.explicit_rise().map(f64::from),
                },
                signal: None,
                junction: false,
                indicator: None,
                source: s.id_code,
            });
        }
    }
    out
}

fn object_segments(
    tile: &MapTile,
    origin: DVec2,
    source: &mut impl PathSource,
) -> Vec<LaneSegment> {
    let mut out = Vec::new();
    for (oi, o) in tile.objects.iter().enumerate() {
        if source
            .sco_paths(&o.sco_file)
            .is_none_or(<[ScoPath]>::is_empty)
        {
            continue;
        }
        let base_height = source.object_base_height(o);
        let Some(paths) = source.sco_paths(&o.sco_file) else {
            continue;
        };
        let position = origin + DVec2::new(f64::from(o.position.x), f64::from(o.position.y));
        let heading = f64::from(o.heading_deg).to_radians();
        let owner = RuleOwner::Object {
            index: oi,
            id_code: o.id_code,
        };
        for (pi, p) in paths.iter().enumerate() {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            if !(p.length_m.is_finite() && p.length_m > 0.0) {
                continue;
            }
            let local = PathGeometry {
                start: DVec2::new(f64::from(p.start.x), f64::from(p.start.y)),
                heading: f64::from(p.heading_deg).to_radians(),
                radius: match f64::from(p.radius_m) {
                    r if r.abs() >= SCO_STRAIGHT_RADIUS => 0.0,
                    r => r,
                },
                length: f64::from(p.length_m),
            };
            let (speed_limit, rules) = rules_for(&tile.rules, owner, pi);
            out.push(LaneSegment {
                geometry: local.to_world(position, heading),
                kind,
                width: f64::from(p.width_m),
                direction: Direction::from_number(p.direction),
                speed_limit,
                rules,
                elevation: Elevation {
                    z0: base_height + f64::from(p.start.z),
                    grad_start: f64::from(p.slope_start),
                    grad_end: f64::from(p.slope_end),
                    rise: None,
                },
                signal: p.traffic_light_index.map(|i| (o.id_code, i)),
                junction: true,
                indicator: match p.extra {
                    Some(0) => Some(Blinker::Off),
                    Some(2) => Some(Blinker::Left),
                    Some(3) => Some(Blinker::Right),
                    _ => None,
                },
                source: o.id_code,
            });
        }
    }
    out
}

/// Lane segments of every spline and object of `tile`, which sits at `origin`.
///
/// Pedestrian and rail paths are included (see [`LaneKind`]); filter by kind when building a
/// road-only graph.
pub fn segments_from_tile(
    tile: &MapTile,
    origin: DVec2,
    source: &mut impl PathSource,
) -> Vec<LaneSegment> {
    let mut out = spline_segments(tile, origin, source);
    out.extend(object_segments(tile, origin, source));
    out
}
