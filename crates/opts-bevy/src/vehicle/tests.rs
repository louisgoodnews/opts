//! Unit tests for `super` (`vehicle.rs`).

use super::*;

fn spot(y: f32, dir_y: f32) -> Spotlight {
    Spotlight {
        pos: Vec3::new(0.0, y, 1.1),
        dir: Vec3::new(0.0, dir_y, -0.3),
        color_rgb: [255; 3],
        range: 100.0,
        inner_cone: 0.0,
        outer_cone: 30.0,
    }
}

#[test]
fn a_forward_emitter_inside_the_body_moves_to_the_headlights() {
    assert_eq!(spot_emitter(&spot(2.5, 1.0), Some(5.9)).y, 5.9);
    // At the front already, pointing backwards, or without headlights: unchanged.
    assert_eq!(spot_emitter(&spot(5.8, 1.0), Some(5.9)).y, 5.8);
    assert_eq!(spot_emitter(&spot(2.5, -1.0), Some(5.9)).y, 2.5);
    assert_eq!(spot_emitter(&spot(2.5, 1.0), None).y, 2.5);
}
