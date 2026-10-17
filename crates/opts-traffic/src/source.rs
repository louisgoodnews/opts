//! Turns placed splines and objects of a map tile into [`LaneSegment`]s.
//!
//! * A **spline** carries its lanes in its `.sli` profile (`[path]`: type, lateral offset,
//!   width, direction); the lane follows the spline's centre line shifted sideways by the offset
//!   (negated on a `mirror`ed spline, like the mesh). Spline lanes arc for every radius and take
//!   the height profile of the mesh (`[spline_h]` ramp).
//! * A **scenery object** (junction, crossing, bus stop) carries object-local `[path]` records in
//!   its `.sco`; they are rotated clockwise by the object's heading and moved to its position.
//!
//! The host supplies the parsed `.sli`/`.sco` through [`PathSource`] (it already caches them);
//! this module does no I/O. The tile's `[rule]` records are applied to the lane they address (owner
//! element and path index in file order; speed limits are km/h in the file). Splines and paths
//! whose length is not a positive number are skipped (the installed maps contain a few with
//! negative lengths such as `-9.11`; what they mean is an open question). Spline arms embedded in
//! objects with `[splinehelper]` are not converted yet: rule indices of such objects continue
//! into the helpers' paths (`docs/src/formats/map/tiles.md`).

use glam::DVec2;
use opts_core::formats::map::{MapObjectInstance, MapTile, MapTrafficRule, RuleKind, RuleOwner};
use opts_core::formats::scenery::{ScoPath, ScoTrafficLightGroup, SplinePath};

use crate::geometry::{PathGeometry, SCO_STRAIGHT_RADIUS};
use crate::graph::{Direction, Elevation, LaneKind, LaneRules, LaneSegment};
use crate::signals::{SignalKey, SignalProgram};

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
    /// Height of the object's origin in metres. The default is the position's `z` as written in
    /// the tile; hosts add the terrain height for objects that are not stored at absolute height
    /// (`[absheight]`, crossings with spline helpers).
    fn object_base_height(&mut self, object: &MapObjectInstance) -> f64 {
        f64::from(object.position.z)
    }
    /// The light programs of a `.sco` (`None` when the file is unknown).
    fn sco_lights(&mut self, _sco_file: &str) -> Option<&[ScoTrafficLightGroup]> {
        None
    }
}

/// The light programs of every object of `tile`, keyed like [`LaneSegment::signal`]: the object's
/// `id_code` and the program's index in declaration order across its groups.
pub fn signals_from_tile(
    tile: &MapTile,
    source: &mut impl PathSource,
) -> Vec<(SignalKey, SignalProgram)> {
    let mut out = Vec::new();
    for o in &tile.objects {
        let Some(groups) = source.sco_lights(&o.sco_file) else {
            continue;
        };
        let programs = groups.iter().flat_map(|g| {
            g.programs
                .iter()
                .map(move |p| (f64::from(g.period_sec()), p))
        });
        for (i, (period, p)) in programs.enumerate() {
            let phases = p
                .phases
                .iter()
                .map(|ph| (ph.state, f64::from(ph.duration_sec)))
                .collect();
            let approach = f64::from(p.approach_distance_m.unwrap_or(0.0));
            if let Some(program) = SignalProgram::in_period(phases, period, approach) {
                out.push(((o.id_code, i), program));
            }
        }
    }
    out
}

/// `speedlimit` values (km/h) from this on mean "no limit".
pub const NO_LIMIT_KMH: f64 = 200.0;

/// Folds the rules that address path `path_index` of `owner` into a speed limit (m/s) and the
/// other lane rules.
fn rules_for(
    rules: &[MapTrafficRule],
    owner: RuleOwner,
    path_index: usize,
) -> (Option<f64>, LaneRules) {
    let mut speed = None;
    let mut out = LaneRules::default();
    for r in rules
        .iter()
        .filter(|r| r.owner == Some(owner) && r.path_index == path_index)
    {
        let value = f64::from(r.value);
        match r.kind() {
            // km/h in the file; `0` would stop all traffic, so it counts as "no limit" ❓, and so
            // do 200 km/h and more (the editor's "no limit" is 250, Q-AK-02).
            Some(RuleKind::SpeedLimit) if value > 0.0 && value < NO_LIMIT_KMH => {
                speed = Some(value / 3.6);
            }
            Some(RuleKind::TrafficDensity) => out.density = Some(value),
            Some(RuleKind::Priority) => out.priority = Some(r.value.max(0.0) as u32),
            Some(RuleKind::Trucks) => out.trucks = true,
            Some(RuleKind::Bus) => out.bus = true,
            Some(RuleKind::NoCars) => out.no_cars = true,
            Some(RuleKind::OvertakingProhibited) => out.no_overtaking = true,
            _ => {}
        }
    }
    (speed, out)
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
        // A mirrored spline turns its profile round: the lanes change sides *and* direction, so
        // a right-hand road stays right-hand (on Berlin-Spandau the lanes then join up: 178 dead
        // ends against 336 with the offsets negated alone and 279 with `mirror` ignored).
        let mirror = if s.mirrored { -1.0 } else { 1.0 };
        for (pi, p) in paths.iter().enumerate() {
            let Some(kind) = LaneKind::from_path_type(p.path_type) else {
                continue;
            };
            let (speed_limit, rules) = rules_for(&tile.rules, owner, pi);
            let geometry = centre.offset(mirror * f64::from(p.offset_x));
            // The height follows the centre line at the same fraction of the length, so the
            // gradients scale with the lane's length on an arc.
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
                source: s.id_code,
            });
        }
    }
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
    fn rules_apply_to_the_addressed_path_of_their_owner() {
        use opts_core::formats::map::MapTrafficRule;
        let rule = |owner, path_index, rule_type: &str, value| MapTrafficRule {
            owner: Some(owner),
            path_index,
            rule_type: rule_type.into(),
            value,
            flag: 0,
        };
        let me = RuleOwner::Spline {
            index: 0,
            id_code: 7,
        };
        let other = RuleOwner::Spline {
            index: 1,
            id_code: 8,
        };
        let tile = MapTile {
            splines: vec![spline(0.0, 100.0)],
            rules: vec![
                rule(me, 1, "speedlimit", 36.0),
                rule(me, 1, "trafficdensity", 0.5),
                rule(me, 1, "priority", 192.0),
                rule(me, 1, "trucks", 0.0),
                rule(me, 0, "overtaking_prohib", 0.0),
                rule(other, 0, "speedlimit", 5.0),
                rule(me, 0, "speedlimit", 0.0),
                rule(me, 0, "speedlimit", 250.0),
            ],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![spline_path(1.75, 0), spline_path(-1.75, 0)],
            sco: Vec::new(),
        };
        let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
        // Path 0: only the overtaking ban; limits of 0 and 250 mean none, the other owner's is
        // ignored.
        assert_eq!(segs[0].speed_limit, None);
        assert!(segs[0].rules.no_overtaking && segs[0].rules.density.is_none());
        // Path 1: 36 km/h = 10 m/s, density 0.5, priority 192, trucks.
        assert!((segs[1].speed_limit.unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(
            (segs[1].rules.density, segs[1].rules.priority),
            (Some(0.5), Some(192))
        );
        assert!(segs[1].rules.trucks && !segs[1].rules.bus && !segs[1].rules.no_overtaking);
    }

    #[test]
    fn rule_indices_count_every_path_even_filtered_ones() {
        // Path 0 is a pedestrian path; the rule addresses path 1, the road lane.
        let mut ped = spline_path(3.0, 0);
        ped.path_type = 1;
        let owner = RuleOwner::Spline {
            index: 0,
            id_code: 7,
        };
        let tile = MapTile {
            splines: vec![spline(0.0, 50.0)],
            rules: vec![MapTrafficRule {
                owner: Some(owner),
                path_index: 1,
                rule_type: "speedlimit".into(),
                value: 72.0,
                flag: 0,
            }],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![ped, spline_path(0.0, 0)],
            sco: Vec::new(),
        };
        let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].speed_limit, None);
        assert!((segs[1].speed_limit.unwrap() - 20.0).abs() < 1e-9);
    }

    /// End of a placed spline's centre line by the mesh formula (`opts_bevy::scenery::spline_pose`).
    fn mesh_end(s: &MapSplineInstance) -> DVec2 {
        let (r, l) = (f64::from(s.radius_m), f64::from(s.length_m));
        let phi = f64::from(s.heading_deg).to_radians();
        let h = phi + l / r;
        DVec2::new(
            f64::from(s.position.x) - r * (h.cos() - phi.cos()),
            f64::from(s.position.y) + r * (h.sin() - phi.sin()),
        )
    }

    #[test]
    fn spline_lanes_arc_for_large_radii_and_end_at_the_mesh_end() {
        let mut road = spline(0.0, 183.0);
        road.radius_m = 1100.0;
        road.heading_deg = 20.0;
        let tile = MapTile {
            splines: vec![road.clone()],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![spline_path(0.0, 0)],
            sco: Vec::new(),
        };
        let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
        let end = segs[0].geometry.end();
        assert!(end.distance(mesh_end(&road)) < 0.01, "{end:?}");
    }

    #[test]
    fn mirrored_splines_put_their_lanes_on_the_other_side() {
        let mut road = spline(0.0, 50.0);
        road.mirrored = true;
        let tile = MapTile {
            splines: vec![road],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![spline_path(1.75, 0)],
            sco: Vec::new(),
        };
        let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
        // Heading north, the profile's +1.75 m (east) becomes 1.75 m west, and the forward lane
        // is driven backwards: right-hand traffic seen from the spline's end.
        assert!(segs[0]
            .geometry
            .start
            .abs_diff_eq(DVec2::new(-1.75, 0.0), 1e-9));
        assert_eq!(segs[0].direction, Direction::Reverse);
        let g = LaneGraph::build(&segs, &ConnectParams::default());
        let lane = g.lane(crate::graph::LaneId(0)).unwrap();
        assert!(lane
            .geometry
            .start
            .abs_diff_eq(DVec2::new(-1.75, 50.0), 1e-9));
    }

    #[test]
    fn spline_h_lanes_follow_the_mesh_ramp_on_inner_and_outer_lanes() {
        let mut road = spline(0.0, 60.0);
        road.radius_m = 40.0;
        road.has_height_profile = true;
        road.gradient_start = 0.01;
        road.gradient_end = 0.03;
        road.raw_params = vec!["0".into(), "0".into(), "1.5".into()];
        let tile = MapTile {
            splines: vec![road.clone()],
            ..MapTile::default()
        };
        let mut src = Fixed {
            sli: vec![spline_path(1.75, 0), spline_path(-1.75, 0)],
            sco: Vec::new(),
        };
        let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
        for seg in &segs {
            let len = seg.geometry.length;
            for k in 0..=6 {
                let t = f64::from(k) / 6.0;
                let mesh = road.height_offset((t * 60.0) as f32);
                let lane = seg.elevation.at(t * len, len);
                assert!(
                    (lane - f64::from(mesh)).abs() < 1e-4,
                    "t {t}: {lane} vs {mesh}"
                );
            }
        }
        // The explicit rise is reached exactly (the linear rule would give 60 · 0.02 = 1.2 m).
        assert!(
            (segs[0]
                .elevation
                .at(segs[0].geometry.length, segs[0].geometry.length)
                - 1.5)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn object_paths_read_the_straight_marker_and_arc_otherwise() {
        let object = MapObjectInstance {
            flag: 0,
            sco_file: "junction.sco".into(),
            id_code: 9,
            position: Vec3::ZERO,
            heading_deg: 0.0,
            pitch_deg: None,
            roll_deg: None,
            string_vars: Vec::new(),
        };
        let tile = MapTile {
            objects: vec![object],
            ..MapTile::default()
        };
        let mut bend = sco_path(0.0, 0.0);
        bend.radius_m = 1200.0;
        let mut src = Fixed {
            sli: Vec::new(),
            sco: vec![sco_path(0.0, 0.0), bend],
        };
        let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
        assert!(segs[0].geometry.is_straight(), "-999.99 is straight");
        assert!(
            segs[1].geometry.is_straight(),
            "|r| >= 999 of an object path"
        );
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
