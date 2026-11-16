//! Tests of the camera state.

use super::*;

#[test]
fn speed_never_drops_below_minimum_and_has_no_cap() {
    assert_eq!(scale_free_roam_speed(5.0, -3.0), 5.0);
    assert!((scale_free_roam_speed(5.0, 1.0) - 6.25).abs() < 1e-5);
    assert!(scale_free_roam_speed(5.0, 100.0) > 1.0e9);
}

#[test]
fn sub_camera_cycling_wraps_both_ways() {
    assert_eq!(cycle_index(2, 1, 3), 0);
    assert_eq!(cycle_index(0, -1, 3), 2);
    assert_eq!(cycle_index(1, 7, 3), 2);
    assert_eq!(cycle_index(5, 1, 0), 0);
}

#[test]
fn up_then_down_is_reciprocal() {
    let s = scale_free_roam_speed(scale_free_roam_speed(22.0, 4.0), -4.0);
    assert!((s - 22.0).abs() < 1e-3);
}

#[test]
fn switch_remembers_previous_and_is_idempotent() {
    let mut s = CameraState::default();
    s.switch_to(CameraMode::Cockpit);
    assert_eq!(
        (s.mode, s.previous_mode),
        (CameraMode::Cockpit, CameraMode::FreeRoam)
    );
    s.switch_to(CameraMode::Cockpit);
    assert_eq!(s.previous_mode, CameraMode::FreeRoam);
}
