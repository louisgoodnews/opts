//! Unit test for the plate surface: height and map lookup at a point on a triangle.
use super::*;

#[test]
fn a_point_on_the_triangle_reads_its_height_and_its_map() {
    let tri = SurfTri {
        xy: [Vec2::ZERO, Vec2::new(4.0, 0.0), Vec2::new(0.0, 4.0)],
        z: [1.0, 1.0, 1.0],
        uv: [Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
        map: Arc::new(SurfMap::from_heights(1, 1, vec![0.01])),
        period: None,
    };
    let (z, u) = tri.at(Vec2::new(1.0, 1.0)).expect("inside");
    assert!((z - 1.0).abs() < 1e-6 && (u - 0.01).abs() < 1e-6);
    assert!(
        tri.at(Vec2::new(3.0, 3.0)).is_none(),
        "outside the hypotenuse"
    );
    assert_eq!(tri.bounds(), (Vec2::ZERO, Vec2::new(4.0, 4.0)));
}
