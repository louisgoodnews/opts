//! Unit tests for `super` (`lib.rs`): the outlet is the model's own when it defines one, the
//! fallback behind and above the vehicle only when it does not.

use super::*;
use opts_bevy::vehicle::ExhaustOutlet;

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

fn at(x: f32, z: f32) -> GlobalTransform {
    GlobalTransform::from(Transform::from_xyz(x, 0.0, z))
}

#[test]
fn a_trailer_with_its_own_emitter_gives_the_trailer_outlet() {
    let trailer = at(10.0, -8.0);
    let emitter = ExhaustOutlet(Vec3::new(0.2, 1.0, 1.5));
    let outlet = chain_outlet(&vehicle(), None, [(&trailer, Some(&emitter))].into_iter());
    assert_eq!(outlet, Vec3::new(10.2, 1.0, -6.5));
    assert_ne!(outlet, fallback_outlet_of(&vehicle()));
}

#[test]
fn a_trailer_without_an_emitter_gives_the_fallback() {
    let trailer = at(10.0, -8.0);
    let outlet = chain_outlet(&vehicle(), None, [(&trailer, None)].into_iter());
    assert_eq!(outlet, fallback_outlet_of(&vehicle()));
}

#[test]
fn the_rear_most_emitter_wins_and_the_head_emitter_is_the_last_choice() {
    let front = at(10.0, -4.0);
    let rear = at(10.0, -12.0);
    let head = ExhaustOutlet(Vec3::new(0.0, 1.0, 2.0));
    let e_front = ExhaustOutlet(Vec3::ZERO);
    let e_rear = ExhaustOutlet(Vec3::Y * 2.0);
    let units = [(&front, Some(&e_front)), (&rear, Some(&e_rear))];
    let outlet = chain_outlet(&vehicle(), Some(&head), units.into_iter());
    assert_eq!(outlet, Vec3::new(10.0, 2.0, -12.0));
    // No trailer defines one: the head's own outlet, not the fallback.
    let none = chain_outlet(&vehicle(), Some(&head), [(&front, None)].into_iter());
    assert_eq!(none, outlet_of(&vehicle(), Some(&head)));
}

#[test]
fn outlet_falls_back_only_where_the_data_is_absent() {
    let outlet = outlet_of(&vehicle(), None);
    // Behind the vehicle (+Z for a vehicle facing -Z), 1 m up.
    assert_eq!(outlet, Vec3::new(10.0, 1.0, REAR_OFFSET_M));
    assert_eq!(outlet, fallback_outlet_of(&vehicle()));
}
