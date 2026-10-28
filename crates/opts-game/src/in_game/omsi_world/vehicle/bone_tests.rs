//! Tests: bone_tests.

use super::skins::is_bone_stem;

#[test]
fn bone_dummies_are_bone_underscore_letter() {
    assert!(is_bone_stem("bone_a"));
    assert!(is_bone_stem("bone_d"));
    assert!(is_bone_stem("dummya"));
    assert!(is_bone_stem("dummy_c"));
    assert!(!is_bone_stem("bone_ab"));
    assert!(!is_bone_stem("bone_1"));
    assert!(!is_bone_stem("gn_dum_gelenk_a"));
}

use super::skins::{joints_in_frame, pick_run};
use bevy::prelude::Entity;

fn bone(name: &str, index: i32) -> opts_core::formats::vehicles::model::MeshBone {
    opts_core::formats::vehicles::model::MeshBone {
        name: name.to_owned(),
        index,
    }
}

#[test]
fn a_skin_in_the_vehicle_frame_uses_the_dummies_themselves() {
    let (root, a, b) = (
        Entity::from_raw_u32(1).unwrap(),
        Entity::from_raw_u32(2).unwrap(),
        Entity::from_raw_u32(3).unwrap(),
    );
    let joints = joints_in_frame(&[a, b, root], root, root, |_| panic!("no proxy"));
    assert_eq!(joints, vec![a, b, root]);
}

#[test]
fn a_skin_below_an_animparent_moves_its_bones_in_that_frame() {
    // The Valiant 7700A's rear bellows half (`[animparent] tourelle`): bones 0 and 1 are the
    // dummies `bone_A`/`bone_B` turned by the half-angle frame, the unlisted base bone is the frame.
    let e = |n: u32| Entity::from_raw_u32(n).unwrap();
    let (root, a, b, frame) = (e(1), e(2), e(3), e(4));
    let mut next = 10;
    let mut made = Vec::new();
    let joints = joints_in_frame(&[a, b, a, root], root, frame, |d| {
        next += 1;
        made.push(d);
        e(next)
    });
    assert_eq!(joints, vec![e(11), e(12), e(11), frame]);
    assert_eq!(made, vec![a, b], "one proxy per dummy");
}

#[test]
fn a_skin_takes_the_last_run_long_enough_for_its_bones() {
    let e = |n: u32| Entity::from_raw_u32(n).unwrap();
    let runs = vec![
        ("bone_".to_owned(), vec![e(1), e(2), e(3), e(4)]),
        ("damper_".to_owned(), vec![e(5)]),
    ];
    assert_eq!(
        pick_run(&runs, Some(1), &[bone("b1", 0), bone("b2", 1)]),
        vec![e(1), e(2), e(3), e(4)]
    );
    assert_eq!(pick_run(&runs, Some(1), &[bone("b1", 0)]), vec![e(5)]);
    assert!(pick_run(&[], None, &[bone("b1", 0)]).is_empty());
}
