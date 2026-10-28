//! Unit tests for `super` (`viewpoint.rs`).

use super::*;

fn state() -> ViewpointState {
    ViewpointState {
        default_fov_deg: 70.0,
        ..default()
    }
}

#[test]
fn default_is_the_vehicle_view() {
    let vp = Viewpoint::default();
    assert!(vp.is_default() && vp.is_valid());
    assert_eq!(vp.local_offset(), Vec3::ZERO);
}

#[test]
fn limits_clamp_and_non_finite_values_vanish() {
    let vp = Viewpoint {
        x: 5.0,
        y: -5.0,
        z: f32::NAN,
        fov_deg: Some(500.0),
    }
    .clamped();
    assert_eq!((vp.x, vp.y, vp.z), (0.6, -0.6, 0.0));
    assert_eq!(vp.fov_deg, Some(120.0));
    assert!(vp.is_valid());
    assert!(!Viewpoint {
        fov_deg: Some(10.0),
        ..default()
    }
    .is_valid());
    assert_eq!(
        Viewpoint {
            fov_deg: Some(10.0),
            ..default()
        }
        .clamped()
        .fov_deg,
        Some(30.0)
    );
}

#[test]
fn forward_offset_points_down_the_bevy_forward_axis() {
    let vp = Viewpoint {
        x: 0.1,
        y: 0.2,
        z: 0.3,
        fov_deg: None,
    };
    assert_eq!(vp.local_offset(), Vec3::new(0.1, 0.2, -0.3));
}

#[test]
fn adjusting_stays_inside_the_limits() {
    let mut s = state();
    s.adjust(ViewpointRow::Y, 1000.0);
    assert_eq!(s.active.y, 0.6);
    s.adjust(ViewpointRow::Fov, 5.0);
    assert_eq!(s.active.fov_deg, Some(75.0));
    s.adjust(ViewpointRow::Fov, -1000.0);
    assert_eq!(s.active.fov_deg, Some(30.0));
}

#[test]
fn save_requests_a_write_once_and_cancel_restores() {
    let mut s = state();
    s.open();
    s.adjust(ViewpointRow::X, 3.0);
    assert!(s.is_dirty());
    s.cancel();
    assert!(!s.editing && s.active == Viewpoint::default());

    s.open();
    s.adjust(ViewpointRow::X, 3.0);
    s.save();
    assert!(!s.editing);
    assert_eq!(
        s.request.take(),
        Some(ViewpointRequest::Save(Viewpoint {
            x: 0.03,
            ..default()
        }))
    );
    s.open();
    s.save();
    assert_eq!(s.request, None, "an unchanged viewpoint writes nothing");
}

#[test]
fn reset_then_save_stores_the_default() {
    let mut s = state();
    s.committed = Viewpoint {
        z: 0.2,
        ..default()
    };
    s.active = s.committed;
    s.open();
    s.reset();
    s.save();
    assert_eq!(
        s.request,
        Some(ViewpointRequest::Save(Viewpoint::default()))
    );
}

#[test]
fn fraction_round_trips() {
    let mut s = state();
    s.set_fraction(ViewpointRow::Fov, 0.5);
    assert_eq!(s.active.fov_deg, Some(75.0));
    assert!((s.fraction(ViewpointRow::Fov) - 0.5).abs() < 1e-3);
    s.set_fraction(ViewpointRow::X, 1.0);
    assert_eq!(s.active.x, 0.6);
    assert!((s.fraction(ViewpointRow::X) - 1.0).abs() < 1e-3);
}

#[test]
fn rows_wrap() {
    assert_eq!(ViewpointRow::Fov.next(), ViewpointRow::X);
    assert_eq!(ViewpointRow::X.prev(), ViewpointRow::Fov);
}
