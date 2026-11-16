//! Road lanes built from splines: junction arms, arcs, mirrored and height-profiled lanes.

use super::*;
use crate::graph::Direction;

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
