//! Unit tests of `source.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

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
        switch_position: None,
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
    let segs = segments_from_tile(&tile, tile_origin(&TileGrid::default(), 1, 0), &mut src);
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
        flag: u32::from(rule_type == "trafficdensity" && value > 1.0) * 4,
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
            rule(me, 1, "trafficdensity", 3.0),
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
    assert!(segs[0].rules.no_overtaking && segs[0].rules.density.is_empty());
    // Path 1: 36 km/h = 10 m/s, density 0.5 for group 0 and 3 (unclamped) for group 4 (the
    // rule's flag), priority 192, trucks.
    assert!((segs[1].speed_limit.unwrap() - 10.0).abs() < 1e-9);
    assert_eq!(
        (
            segs[1].rules.density.get(0),
            segs[1].rules.density.get(4),
            segs[1].rules.priority
        ),
        (Some(0.5), Some(3.0), Some(192))
    );
    assert_eq!(segs[1].rules.density.get(1), None);
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
fn path_parameter_12_gives_the_indicator() {
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
    let path = |extra| ScoPath {
        extra,
        ..sco_path(0.0, 0.0)
    };
    let mut src = Fixed {
        sli: Vec::new(),
        sco: [None, Some(0), Some(1), Some(2), Some(3)]
            .into_iter()
            .map(path)
            .collect(),
    };
    let segs = segments_from_tile(&tile, DVec2::ZERO, &mut src);
    let got: Vec<_> = segs.iter().map(|s| s.indicator).collect();
    assert_eq!(
        got,
        [
            None,
            Some(Blinker::Off),
            None,
            Some(Blinker::Left),
            Some(Blinker::Right)
        ]
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
