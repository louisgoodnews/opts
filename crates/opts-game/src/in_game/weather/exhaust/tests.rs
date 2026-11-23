//! Unit tests for `super` (`exhaust.rs`): the outlet is the model's own when it defines one, the
//! fallback behind and above the vehicle only when it does not.

use super::*;

/// A vehicle at `(10, 0, 0)` facing along the Bevy `-Z` axis (identity rotation).
fn vehicle() -> GlobalTransform {
    GlobalTransform::from(Transform::from_xyz(10.0, 0.0, 0.0))
}

#[test]
fn outlet_comes_from_the_vehicle_data_where_it_is_defined() {
    let data = ExhaustOutlet(Vec3::new(0.3, 1.2, 4.0));
    let outlet = outlet_of(&vehicle(), Some(&data));
    assert_eq!(outlet, Vec3::new(10.3, 1.2, 4.0));
    assert_ne!(outlet, fallback_outlet_of(&vehicle()));
}

#[test]
fn outlet_falls_back_only_where_the_data_is_absent() {
    let outlet = outlet_of(&vehicle(), None);
    // Behind the vehicle (+Z for a vehicle facing -Z), 1 m up.
    assert_eq!(outlet, Vec3::new(10.0, 1.0, REAR_OFFSET_M));
    assert_eq!(outlet, fallback_outlet_of(&vehicle()));
}
