//! Tests of the segment placement (`segment_placement`).

use super::*;
use crate::coords::omsi_to_bevy_pos;

/// The coupling point of a placement in Bevy space.
fn coupling(translation: Vec3, rotation: Quat, front: Vec3) -> Vec3 {
    translation + rotation * omsi_to_bevy_pos(front)
}

#[test]
fn tilt_pivots_about_the_front_coupling() {
    let origin = Vec3::new(1.0, 2.0, 0.3);
    let front = Vec3::new(0.2, 4.1, 0.4);
    let level = coupling(
        segment_placement(origin, front, 0.5, 0.0, 0.0).0,
        Quat::from_rotation_y(0.5),
        front,
    );
    for (pitch, roll) in [(0.0, 0.0), (0.05, 0.0), (0.0, -0.03), (0.05, -0.03)] {
        let (t, r) = segment_placement(origin, front, 0.5, pitch, roll);
        let c = coupling(t, r, front);
        assert!(
            (c - level).length() < 1e-5,
            "pitch {pitch}, roll {roll}: coupling {c:?} left {level:?}"
        );
    }
}

#[test]
fn level_placement_is_the_origin() {
    let origin = Vec3::new(-3.0, 7.0, 0.5);
    let (t, _) = segment_placement(origin, Vec3::new(0.0, 4.0, 0.4), 1.2, 0.0, 0.0);
    assert!((t - omsi_to_bevy_pos(origin)).length() < 1e-6);
}
