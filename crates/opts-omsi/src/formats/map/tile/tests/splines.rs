//! Spline records: version 10 connections, unconnected splines and terrain alignment.

use super::super::*;

/// MI-0021: a version 10 tile has one connection field; `-1` places the spline, `0` continues
/// the previous one (all pose fields are zero).
#[test]
fn test_version_10_splines_are_placed_or_continue_the_previous() {
    let rec = |id: u32, conn: &str, x: &str, y: &str, h: &str, len: &str| {
        format!(
            "[spline]\n0\nSplines\\a.sli\n{id}\n{conn}\n{x}\n0\n{y}\n{h}\n{len}\n0\n0\n0\n0\n0\n\n"
        )
    };
    let text = format!(
        "File created\n\n[version]\n10\n\n{}{}",
        rec(1, "-1", "100", "200", "90", "30"),
        rec(2, "0", "0", "0", "0", "10")
    ) + "[rule]\n0\ntrafficdensity\n3.000\n\n[rule]\n1\nspeedlimit\n30\nObject Nr. 0\n";
    let tile = MapTile::from_text(&text).expect("v10 tile parses");
    assert_eq!(tile.rules.len(), 2, "v10 rules have no flag line");
    assert_eq!(tile.rules[1].flag, 0);
    assert_eq!(tile.splines.len(), 2);
    assert_eq!(tile.splines[0].position.x, 100.0);
    assert_eq!(tile.splines[0].position.y, 200.0);
    assert_eq!(tile.splines[0].length_m, 30.0);
    // heading 90 degrees runs towards +x: the second spline starts 30 m east of the first
    let second = &tile.splines[1];
    assert!(
        (second.position.x - 130.0).abs() < 1e-3,
        "{:?}",
        second.position
    );
    assert!((second.position.y - 200.0).abs() < 1e-3);
    assert!((second.heading_deg - 90.0).abs() < 1e-3);
    assert_eq!(second.length_m, 10.0);
}

#[test]
fn test_spline_connection_minus_one_means_unconnected() {
    let text =
        "[version]\n14\n\n[spline]\n0\nSplines\\a.sli\n77\n-1\n-1\n1.0\n2.0\n3.0\n0.0\n10.0\n0.0\n";
    let tile = MapTile::from_text(text).expect("-1 connections parse");
    assert_eq!(tile.splines.len(), 1);
    assert_eq!(tile.splines[0].conn_id_1, 0);
    assert_eq!(tile.splines[0].conn_id_2, 0);
}

#[test]
fn terrain_align_marks_the_spline_before_it() {
    let spline = |id: u32| {
        format!(
            "[spline]\n0\nSplines\\a.sli\n{id}\n0\n0\n1.0\n-12.0\n3.0\n0.0\n10.0\n0.0\n0\n0\n\n"
        )
    };
    let text = format!(
        "[version]\n14\n\n{}[spline_terrain_align_2]\n1\n\n{}[spline_terrain_align]\n0\n\n{}",
        spline(12663),
        spline(12664),
        spline(12665)
    );
    let tile = MapTile::from_text(&text).expect("aligned splines parse");
    assert_eq!(tile.splines.len(), 3);
    assert_eq!(tile.terrain_aligned, vec![12663], "value 0 is no alignment");
}
