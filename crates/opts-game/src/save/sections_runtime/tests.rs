//! Unit tests for `super` (`sections_runtime.rs`).

use super::*;

fn world_with(count: Option<usize>) -> World {
    let mut world = World::new();
    world.insert_resource(PassengerCountOverride(count));
    world
}

#[test]
fn passenger_load_is_left_out_without_an_override() {
    let mut world = world_with(None);
    assert_eq!(passenger_load_capture(&mut world), Ok(None));
    let mut bare = World::new();
    assert_eq!(passenger_load_capture(&mut bare), Ok(None));
}

#[test]
fn passenger_load_round_trips_the_override() {
    let mut world = world_with(Some(37));
    let data = passenger_load_capture(&mut world)
        .expect("capture")
        .expect("a section");
    assert_eq!(data, json!({ "count_override": 37 }));
    let mut other = world_with(None);
    passenger_load_apply(&mut other, &data).expect("apply");
    assert_eq!(other.resource::<PassengerCountOverride>().0, Some(37));
}

#[test]
fn a_saved_zero_is_an_override_not_the_real_passengers() {
    let mut world = world_with(Some(0));
    let data = passenger_load_capture(&mut world)
        .expect("capture")
        .expect("a section");
    let mut other = world_with(None);
    passenger_load_apply(&mut other, &data).expect("apply");
    assert_eq!(other.resource::<PassengerCountOverride>().0, Some(0));
}

#[test]
fn a_saved_count_above_the_protocol_limit_is_cut_to_it() {
    let mut world = world_with(None);
    passenger_load_apply(&mut world, &json!({ "count_override": 100_000 })).expect("apply");
    assert_eq!(world.resource::<PassengerCountOverride>().0, Some(400));
}

#[test]
fn a_damaged_passenger_load_section_is_an_error() {
    let mut world = world_with(None);
    assert!(passenger_load_apply(&mut world, &json!({ "count_override": "many" })).is_err());
    assert_eq!(world.resource::<PassengerCountOverride>().0, None);
}
