//! Turns placed splines and objects of a map tile into [`LaneSegment`]s.
//!
//! * A **spline** carries its lanes in its `.sli` profile (`[path]`: type, lateral offset,
//!   width, direction); the lane follows the spline's centre line shifted sideways by the offset.
//! * A **scenery object** (junction, crossing, bus stop) carries object-local `[path]` records in
//!   its `.sco`; they are rotated clockwise by the object's heading and moved to its position.
//!
//! The host supplies the parsed `.sli`/`.sco` through [`PathSource`] (it already caches them);
//! this module does no I/O. Splines and paths whose length is not a positive number are skipped
//! (the installed maps contain a few with negative lengths such as `-9.11`; what they mean is an
//! open question). Speed limits and priorities (`[rule]`) are not applied yet: the tile
//! parser keeps the rules apart from the spline they belong to
//! (`docs/src/architecture/ai-traffic.md`, "Gaps in `opts-core`").

use glam::DVec2;
use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::{ScoPath, SplinePath};

use crate::geometry::PathGeometry;
use crate::graph::{Direction, LaneKind, LaneSegment};

/// Tile edge length in metres: tile `(x, y)` starts at `(x · 300, y · 300)`.
pub const TILE_SIZE_M: f64 = 300.0;

/// World origin of tile `(x, y)`.
#[must_use]
pub fn tile_origin(x: i32, y: i32) -> DVec2 {
    DVec2::new(f64::from(x), f64::from(y)) * TILE_SIZE_M
}

/// Lookup of parsed path data by the file name written in the tile.
pub trait PathSource {
    /// The `[path]` records of a `.sli` profile; `None` when the file is unknown or unreadable.
    fn sli_paths(&mut self, sli_file: &str) -> Option<&[SplinePath]>;
    /// The `[path]` records of a `.sco`; `None` when the file is unknown or unreadable.
    fn sco_paths(&mut self, sco_file: &str) -> Option<&[ScoPath]>;
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
    let mut out = Vec::new();
    for s in &tile.splines {
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
        for p in paths {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            out.push(LaneSegment {
                geometry: centre.offset(f64::from(p.offset_x)),
                kind,
                width: f64::from(p.width),
                direction: Direction::from_number(p.direction),
                speed_limit: None,
                source: s.id_code,
            });
        }
    }
    for o in &tile.objects {
        let Some(paths) = source.sco_paths(&o.sco_file) else {
            continue;
        };
        let position = origin + DVec2::new(f64::from(o.position.x), f64::from(o.position.y));
        let heading = f64::from(o.heading_deg).to_radians();
        for p in paths {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            if !(p.length_m.is_finite() && p.length_m > 0.0) {
                continue;
            }
            let local = PathGeometry {
                start: DVec2::new(f64::from(p.start.x), f64::from(p.start.y)),
                heading: f64::from(p.heading_deg).to_radians(),
                radius: f64::from(p.radius_m),
                length: f64::from(p.length_m),
            };
            out.push(LaneSegment {
                geometry: local.to_world(position, heading),
                kind,
                width: f64::from(p.width_m),
                direction: Direction::from_number(p.direction),
                speed_limit: None,
                source: o.id_code,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ConnectParams, LaneGraph};
    use glam::Vec3;
    use opts_core::formats::map::{MapObjectInstance, MapSplineInstance};

    struct Fixed {
        sli: Vec<SplinePath>,
        sco: Vec<ScoPath>,
    }

    impl PathSource for Fixed {
        fn sli_paths(&mut self, _: &str) -> Option<&[SplinePath]> {
            Some(&self.sli)
        }
        fn sco_paths(&mut self, _: &str) -> Option<&[ScoPath]> {
            Some(&self.sco)
        }
    }

    fn spline_path(offset_x: f32, direction: u32) -> SplinePath {
        SplinePath {
            path_type: 0,
            offset_x,
            offset_z: 0.0,
            width: 2.5,
            direction,
            extra: None,
        }
    }

    fn spline(y: f32, len: f32) -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: "road.sli".into(),
            id_code: 7,
            conn_id_1: 0,
            conn_id_2: 0,
            position: Vec3::new(0.0, y, 0.0),
            heading_deg: 0.0,
            length_m: len,
            radius_m: 0.0,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: Vec::new(),
        }
    }

    fn sco_path(x: f32, y: f32) -> ScoPath {
        ScoPath {
            start: Vec3::new(x, y, 0.0),
            heading_deg: 0.0,
            radius_m: -999.99,
            length_m: 20.0,
            slope_start: 0.0,
            slope_end: 0.0,
            path_type: 0,
            width_m: 2.5,
            direction: 0,
            extra: None,
            traffic_light_index: None,
            approach_distance_m: None,
        }
    }

    #[test]
    fn a_two_lane_road_runs_into_a_junction_object_rotated_with_the_object() {
        // Road 0..100 m north with a forward lane (right, +1.75) and a reverse lane (left).
        let road = spline(0.0, 100.0);
        // A junction arm placed at the road's end, turned 0°: local path from (1.75, 0) north.
        let object = MapObjectInstance {
            flag: 0,
            sco_file: "junction.sco".into(),
            id_code: 9,
            position: Vec3::new(0.0, 100.0, 0.0),
            heading_deg: 0.0,
            pitch_deg: None,
            roll_deg: None,
            string_vars: Vec::new(),
        };
        let tile = MapTile {
            splines: vec![road],
            objects: vec![object],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![spline_path(1.75, 0), spline_path(-1.75, 1)],
            sco: vec![sco_path(1.75, 0.0)],
        };
        let segs = segments_from_tile(&tile, tile_origin(1, 0), &mut src);
        assert_eq!(segs.len(), 3);
        // Tile (1, 0) starts 300 m east; the right lane sits 1.75 m east of the centre line.
        assert!(segs[0]
            .geometry
            .start
            .abs_diff_eq(DVec2::new(301.75, 0.0), 1e-9));
        assert_eq!(segs[0].source, 7);
        assert_eq!(segs[2].source, 9);
        let g = LaneGraph::build(&segs, &ConnectParams::default());
        // 0 (forward lane) → 2 (junction arm); the reverse lane flows the other way.
        assert_eq!(
            g.successors(crate::graph::LaneId(0)),
            &[crate::graph::LaneId(2)]
        );
        assert!(g.successors(crate::graph::LaneId(1)).is_empty());
    }

    #[test]
    fn non_positive_lengths_are_skipped() {
        let tile = MapTile {
            splines: vec![spline(0.0, -9.11), spline(0.0, 0.0)],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![spline_path(0.0, 0)],
            sco: Vec::new(),
        };
        assert!(segments_from_tile(&tile, DVec2::ZERO, &mut src).is_empty());
    }
}
