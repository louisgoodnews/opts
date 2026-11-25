//! Tests of the collision boxes that follow a mesh's shape.

use super::*;

/// A quad (two triangles) from corners in order.
fn quad(v: &mut Vec<Vec3>, i: &mut Vec<u32>, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
    let n = v.len() as u32;
    v.extend([a, b, c, d]);
    i.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
}

/// A bridge over a 10 m wide road: two abutment walls 5 m high, a deck at 5 m, 30 m long.
fn bridge() -> (Vec<Vec3>, Vec<u32>) {
    let (mut v, mut i) = (Vec::new(), Vec::new());
    // Deck, top and underside, x -20..20, z 0..30.
    for y in [5.0, 5.8] {
        quad(
            &mut v,
            &mut i,
            Vec3::new(-20.0, y, 0.0),
            Vec3::new(20.0, y, 0.0),
            Vec3::new(20.0, y, 30.0),
            Vec3::new(-20.0, y, 30.0),
        );
    }
    // Abutment walls at x = -20..-5 and 5..20 (their inner faces at +-5), z 0..30.
    for (x0, x1) in [(-20.0, -5.0), (5.0, 20.0)] {
        for x in [x0, x1] {
            quad(
                &mut v,
                &mut i,
                Vec3::new(x, 0.0, 0.0),
                Vec3::new(x, 0.0, 30.0),
                Vec3::new(x, 5.0, 30.0),
                Vec3::new(x, 5.0, 0.0),
            );
        }
    }
    (v, i)
}

#[test]
fn a_tiny_mesh_stays_one_box() {
    let v = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 1.0),
    ];
    assert_eq!(boxes_of(&v, &[0, 1, 2]).len(), 1);
}

/// A street lamp drawn in one plane: a 0.2 m pole 7.5 m high and an arm 3.7 m long at 7.2 m.
/// A bus drives under the arm; the pole stays solid.
#[test]
fn a_street_lamp_blocks_its_pole_not_the_space_under_its_arm() {
    let (mut v, mut i) = (Vec::new(), Vec::new());
    quad(
        &mut v,
        &mut i,
        Vec3::new(-0.1, 0.2, 0.0),
        Vec3::new(0.1, 0.2, 0.0),
        Vec3::new(0.1, 7.7, 0.0),
        Vec3::new(-0.1, 7.7, 0.0),
    );
    quad(
        &mut v,
        &mut i,
        Vec3::new(-3.6, 7.2, 0.0),
        Vec3::new(0.1, 7.2, 0.0),
        Vec3::new(0.1, 7.5, 0.0),
        Vec3::new(-3.6, 7.5, 0.0),
    );
    let boxes = boxes_of(&v, &i);
    assert!(boxes.len() > 1, "{boxes:?}");
    let blocks_at = |x: f32, y: f32| {
        boxes
            .iter()
            .any(|(lo, hi)| lo.x <= x && hi.x >= x && lo.y <= y && hi.y >= y)
    };
    assert!(blocks_at(0.0, 2.0), "the pole");
    assert!(!blocks_at(-2.5, 2.0), "under the arm");
    assert!(blocks_at(-2.5, 7.3), "the arm itself");
}

#[test]
fn the_passage_under_a_bridge_deck_stays_free() {
    let (v, i) = bridge();
    let boxes = boxes_of(&v, &i);
    assert!(boxes.len() > 1, "{boxes:?}");
    // A bus (4 m high) at x = 0 in the middle: no box reaches into 0..4.5 m there.
    let blocks = boxes
        .iter()
        .any(|(lo, hi)| lo.x <= 0.5 && hi.x >= -0.5 && lo.y < 4.5 && hi.y > 0.0);
    assert!(!blocks, "{boxes:?}");
    // The abutment walls (faces at x = +-5) still block.
    assert!(boxes
        .iter()
        .any(|(lo, hi)| lo.x <= 5.0 && hi.x >= 5.0 && lo.y <= 0.5 && hi.y >= 4.5));
    // The deck is still there at 5 m over the road.
    assert!(boxes
        .iter()
        .any(|(lo, hi)| lo.x <= 0.0 && hi.x >= 0.0 && lo.y > 4.5 && hi.y > 5.0));
}
