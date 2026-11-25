//! Tests: formation_tests.

use super::trailers::{coupling_joint, max_parts};
use super::*;

/// A manifest with the two coupling points at `front_y` and `back_y` (x 0, z 1).
fn car(front_y: f32, back_y: f32) -> BusManifest {
    BusManifest::parse(&format!(
        "[coupling_front]\n0\n{front_y}\n1\n\n[coupling_back]\n0\n{back_y}\n1\n\n"
    ))
    .expect("manifest")
}

/// Distance of the follower's origin behind the leader's origin: the loader's `f.y - b.y`.
fn gap(parent: &BusManifest, pt: bool, own: &BusManifest, ot: bool) -> f32 {
    let (b, f) = coupling_joint(parent, pt, own, ot).expect("points");
    f.y - b.y
}

#[test]
fn two_unturned_cars_join_back_to_front() {
    // Leader's rear coupling 10 m behind its origin, follower's front coupling 12 m ahead.
    let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
    assert!((gap(&a, false, &b, false) - 22.0).abs() < 1e-4);
}

#[test]
fn a_turned_follower_presents_its_back_point_as_its_front() {
    let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
    // Turned: its model's back point (-9) becomes the front point (+9).
    assert!((gap(&a, false, &b, true) - 19.0).abs() < 1e-4);
    let (b_pt, f_pt) = coupling_joint(&a, false, &b, true).expect("points");
    assert_eq!((b_pt.y, f_pt.y), (-10.0, 9.0));
}

#[test]
fn a_turned_leader_presents_its_front_point_as_its_rear() {
    let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
    // Turned leader: its model's front point (+11) becomes the rear point (-11).
    assert!((gap(&a, true, &b, false) - 23.0).abs() < 1e-4);
    // Both turned: rear -11, follower front +9.
    assert!((gap(&a, true, &b, true) - 20.0).abs() < 1e-4);
}

#[test]
fn a_symmetric_pair_is_the_same_whichever_way_it_is_turned() {
    let (a, b) = (car(13.2, -13.2), car(13.2, -13.2));
    for (pt, ot) in [(false, false), (false, true), (true, false), (true, true)] {
        assert!((gap(&a, pt, &b, ot) - 26.4).abs() < 1e-4, "{pt} {ot}");
    }
}

#[test]
fn a_missing_point_gives_no_joint() {
    let a = car(11.0, -10.0);
    let bare = BusManifest::parse("[mass]\n10\n").expect("manifest");
    assert!(coupling_joint(&a, false, &bare, false).is_none());
    assert!(coupling_joint(&bare, false, &a, false).is_none());
}

#[test]
fn the_ai_loader_and_the_formation_loader_agree_on_a_couple_back_link() {
    // The old inline formula of `load_spec`/`spawn_trailer_chain` for a link flagged reversed.
    let (a, b) = (car(11.0, -10.0), car(12.0, -9.0));
    let old = |reversed: bool| {
        let own = if reversed {
            b.coupling_back.map(|c| -c)
        } else {
            b.coupling_front
        };
        own.expect("own").y - a.coupling_back.expect("back").y
    };
    for reversed in [false, true] {
        assert!((gap(&a, false, &b, reversed) - old(reversed)).abs() < 1e-6);
    }
}

#[test]
fn the_part_limit_is_forty_for_a_single_vehicle_and_the_formation_limit_otherwise() {
    assert_eq!(max_parts(false), 40);
    assert_eq!(max_parts(true), opts_core::formats::trains::MAX_PARTS);
}
