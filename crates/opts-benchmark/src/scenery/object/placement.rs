//! Where the tile places an object, where the analysis expects it, and the views framed on it.

use super::*;

/// Where the tile places the object and where the analysis expects it (they differ only for
/// seeded faults).
pub(super) fn placements(fault: Option<&str>) -> (mapgen::Placement, Placement) {
    let mut tile = mapgen::Placement::default();
    let mut expect = Placement {
        position: OBJECT_POS,
        heading_deg: OBJECT_HEADING_DEG,
        mirror_x: false,
        scale: 1.0,
    };
    match fault {
        Some("heading-mirror") => tile.heading_deg = -OBJECT_HEADING_DEG,
        Some("offset-10m") => tile.position.x += 10.0,
        Some("scale-2x") => expect.scale = 0.5,
        _ => {}
    }
    (tile, expect)
}

/// The views, framed on the expected geometry (a 20 m box when there is none).
pub(super) fn frame(exp: &Expected, placement: &Placement) -> (Bounds, [View; 2]) {
    let placed = exp
        .tris
        .iter()
        .flat_map(|t| t.v)
        .map(|v| placement.to_map(v));
    let bounds = Bounds::of(placed).unwrap_or_else(|| Bounds {
        min: placement.position + Vec3::new(-10.0, -10.0, 0.0),
        max: placement.position + Vec3::new(10.0, 10.0, 20.0),
    });
    let views = [
        oblique_view(&bounds, placement.heading_deg),
        top_view(&bounds),
    ];
    (bounds, views)
}
