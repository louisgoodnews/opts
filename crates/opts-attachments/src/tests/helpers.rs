//! Shared builders of the attachment tests: splines, attachments and the spline index.

use super::super::*;

pub(super) fn spline(id: u64, prev: u64, next: u64, x: f32, len: f32) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: id,
        conn_id_1: prev,
        conn_id_2: next,
        position: Vec3::new(x, 10.0, 0.0),
        heading_deg: 90.0,
        length_m: len,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: vec![],
    }
}

pub(super) fn attachment(index: usize, lateral: f32, d: f32) -> MapSplineAttachment {
    MapSplineAttachment {
        sco_file: "a.sco".into(),
        id_code: 1,
        spline_index: index,
        lateral_m: lateral,
        height_m: 0.25,
        distance_m: d,
        heading_deg: 180.0,
        field8: 0.0,
        field9: 0.0,
        spacing_m: 0.0,
        range_m: 0.0,
        field12: 0.0,
        repeater: None,
        string_vars: vec![],
    }
}

pub(super) fn run(index: usize, d: f32, spacing: f32, range: f32) -> MapSplineAttachment {
    MapSplineAttachment {
        spacing_m: spacing,
        range_m: range,
        ..attachment(index, 0.0, d)
    }
}

pub(super) fn index_of(tiles: &[((i32, i32), Vec<MapSplineInstance>)]) -> SplineIndex {
    let mut index = SplineIndex::default();
    for (key, splines) in tiles {
        index.add_tile(*key, splines);
    }
    index.finish();
    index
}

pub(super) fn chain() -> (Vec<MapSplineInstance>, SplineIndex) {
    let splines = vec![spline(1, 0, 2, 0.0, 50.0), spline(2, 1, 0, 50.0, 100.0)];
    let index = index_of(&[((0, 0), splines.clone())]);
    (splines, index)
}

pub(super) fn xs(out: &[(MapObjectInstance, f32)]) -> Vec<f32> {
    out.iter().map(|o| o.0.position.x).collect()
}

pub(super) fn expand(
    atts: &[MapSplineAttachment],
    splines: &[MapSplineInstance],
    index: &SplineIndex,
    tile: (i32, i32),
) -> Vec<(MapObjectInstance, f32)> {
    expand_attachments(atts, splines, index, tile, &TileGrid::default(), |_, _| 0.0)
}
