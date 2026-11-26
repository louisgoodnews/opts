//! The geometry of the zone overlay (M-0115): the corners and the twelve edges of a zone box, as
//! plain vectors. The gizmo draw and the colours stay in `opts-spatial-zones` (ISS-092).

use glam::{Vec2, Vec3};

/// The floor of a zone box (m above the ground reference).
pub const ZONE_BOTTOM_M: f32 = 0.0;
/// The roof of a zone box drawn for orientation (m); zones have no height in the model.
pub const ZONE_TOP_M: f32 = 4.0;
/// Radius of the subject marker (m).
pub const SUBJECT_RADIUS_M: f32 = 0.6;

/// The four ground corners of the box `min`..`max` at height `y`, in the order south-west,
/// south-east, north-east, north-west. The ground point of a zone is `(x, z)` of the Bevy
/// transform, the same point the sampler tests, so a corner `c` satisfies
/// `zone.contains(Vec2::new(c.x, c.z))` for the zone that owns it.
#[must_use]
pub fn ground_corners(min: Vec2, max: Vec2, y: f32) -> [Vec3; 4] {
    [
        Vec3::new(min.x, y, min.y),
        Vec3::new(max.x, y, min.y),
        Vec3::new(max.x, y, max.y),
        Vec3::new(min.x, y, max.y),
    ]
}

/// The twelve edges of the box `min`..`max`: the floor and the roof rectangles and the four
/// verticals.
#[must_use]
pub fn box_edges(min: Vec2, max: Vec2) -> [(Vec3, Vec3); 12] {
    let floor = ground_corners(min, max, ZONE_BOTTOM_M);
    let roof = ground_corners(min, max, ZONE_TOP_M);
    let mut edges = [(Vec3::ZERO, Vec3::ZERO); 12];
    for i in 0..4 {
        let next = (i + 1) % 4;
        edges[i] = (floor[i], floor[next]);
        edges[4 + i] = (roof[i], roof[next]);
        edges[8 + i] = (floor[i], roof[i]);
    }
    edges
}
