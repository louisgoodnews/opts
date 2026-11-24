//! Shared builders of the overlay tests: object, spline and rule texts, and the base tile.

use super::super::*;

pub(super) fn object(id: u64, sco: &str, strings: &[&str]) -> String {
    let mut s = format!(
        "[object]\n0\n{sco}\n{id}\n10\n20\n0\n90\n0\n0\n{}\n",
        strings.len()
    );
    for v in strings {
        s.push_str(v);
        s.push('\n');
    }
    s.push_str("Object Nr. 1\n\n");
    s
}

pub(super) fn spline(id: u64, sli: &str) -> String {
    format!("[spline]\n0\n{sli}\n{id}\n0\n0\n0\n0\n0\n0\n50\n0\n0\n0\n0\n0\n0\n0\n\n")
}

pub(super) fn rule(i: usize, t: &str, v: f32, f: u32) -> String {
    format!("[rule]\n{i}\n{t}\n{v}\n{f}\n\n")
}

pub(super) fn base() -> MapTile {
    let text = format!(
        "[version]\n14\n\n[terrain]\n\n{}{}{}{}{}{}[attachObj]\n0\nline.sco\n300\n100\n0\n1\n0\n0\n0\n1\n13N\n\n[splineAttachement]\n0\nbollard.sco\n400\n1\n2\n0\n5\n0\n0\n0\n",
        object(100, "a.sco", &["x", "y"]),
        rule(0, "priority", 1.0, 0),
        spline(10, "road.sli"),
        rule(0, "trafficdensity", 1.0, 0),
        rule(1, "speedlimit", 50.0, 0),
        spline(11, "path.sli"),
    );
    MapTile::from_text(&text).unwrap()
}

pub(super) fn apply(tile: &mut MapTile, overlay: &str) -> OverlayReport {
    tile.apply_overlay(&TileOverlay::from_text(overlay))
}
