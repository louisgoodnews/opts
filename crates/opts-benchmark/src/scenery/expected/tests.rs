//! Unit tests for `super` (`expected.rs`).

use super::*;

#[test]
fn heading_turns_forward_clockwise_from_north() {
    let p = Placement {
        position: Vec3::new(150.0, 150.0, 0.0),
        heading_deg: 90.0,
        mirror_x: false,
        scale: 1.0,
    };
    // Heading 90°: forward (+z) points east, right (+x) points south, up stays up.
    let fwd = p.to_map(Vec3::new(0.0, 0.0, 1.0)) - p.position;
    let right = p.to_map(Vec3::new(1.0, 0.0, 0.0)) - p.position;
    let up = p.to_map(Vec3::new(0.0, 1.0, 0.0)) - p.position;
    assert!((fwd - Vec3::X).length() < 1e-5);
    assert!((right + Vec3::Y).length() < 1e-5);
    assert!((up - Vec3::Z).length() < 1e-5);
}

#[test]
fn tree_billboard_has_four_triangles_of_the_given_size() {
    let t = tree_billboard(10.0, 4.0);
    assert_eq!(t.len(), 4);
    let top = t.iter().flat_map(|t| t.v).map(|v| v.y).fold(0.0, f32::max);
    assert!((top - 10.0).abs() < 1e-6);
}
