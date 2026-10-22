//! Naming the mirrors of a vehicle for the virtual mirrors.

use super::*;

fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    (0..n).map(|_| world.spawn_empty().id()).collect()
}

/// The six panes of the MAN Lion's City as the vehicle frame has them (Bevy axes, from the
/// `[add_camera_reflexion_2]` tags): left main, right main, right wide-angle, a centre camera,
/// a camera along the right side and the interior mirror.
fn lions_city(e: &[Entity]) -> Vec<(Entity, Vec3)> {
    vec![
        (e[0], Vec3::new(-1.3, 2.17, -5.47)),
        (e[1], Vec3::new(1.38, 2.128, -6.25)),
        (e[2], Vec3::new(1.2, 2.49, -6.18)),
        (e[3], Vec3::new(0.005, 2.46, -5.48)),
        (e[4], Vec3::new(1.4, 3.0, -2.65)),
        (e[5], Vec3::new(-0.28, 2.7, -0.1)),
    ]
}

#[test]
fn the_side_mirrors_are_named_and_the_interior_mirror_is_not() {
    let e = entities(6);
    let slots = assign_slots(&lions_city(&e));
    assert_eq!(slots, [Some(e[0]), Some(e[2]), Some(e[1])]);
}

#[test]
fn a_vehicle_without_a_second_right_mirror_leaves_that_slot_empty() {
    let e = entities(2);
    let slots = assign_slots(&[
        (e[0], Vec3::new(-1.1, 2.0, -3.0)),
        (e[1], Vec3::new(1.1, 2.0, -3.0)),
    ]);
    assert_eq!(slots, [Some(e[0]), Some(e[1]), None]);
    assert_eq!(assign_slots(&[]), [None, None, None]);
}

#[test]
fn the_stream_picture_is_twice_the_size_up_to_1024() {
    assert_eq!(stream_size(UVec2::new(256, 128)), UVec2::new(512, 256));
    assert_eq!(stream_size(UVec2::new(700, 600)), UVec2::new(1024, 1024));
}

#[test]
fn toggling_a_stream_flips_only_that_slot() {
    let mut vm = VirtualMirrors::default();
    assert!(!vm.any_stream());
    vm.toggle(MirrorSlot::RightLower);
    assert_eq!(vm.streams, [false, false, true]);
    assert!(vm.any_stream());
    vm.toggle(MirrorSlot::RightLower);
    assert!(!vm.any_stream());
}
