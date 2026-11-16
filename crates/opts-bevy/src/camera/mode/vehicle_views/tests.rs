//! Vehicle view tests: the horizontal to vertical field of view.

use super::*;

#[test]
fn horizontal_to_vertical_fov() {
    let v = fov_y_from_horizontal(90.0, 16.0 / 9.0);
    assert!((v.to_degrees() - 58.7).abs() < 0.2, "{}", v.to_degrees());
    assert!((fov_y_from_horizontal(60.0, 1.0).to_degrees() - 60.0).abs() < 1e-3);
}
