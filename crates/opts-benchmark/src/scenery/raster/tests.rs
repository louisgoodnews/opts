//! Unit tests for raster.

use super::*;

#[test]
fn square_of_two_triangles_covers_its_area() {
    let mut m = Mask::new(16, 16);
    let (a, b, c, d) = (
        Vec2::new(2.0, 2.0),
        Vec2::new(10.0, 2.0),
        Vec2::new(10.0, 10.0),
        Vec2::new(2.0, 10.0),
    );
    fill_triangle(&mut m, [a, b, c]);
    fill_triangle(&mut m, [a, d, c]); // opposite winding
    assert_eq!(m.count(), 64);
    assert_eq!(m.bbox(), Some([2, 2, 9, 9]));
    assert_eq!(m.centroid(), Some(Vec2::new(6.0, 6.0)));
}

#[test]
fn dilation_grows_by_k() {
    let mut m = Mask::new(9, 9);
    m.px[4 * 9 + 4] = true;
    assert_eq!(m.dilate(2).count(), 25);
}

#[test]
fn degenerate_and_offscreen_triangles_cover_nothing() {
    let mut m = Mask::new(8, 8);
    fill_triangle(
        &mut m,
        [
            Vec2::new(1.0, 1.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(3.0, 3.0),
        ],
    );
    fill_triangle(
        &mut m,
        [
            Vec2::new(-10.0, -10.0),
            Vec2::new(-5.0, -10.0),
            Vec2::new(-5.0, -5.0),
        ],
    );
    assert_eq!(m.count(), 0);
}
