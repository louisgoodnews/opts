//! Object paths (junction `.sco` arms): straight markers, arcs, indicators and skipped lengths.

use super::*;
use crate::agent::Blinker;

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
