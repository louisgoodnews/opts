//! Unit tests for `super` (`weight.rs`).

use super::*;

#[test]
fn a_drive_starts_without_a_test_count() {
    let mut world = World::new();
    world.insert_resource(PassengerCountOverride(Some(40)));
    world
        .run_system_cached(clear_override)
        .expect("system runs");
    assert_eq!(world.resource::<PassengerCountOverride>().0, None);
}
