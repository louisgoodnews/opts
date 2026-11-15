//! Unit tests for `super` (`collision.rs`).
use super::*;

fn bx(x: f32, y: f32, hr: f32, hf: f32, h: f32) -> Obb {
    Obb::new(Vec2::new(x, y), Vec2::new(hr, hf), h, 0.0, 2.0)
}

#[test]
fn separated_boxes_do_not_touch() {
    assert!(bx(0.0, 0.0, 1.0, 2.0, 0.0)
        .contact(&bx(5.0, 0.0, 1.0, 1.0, 0.0))
        .is_none());
}

#[test]
fn overlap_gives_the_shortest_push_away_from_the_other() {
    // `a` 2 m wide and 4 m long heading north; `b` 1 m to its right and overlapping by 1 m.
    let a = bx(0.0, 0.0, 1.0, 2.0, 0.0);
    let b = bx(1.5, 0.0, 1.0, 1.0, 0.0);
    let c = a.contact(&b).expect("overlap");
    assert!((c.depth - 0.5).abs() < 1e-4, "{c:?}");
    assert!(c.normal.x < -0.99, "pushed to the left: {c:?}");
}

#[test]
fn rotated_boxes_use_their_own_axes() {
    // Turned by 90° the long side lies along x and reaches x = 2; the box at 2.25 starts at 1.75.
    let a = bx(0.0, 0.0, 1.0, 2.0, std::f32::consts::FRAC_PI_2);
    let b = bx(2.25, 0.0, 0.5, 0.5, 0.0);
    let c = a.contact(&b).expect("overlap");
    assert!((c.depth - 0.25).abs() < 1e-3, "{c:?}");
    assert!(bx(0.0, 0.0, 1.0, 2.0, std::f32::consts::FRAC_PI_2)
        .contact(&bx(3.0, 0.0, 0.5, 0.5, 0.0))
        .is_none());
}

#[test]
fn response_stops_the_normal_component_and_keeps_part_of_the_tangent() {
    let v = respond(Vec2::new(-3.0, 4.0), Vec2::new(1.0, 0.0), 0.0, 0.25);
    assert!(v.x.abs() < 1e-6);
    assert!((v.y - 3.0).abs() < 1e-6);
    // Moving away is left alone.
    assert_eq!(
        respond(Vec2::new(2.0, 1.0), Vec2::new(1.0, 0.0), 0.0, 0.5),
        Vec2::new(2.0, 1.0)
    );
}

#[test]
fn height_slack_lets_a_vehicle_drive_over_a_kerb() {
    let car = Obb::new(Vec2::ZERO, Vec2::ONE, 0.0, 0.0, 1.5);
    let kerb = Obb::new(Vec2::ZERO, Vec2::ONE, 0.0, 0.0, 0.15);
    assert!(!car.overlaps_height(&kerb, 0.3));
    let wall = Obb::new(Vec2::ZERO, Vec2::ONE, 0.0, 0.0, 1.0);
    assert!(car.overlaps_height(&wall, 0.3));
}

#[test]
fn grid_finds_nearby_boxes_and_drops_a_tile() {
    let mut g = ColliderGrid::new(10.0);
    g.insert(1, bx(5.0, 5.0, 1.0, 1.0, 0.0));
    g.insert(2, bx(95.0, 5.0, 1.0, 1.0, 0.0));
    assert_eq!(g.query(Vec2::new(5.0, 5.0), 3.0).count(), 1);
    g.remove(&1);
    assert_eq!(g.query(Vec2::new(5.0, 5.0), 3.0).count(), 0);
    assert!(!g.is_empty());
}
